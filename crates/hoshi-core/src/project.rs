//! Versioned, bounded, checksummed layered data. No filesystem access.
use super::*;
const MAGIC: &[u8; 8] = b"HOSHIPNT";
const VERSION: u32 = 1;
const NAME_LIMIT: usize = 1024;
pub const MAX_PROJECT_BYTES: usize = 128 * 1024 * 1024 + 64 * 1024;
#[derive(Clone, Debug)]
pub struct DocumentSnapshot(State);
impl DocumentSnapshot {
    /// Cheap cache check. Equality still compares content when storage differs.
    pub fn same_storage_as(&self, document: &Document) -> bool {
        let s = &document.state;
        (self.0.width, self.0.height, self.0.resolution) == (s.width, s.height, s.resolution)
            && self.0.layers.len() == s.layers.len()
            && self.0.layers.iter().zip(&s.layers).all(|(a, b)| {
                a.name == b.name
                    && a.visible == b.visible
                    && a.opacity == b.opacity
                    && Arc::ptr_eq(&a.pixels, &b.pixels)
            })
    }
}
impl PartialEq for DocumentSnapshot {
    fn eq(&self, other: &Self) -> bool {
        (self.0.width, self.0.height, self.0.resolution)
            == (other.0.width, other.0.height, other.0.resolution)
            && self.0.layers == other.0.layers
    }
}
impl Eq for DocumentSnapshot {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectError {
    InvalidData,
    UnsupportedVersion(u32),
    InvalidDocument,
    ActiveShape,
    Checksum,
}
impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData => f.write_str("The file is truncated or has invalid project data."),
            Self::UnsupportedVersion(v) => write!(f, "Project version {v} is not supported by this build."),
            Self::InvalidDocument => f.write_str("The document exceeds supported dimensions, layers, resolution, or layer-name limits."),
            Self::ActiveShape => f.write_str("Commit the active shape before saving."),
            Self::Checksum => f.write_str("The project checksum does not match; the file may be damaged."),
        }
    }
}
impl std::error::Error for ProjectError {}
impl Document {
    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot(self.state.clone())
    }
    pub fn from_rgba(
        width: u32,
        height: u32,
        resolution: f64,
        pixels: Vec<Color>,
    ) -> Result<Self, Error> {
        Self::validate_image_size(width, height)?;
        if pixels.len() != (width * height) as usize {
            return Err(Error::InvalidDimensions);
        }
        let mut d = Self::new_with_background(width, height, resolution, Color::TRANSPARENT)?;
        d.state.layers[0].pixels = Arc::new(pixels);
        Ok(d)
    }
    pub fn has_transparency(&self) -> bool {
        self.composite(true).iter().any(|c| c.a < 255)
    }
    /// Explicit, undoable preparation for a flat file format, after user consent.
    pub fn flatten_for_save(&mut self, matte: Option<Color>) -> Result<(), Error> {
        self.image_editable_for_project()?;
        let before = self.state.clone();
        let mut pixels = self.composite(false);
        if let Some(background) = matte {
            for c in &mut pixels {
                *c = c.over(background, 255)
            }
        }
        if self.state.layers.len() == 1
            && self.state.layers[0].visible
            && self.state.layers[0].opacity == 255
            && *self.state.layers[0].pixels == pixels
        {
            return Ok(());
        }
        self.state.layers = vec![Layer {
            name: "Background".into(),
            visible: true,
            opacity: 255,
            pixels: Arc::new(pixels),
        }];
        self.state.selected = 0;
        self.checkpoint(before, "Flatten for Save");
        Ok(())
    }
    fn image_editable_for_project(&self) -> Result<(), Error> {
        if self.active.is_some() {
            Err(Error::ActiveShape)
        } else {
            Ok(())
        }
    }
    pub fn encode_project(&self) -> Result<Vec<u8>, ProjectError> {
        if self.active.is_some() {
            return Err(ProjectError::ActiveShape);
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        for value in [
            VERSION,
            self.width(),
            self.height(),
            self.state.resolution,
            self.state.selected as u32,
            self.state.layers.len() as u32,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes())
        }
        for layer in &self.state.layers {
            let name = layer.name.as_bytes();
            if layer.name.trim().is_empty() || name.len() > NAME_LIMIT {
                return Err(ProjectError::InvalidDocument);
            }
            bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
            bytes.extend_from_slice(name);
            bytes.extend_from_slice(&[u8::from(layer.visible), layer.opacity]);
            for c in layer.pixels.iter() {
                bytes.extend_from_slice(&[c.r, c.g, c.b, c.a]);
            }
        }
        let checksum = crc32fast::hash(&bytes);
        bytes.extend_from_slice(&checksum.to_le_bytes());
        Ok(bytes)
    }
    pub fn decode_project(bytes: &[u8]) -> Result<Self, ProjectError> {
        if bytes.len() < 36 || bytes.len() > MAX_PROJECT_BYTES || &bytes[..8] != MAGIC {
            return Err(ProjectError::InvalidData);
        }
        let end = bytes.len() - 4;
        let checksum = u32::from_le_bytes(
            bytes[end..]
                .try_into()
                .map_err(|_| ProjectError::InvalidData)?,
        );
        if crc32fast::hash(&bytes[..end]) != checksum {
            return Err(ProjectError::Checksum);
        }
        let mut r = Reader {
            data: &bytes[8..end],
            position: 0,
        };
        let version = r.number()?;
        if version != VERSION {
            return Err(ProjectError::UnsupportedVersion(version));
        }
        let (width, height, resolution, selected, count) = (
            r.number()?,
            r.number()?,
            r.number()?,
            r.number()? as usize,
            r.number()? as usize,
        );
        Self::validate_image_size(width, height).map_err(|_| ProjectError::InvalidDocument)?;
        Self::validate_resolution(f64::from(resolution) / 1000.)
            .map_err(|_| ProjectError::InvalidDocument)?;
        if count == 0 || count > MAX_LAYERS || selected >= count {
            return Err(ProjectError::InvalidDocument);
        }
        let pixel_bytes = width as usize * height as usize * 4;
        // Validate the full pixel payload before allocating any layer buffers.
        if count
            .checked_mul(pixel_bytes)
            .is_none_or(|n| n > r.data.len())
        {
            return Err(ProjectError::InvalidData);
        }
        let mut layers = Vec::with_capacity(count);
        for _ in 0..count {
            let length = r.number()? as usize;
            if length == 0 || length > NAME_LIMIT {
                return Err(ProjectError::InvalidData);
            }
            let name = std::str::from_utf8(r.take(length)?)
                .map_err(|_| ProjectError::InvalidData)?
                .to_owned();
            if name.trim().is_empty() {
                return Err(ProjectError::InvalidData);
            }
            let props = r.take(2)?;
            if props[0] > 1 {
                return Err(ProjectError::InvalidData);
            }
            let (visible, opacity) = (props[0] == 1, props[1]);
            let pixels = r
                .take(pixel_bytes)?
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| Color::rgba(c[0], c[1], c[2], c[3]))
                .collect();
            layers.push(Layer {
                name,
                visible,
                opacity,
                pixels: Arc::new(pixels),
            });
        }
        if r.position != r.data.len() {
            return Err(ProjectError::InvalidData);
        }
        Ok(Self {
            state: State {
                width,
                height,
                resolution,
                selected,
                layers,
            },
            active: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
        })
    }
}
struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProjectError> {
        let end = self
            .position
            .checked_add(n)
            .ok_or(ProjectError::InvalidData)?;
        let slice = self
            .data
            .get(self.position..end)
            .ok_or(ProjectError::InvalidData)?;
        self.position = end;
        Ok(slice)
    }
    fn number(&mut self) -> Result<u32, ProjectError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ProjectError::InvalidData)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn checksum(bytes: &mut [u8]) {
        let end = bytes.len() - 4;
        let crc = crc32fast::hash(&bytes[..end]);
        bytes[end..].copy_from_slice(&crc.to_le_bytes());
    }
    #[test]
    fn layered_round_trip_preserves_data_and_excludes_history() {
        let mut d = Document::from_rgba(
            2,
            1,
            300.125,
            vec![Color::rgba(1, 2, 3, 0), Color::rgba(4, 5, 6, 127)],
        )
        .unwrap();
        d.rename_layer(0, "星 background").unwrap();
        d.add_layer("Overlay").unwrap();
        d.set_layer_properties(1, false, 83).unwrap();
        d.select_layer(1).unwrap();
        let decoded = Document::decode_project(&d.encode_project().unwrap()).unwrap();
        assert_eq!(decoded.snapshot(), d.snapshot());
        assert_eq!(decoded.selected_layer(), 1);
        assert_eq!(decoded.undo_len(), 0);
        assert_eq!(decoded.layers()[0].pixels()[0], Color::rgba(1, 2, 3, 0));
        let saved = d.snapshot();
        d.select_layer(0).unwrap();
        assert_eq!(saved, d.snapshot());
        assert!(saved.same_storage_as(&d));
        d.flatten_for_save(Some(Color::rgba(255, 255, 255, 255)))
            .unwrap();
        assert_eq!(d.layers().len(), 1);
        assert!(!d.has_transparency());
        d.undo().unwrap();
        assert_eq!(d.snapshot(), saved);
    }
    #[test]
    fn corrupt_or_hostile_projects_are_rejected_before_allocation() {
        let original = Document::new(2, 2).unwrap().encode_project().unwrap();
        let mut bytes = original.clone();
        bytes[38] ^= 1;
        assert_eq!(
            Document::decode_project(&bytes).err().unwrap(),
            ProjectError::Checksum
        );
        for length in [0, 8, 32, original.len() - 1] {
            assert!(Document::decode_project(&original[..length]).is_err());
        }
        let mut bytes = original.clone();
        bytes[8..12].copy_from_slice(&99u32.to_le_bytes());
        checksum(&mut bytes);
        assert_eq!(
            Document::decode_project(&bytes).err().unwrap(),
            ProjectError::UnsupportedVersion(99)
        );
        for (offset, value) in [(12, u32::MAX), (24, 32), (28, 33), (32, u32::MAX)] {
            let mut bytes = original.clone();
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            checksum(&mut bytes);
            assert!(Document::decode_project(&bytes).is_err(), "offset {offset}");
        }
        let mut bytes = original.clone();
        let end = bytes.len() - 4;
        bytes.insert(end, 0);
        checksum(&mut bytes);
        assert!(Document::decode_project(&bytes).is_err());
    }
}
