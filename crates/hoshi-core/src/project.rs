//! Versioned, checksummed layered data with bounded streaming I/O.
use super::*;
use std::io::{Read, Write};
const MAGIC: &[u8; 8] = b"HOSHIPNT";
const VERSION: u32 = 2;
const NAME_LIMIT: usize = 1024;
#[derive(Clone, Debug)]
pub struct DocumentSnapshot(State);
impl DocumentSnapshot {
    pub fn sampler(&self, preview: Option<Shape>) -> CompositeSampler<'_> {
        CompositeSampler::new(&self.0, preview)
    }
    pub fn composite_pixel(&self, x: u32, y: u32, preview: Option<Shape>) -> Color {
        self.sampler(preview).pixel(x, y)
    }
    /// Cheap cache check. Equality compares content when storage differs.
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
    MemoryLimit,
    Io(String),
}
/// Developer-facing description for logs. Adapters map each variant to text
/// in their locale catalogs.
impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData => f.write_str("truncated or invalid project data"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported project version {v}"),
            Self::InvalidDocument => f.write_str("document exceeds project limits"),
            Self::ActiveShape => f.write_str("active shape"),
            Self::Checksum => f.write_str("project checksum mismatch"),
            Self::MemoryLimit => f.write_str("memory budget exceeded"),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}
impl std::error::Error for ProjectError {}
impl From<std::io::Error> for ProjectError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            Self::InvalidData
        } else {
            Self::Io(e.to_string())
        }
    }
}
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
        let pixels = PixelStore::from_vec(width, height, pixels)?;
        Self::from_pixels(width, height, resolution, pixels)
    }
    pub fn from_pixels(
        width: u32,
        height: u32,
        resolution: f64,
        pixels: PixelStore,
    ) -> Result<Self, Error> {
        Self::validate_image_size(width, height)?;
        Self::validate_resolution(resolution)?;
        if pixels.width() != width || pixels.height() != height {
            return Err(Error::InvalidDimensions);
        }
        let mut d = Self::new_with_background(width, height, resolution, Color::TRANSPARENT)?;
        d.state.layers[0].pixels = Arc::new(pixels);
        Ok(d)
    }
    pub fn has_transparency(&self) -> bool {
        self.has_transparency_fast()
    }
    /// Explicit, undoable preparation for a flat format, after user consent.
    pub fn flatten_for_save(&mut self, matte: Option<Color>) -> Result<(), Error> {
        self.reject_active_stroke()?;
        if self.active.is_some() {
            return Err(Error::ActiveShape);
        }
        let pixels = self.flattened_pixels(matte)?;
        if self.state.layers.len() == 1
            && self.state.layers[0].visible
            && self.state.layers[0].opacity == 255
            && *self.state.layers[0].pixels == pixels
        {
            return Ok(());
        }
        let before = self.state.clone();
        self.state.layers = vec![Layer {
            name: "Background".into(),
            visible: true,
            opacity: 255,
            pixels: Arc::new(pixels),
        }];
        self.state.selected = 0;
        self.checkpoint(before, HistoryLabel::FlattenForSave)?;
        Ok(())
    }
    /// Stream V2 tile records without allocating an encoded copy of the document.
    pub fn write_project(&self, output: impl Write) -> Result<(), ProjectError> {
        if self.active.is_some() || self.has_active_stroke() {
            return Err(ProjectError::ActiveShape);
        }
        let mut w = CheckedWriter {
            inner: output,
            crc: crc32fast::Hasher::new(),
        };
        w.bytes(MAGIC)?;
        for value in [
            VERSION,
            self.width(),
            self.height(),
            self.state.resolution,
            self.state.selected as u32,
            self.state.layers.len() as u32,
        ] {
            w.number(value)?;
        }
        for layer in &self.state.layers {
            let name = layer.name.as_bytes();
            if layer.name.trim().is_empty() || name.len() > NAME_LIMIT {
                return Err(ProjectError::InvalidDocument);
            }
            w.number(name.len() as u32)?;
            w.bytes(name)?;
            w.bytes(&[u8::from(layer.visible), layer.opacity])?;
            let c = layer.pixels.background();
            w.bytes(&[c.r, c.g, c.b, c.a])?;
            let count = layer.pixels.tile_positions().count();
            w.number(u32::try_from(count).map_err(|_| ProjectError::InvalidDocument)?)?;
            for (tx, ty) in layer.pixels.tile_positions() {
                w.number(tx)?;
                w.number(ty)?;
                let (x0, y0) = (tx * TILE_SIZE, ty * TILE_SIZE);
                for y in y0..(y0 + TILE_SIZE).min(self.height()) {
                    for x in x0..(x0 + TILE_SIZE).min(self.width()) {
                        let c = layer.pixels.get(x, y);
                        w.bytes(&[c.r, c.g, c.b, c.a])?;
                    }
                }
            }
        }
        let crc = w.crc.finalize();
        w.inner.write_all(&crc.to_le_bytes())?;
        Ok(())
    }
    /// Convenience for small in-memory callers. Desktop saving uses streaming I/O.
    pub fn encode_project(&self) -> Result<Vec<u8>, ProjectError> {
        let estimate = self
            .state
            .layers
            .iter()
            .map(|l| {
                16u64
                    + l.name.len() as u64
                    + l.pixels.tile_positions().count() as u64
                        * (8 + u64::from(TILE_SIZE) * u64::from(TILE_SIZE) * 4)
            })
            .sum::<u64>()
            + 36;
        let _memory = MemoryReservation::new(estimate).map_err(|_| ProjectError::MemoryLimit)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(usize::try_from(estimate).map_err(|_| ProjectError::MemoryLimit)?)
            .map_err(|_| ProjectError::MemoryLimit)?;
        self.write_project(&mut output)?;
        Ok(output)
    }
    pub fn decode_project(bytes: &[u8]) -> Result<Self, ProjectError> {
        Self::read_project(std::io::Cursor::new(bytes))
    }
    /// Reads old dense V1 and sparse V2 files. Invalid input never replaces an open document.
    pub fn read_project(input: impl Read) -> Result<Self, ProjectError> {
        let mut r = CheckedReader {
            inner: input,
            crc: crc32fast::Hasher::new(),
        };
        if &r.array::<8>()? != MAGIC {
            return Err(ProjectError::InvalidData);
        }
        let version = r.number()?;
        if !matches!(version, 1 | 2) {
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
        let mut layers = Vec::with_capacity(count);
        for _ in 0..count {
            let length = r.number()? as usize;
            if length == 0 || length > NAME_LIMIT {
                return Err(ProjectError::InvalidData);
            }
            let mut name = vec![0; length];
            r.bytes(&mut name)?;
            let name = String::from_utf8(name).map_err(|_| ProjectError::InvalidData)?;
            if name.trim().is_empty() {
                return Err(ProjectError::InvalidData);
            }
            let props = r.array::<2>()?;
            if props[0] > 1 {
                return Err(ProjectError::InvalidData);
            }
            let mut pixels;
            if version == 1 {
                let first = r.color()?;
                pixels = PixelStore::new(width, height, first);
                for y in 0..height {
                    for x in 0..width {
                        if x == 0 && y == 0 {
                            continue;
                        }
                        pixels
                            .set(x, y, r.color()?)
                            .map_err(|_| ProjectError::MemoryLimit)?;
                    }
                }
            } else {
                pixels = PixelStore::new(width, height, r.color()?);
                let tile_count = r.number()?;
                let (columns, rows) = (width.div_ceil(TILE_SIZE), height.div_ceil(TILE_SIZE));
                if u64::from(tile_count) > u64::from(columns) * u64::from(rows) {
                    return Err(ProjectError::InvalidData);
                }
                let _coordinates = MemoryReservation::new(u64::from(tile_count) * 32)
                    .map_err(|_| ProjectError::MemoryLimit)?;
                let mut seen = std::collections::HashSet::new();
                for _ in 0..tile_count {
                    let (tx, ty) = (r.number()?, r.number()?);
                    if tx >= columns || ty >= rows {
                        return Err(ProjectError::InvalidData);
                    }
                    // Do not allocate a huge coordinate table solely because an
                    // untrusted header declared many tiles; grow as data arrives.
                    seen.try_reserve(1).map_err(|_| ProjectError::MemoryLimit)?;
                    if !seen.insert((tx, ty)) {
                        return Err(ProjectError::InvalidData);
                    }
                    let (x0, y0) = (tx * TILE_SIZE, ty * TILE_SIZE);
                    for y in y0..(y0 + TILE_SIZE).min(height) {
                        for x in x0..(x0 + TILE_SIZE).min(width) {
                            pixels
                                .set(x, y, r.color()?)
                                .map_err(|_| ProjectError::MemoryLimit)?;
                        }
                    }
                }
            }
            layers.push(Layer {
                name,
                visible: props[0] == 1,
                opacity: props[1],
                pixels: Arc::new(pixels),
            });
        }
        let calculated = r.crc.finalize();
        let mut stored = [0; 4];
        r.inner.read_exact(&mut stored)?;
        if calculated != u32::from_le_bytes(stored) {
            return Err(ProjectError::Checksum);
        }
        let mut trailing = [0; 1];
        if r.inner.read(&mut trailing)? != 0 {
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
            stroke: None,
            history: History::default(),
        })
    }
}
struct CheckedWriter<W> {
    inner: W,
    crc: crc32fast::Hasher,
}
impl<W: Write> CheckedWriter<W> {
    fn bytes(&mut self, bytes: &[u8]) -> Result<(), ProjectError> {
        self.inner.write_all(bytes)?;
        self.crc.update(bytes);
        Ok(())
    }
    fn number(&mut self, value: u32) -> Result<(), ProjectError> {
        self.bytes(&value.to_le_bytes())
    }
}
struct CheckedReader<R> {
    inner: R,
    crc: crc32fast::Hasher,
}
impl<R: Read> CheckedReader<R> {
    fn bytes(&mut self, bytes: &mut [u8]) -> Result<(), ProjectError> {
        self.inner.read_exact(bytes)?;
        self.crc.update(bytes);
        Ok(())
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], ProjectError> {
        let mut bytes = [0; N];
        self.bytes(&mut bytes)?;
        Ok(bytes)
    }
    fn number(&mut self) -> Result<u32, ProjectError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn color(&mut self) -> Result<Color, ProjectError> {
        let c = self.array::<4>()?;
        Ok(Color::rgba(c[0], c[1], c[2], c[3]))
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

#[cfg(test)]
mod streaming_tests {
    use super::*;
    #[test]
    fn full_dimension_sparse_project_streams_one_changed_tile_and_preserves_far_corner() {
        let mut d = Document::new_with_background(
            MAX_DIMENSION,
            MAX_DIMENSION,
            144.125,
            Color::rgba(255, 255, 255, 255),
        )
        .unwrap();
        let corner = Color::rgba(17, 92, 203, 63);
        Arc::make_mut(&mut d.state.layers[0].pixels)
            .set(MAX_DIMENSION - 1, MAX_DIMENSION - 1, corner)
            .unwrap();
        let mut bytes = Vec::new();
        d.write_project(&mut bytes).unwrap();
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 2);
        assert!(bytes.len() < 70_000);
        let loaded = Document::read_project(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(loaded.snapshot(), d.snapshot());
        assert_eq!(
            loaded.layers()[0]
                .pixels
                .get(MAX_DIMENSION - 1, MAX_DIMENSION - 1),
            corner
        );
        assert_eq!(loaded.layers()[0].pixels.tile_positions().count(), 1);
        assert_eq!(
            loaded.layers()[0].pixels.get(131_072, 131_072),
            Color::rgba(255, 255, 255, 255)
        );
    }
    #[test]
    fn legacy_dense_v1_import_preserves_hidden_rgb_and_upgrades_to_v2() {
        let mut bytes = MAGIC.to_vec();
        for n in [1, 2, 1, 96_000, 0, 1, 1] {
            bytes.extend_from_slice(&u32::to_le_bytes(n));
        }
        bytes.extend_from_slice(b"A");
        bytes.extend_from_slice(&[1, 255]);
        bytes.extend_from_slice(&[12, 34, 56, 0, 78, 90, 123, 127]);
        bytes.extend_from_slice(&crc32fast::hash(&bytes).to_le_bytes());
        let loaded = Document::read_project(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(loaded.layers()[0].pixels()[0], Color::rgba(12, 34, 56, 0));
        assert_eq!(
            loaded.layers()[0].pixels()[1],
            Color::rgba(78, 90, 123, 127)
        );
        assert_eq!(
            u32::from_le_bytes(loaded.encode_project().unwrap()[8..12].try_into().unwrap()),
            2
        );
    }
    #[test]
    fn streaming_reader_rejects_duplicate_tiles_even_when_checksum_is_valid() {
        let mut d = Document::new(256, 128).unwrap();
        let pixels = Arc::make_mut(&mut d.state.layers[0].pixels);
        pixels.set(0, 0, Color::rgba(255, 0, 0, 255)).unwrap();
        pixels.set(128, 0, Color::rgba(0, 0, 255, 255)).unwrap();
        let mut bytes = d.encode_project().unwrap();
        let first = 32 + 4 + "Background".len() + 2 + 4 + 4;
        let second = first + 8 + 128 * 128 * 4;
        let coordinates = bytes[first..first + 8].to_vec();
        bytes[second..second + 8].copy_from_slice(&coordinates);
        let end = bytes.len() - 4;
        let crc = crc32fast::hash(&bytes[..end]);
        bytes[end..].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(
            Document::read_project(std::io::Cursor::new(bytes)).err(),
            Some(ProjectError::InvalidData)
        );
    }
}
