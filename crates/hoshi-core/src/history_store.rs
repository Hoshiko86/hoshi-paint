//! Session-only, transactional undo pack. Tiles are compressed once and shared
//! by manifests; only weak references to their optional RAM copies are retained.
use crate::pixels::Tile;
use crate::{
    Color, Document, Error, Layer, MAX_LAYERS, MemoryReservation, PixelStore, State, TILE_SIZE,
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};
const PACK_MAGIC: &[u8; 16] = b"HOSHIUNDO-PACK01";
const STATE_MAGIC: &[u8; 8] = b"HSTATE01";
const TILE_MAGIC: &[u8; 8] = b"HTILE001";
const HEADER: usize = 20;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StateRecord {
    offset: u64,
    bytes: u64,
    raw_bytes: u64,
}
struct Blob {
    record: StateRecord,
    cached: Weak<Tile>,
}
trait ScratchFile: Read + Write + Seek + Send {
    fn truncate(&mut self, len: u64) -> std::io::Result<()>;
    fn sync(&mut self) -> std::io::Result<()>;
}
impl ScratchFile for File {
    fn truncate(&mut self, len: u64) -> std::io::Result<()> {
        self.set_len(len)
    }
    fn sync(&mut self) -> std::io::Result<()> {
        self.sync_data()
    }
}
struct Bytes {
    data: Vec<u8>,
    _memory: MemoryReservation,
}
impl Bytes {
    fn capacity(bytes: usize) -> Result<Self, Error> {
        let memory = MemoryReservation::new(bytes as u64)?;
        let mut data = Vec::new();
        data.try_reserve_exact(bytes)
            .map_err(|_| Error::MemoryLimit)?;
        Ok(Self {
            data,
            _memory: memory,
        })
    }
    fn zeroed(bytes: usize) -> Result<Self, Error> {
        let mut result = Self::capacity(bytes)?;
        result.data.resize(bytes, 0);
        Ok(result)
    }
    fn number(&mut self, n: u32) {
        self.data.extend_from_slice(&n.to_le_bytes())
    }
    fn id(&mut self, n: u64) {
        self.data.extend_from_slice(&n.to_le_bytes())
    }
    fn color(&mut self, c: Color) {
        self.data.extend_from_slice(&[c.r, c.g, c.b, c.a])
    }
}
struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let end = self
            .position
            .checked_add(len)
            .ok_or(Error::HistoryCorrupt)?;
        let result = self
            .bytes
            .get(self.position..end)
            .ok_or(Error::HistoryCorrupt)?;
        self.position = end;
        Ok(result)
    }
    fn number(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| Error::HistoryCorrupt)?,
        ))
    }
    fn id(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| Error::HistoryCorrupt)?,
        ))
    }
    fn color(&mut self) -> Result<Color, Error> {
        let c = self.take(4)?;
        Ok(Color::rgba(c[0], c[1], c[2], c[3]))
    }
    fn end(&self) -> Result<(), Error> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::HistoryCorrupt)
        }
    }
}
pub(crate) struct HistoryStore {
    file: Box<dyn ScratchFile>,
    end: u64,
    blobs: HashMap<u64, Blob>,
    directory: Option<PathBuf>,
}
impl HistoryStore {
    pub(crate) fn new(directory: Option<&Path>) -> Result<Self, Error> {
        let file = match directory {
            Some(path) => tempfile::tempfile_in(path),
            None => tempfile::tempfile(),
        }
        .map_err(|_| Error::HistoryIo)?;
        Self::with_file(Box::new(file), directory.map(Path::to_path_buf))
    }
    fn with_file(
        mut file: Box<dyn ScratchFile>,
        directory: Option<PathBuf>,
    ) -> Result<Self, Error> {
        file.write_all(PACK_MAGIC).map_err(|_| Error::HistoryIo)?;
        file.sync().map_err(|_| Error::HistoryIo)?;
        Ok(Self {
            file,
            end: PACK_MAGIC.len() as u64,
            blobs: HashMap::new(),
            directory,
        })
    }
    pub(crate) fn bytes(&self) -> u64 {
        self.end
    }
    pub(crate) fn archive_many(&mut self, states: &[&State]) -> Result<Vec<StateRecord>, Error> {
        let start = self.end;
        let mut staged = HashMap::<u64, Blob>::new();
        let mut refreshed = Vec::new();
        let mut records = Vec::new();
        records
            .try_reserve_exact(states.len())
            .map_err(|_| Error::MemoryLimit)?;
        let result = (|| {
            for state in states {
                for layer in &state.layers {
                    for (_, _, tile) in layer.pixels.shared_tiles() {
                        let id = tile.id();
                        if self.blobs.contains_key(&id) {
                            refreshed.try_reserve(1).map_err(|_| Error::MemoryLimit)?;
                            refreshed.push((id, Arc::downgrade(tile)));
                            continue;
                        }
                        if staged.contains_key(&id) {
                            continue;
                        }
                        staged.try_reserve(1).map_err(|_| Error::MemoryLimit)?;
                        let mut raw = Bytes::capacity(24 + tile.colors().len() * 4)?;
                        raw.data.extend_from_slice(TILE_MAGIC);
                        raw.id(id);
                        raw.number(tile.width());
                        raw.number(tile.height());
                        for &color in tile.colors() {
                            raw.color(color)
                        }
                        let record = self.append(&raw.data)?;
                        staged.insert(
                            id,
                            Blob {
                                record,
                                cached: Arc::downgrade(tile),
                            },
                        );
                    }
                }
                let raw = manifest(state)?;
                records.push(self.append(&raw.data)?);
            }
            self.blobs
                .try_reserve(staged.len())
                .map_err(|_| Error::MemoryLimit)?;
            // A previous rollback may have been unable to trim its failed tail.
            // A successful transaction must reclaim it before publishing the end.
            self.file.truncate(self.end).map_err(|_| Error::HistoryIo)?;
            self.file.sync().map_err(|_| Error::HistoryIo)?;
            Ok(())
        })();
        if let Err(error) = result {
            // Only bytes beyond the last committed pack end were touched. If the
            // filesystem also refuses truncation, that unreferenced tail is never read.
            self.end = start;
            let _ = self.file.truncate(start);
            let _ = self.file.seek(SeekFrom::Start(start));
            return Err(error);
        }
        self.blobs.extend(staged);
        for (id, cached) in refreshed {
            if let Some(blob) = self.blobs.get_mut(&id) {
                blob.cached = cached
            }
        }
        Ok(records)
    }
    fn append(&mut self, raw: &[u8]) -> Result<StateRecord, Error> {
        let maximum = lz4_flex::block::get_maximum_output_size(raw.len());
        let mut packed = Bytes::zeroed(maximum)?;
        let count = lz4_flex::block::compress_into(raw, &mut packed.data)
            .map_err(|_| Error::HistoryCorrupt)?;
        let record = StateRecord {
            offset: self.end,
            bytes: HEADER as u64 + count as u64,
            raw_bytes: raw.len() as u64,
        };
        self.file
            .seek(SeekFrom::Start(self.end))
            .map_err(|_| Error::HistoryIo)?;
        self.file
            .write_all(&(raw.len() as u64).to_le_bytes())
            .map_err(|_| Error::HistoryIo)?;
        self.file
            .write_all(&(count as u64).to_le_bytes())
            .map_err(|_| Error::HistoryIo)?;
        self.file
            .write_all(&crc32fast::hash(&packed.data[..count]).to_le_bytes())
            .map_err(|_| Error::HistoryIo)?;
        self.file
            .write_all(&packed.data[..count])
            .map_err(|_| Error::HistoryIo)?;
        self.end = self
            .end
            .checked_add(record.bytes)
            .ok_or(Error::MemoryLimit)?;
        Ok(record)
    }
    fn header(&mut self, record: StateRecord) -> Result<(usize, usize, u32), Error> {
        if record.offset < PACK_MAGIC.len() as u64
            || record.bytes < HEADER as u64
            || record
                .offset
                .checked_add(record.bytes)
                .is_none_or(|end| end > self.end)
        {
            return Err(Error::HistoryCorrupt);
        }
        self.file
            .seek(SeekFrom::Start(record.offset))
            .map_err(|_| Error::HistoryIo)?;
        let mut header = [0; HEADER];
        self.file
            .read_exact(&mut header)
            .map_err(|_| Error::HistoryIo)?;
        let raw = u64::from_le_bytes(header[..8].try_into().map_err(|_| Error::HistoryCorrupt)?);
        let packed = u64::from_le_bytes(
            header[8..16]
                .try_into()
                .map_err(|_| Error::HistoryCorrupt)?,
        );
        if raw != record.raw_bytes || packed.checked_add(HEADER as u64) != Some(record.bytes) {
            return Err(Error::HistoryCorrupt);
        }
        Ok((
            usize::try_from(raw).map_err(|_| Error::MemoryLimit)?,
            usize::try_from(packed).map_err(|_| Error::MemoryLimit)?,
            u32::from_le_bytes(header[16..].try_into().map_err(|_| Error::HistoryCorrupt)?),
        ))
    }
    fn read(&mut self, record: StateRecord) -> Result<Bytes, Error> {
        let (raw_len, packed_len, crc) = self.header(record)?;
        let mut packed = Bytes::zeroed(packed_len)?;
        self.file
            .read_exact(&mut packed.data)
            .map_err(|_| Error::HistoryIo)?;
        if crc32fast::hash(&packed.data) != crc {
            return Err(Error::HistoryCorrupt);
        }
        let mut raw = Bytes::zeroed(raw_len)?;
        let count = lz4_flex::block::decompress_into(&packed.data, &mut raw.data)
            .map_err(|_| Error::HistoryCorrupt)?;
        if count != raw_len {
            return Err(Error::HistoryCorrupt);
        }
        Ok(raw)
    }
    fn tile(&mut self, id: u64) -> Result<Arc<Tile>, Error> {
        let blob = self.blobs.get(&id).ok_or(Error::HistoryCorrupt)?;
        if let Some(tile) = blob.cached.upgrade() {
            return Ok(tile);
        }
        let raw = self.read(blob.record)?;
        let mut cursor = Cursor {
            bytes: &raw.data,
            position: 0,
        };
        if cursor.take(8)? != TILE_MAGIC || cursor.id()? != id {
            return Err(Error::HistoryCorrupt);
        }
        let (width, height) = (cursor.number()?, cursor.number()?);
        let remaining = cursor.bytes.len() - cursor.position;
        let tile = Arc::new(Tile::from_archived(
            id,
            width,
            height,
            cursor.take(remaining)?,
        )?);
        cursor.end()?;
        self.blobs.get_mut(&id).ok_or(Error::HistoryCorrupt)?.cached = Arc::downgrade(&tile);
        Ok(tile)
    }
    pub(crate) fn restore(&mut self, record: StateRecord) -> Result<State, Error> {
        let raw = self.read(record)?;
        parse_manifest(&raw.data, |id| self.tile(id))
    }
    /// Reclaim discarded redo states without mutating the live pack until a
    /// replacement file and every remapped reference have been written and synced.
    pub(crate) fn compact(&mut self, records: &mut [StateRecord]) -> Result<(), Error> {
        let mut replacement = Self::new(self.directory.as_deref())?;
        let mut remapped = Vec::new();
        remapped
            .try_reserve_exact(records.len())
            .map_err(|_| Error::MemoryLimit)?;
        let mut manifests = HashMap::new();
        for &record in records.iter() {
            if let Some(&mapped) = manifests.get(&record) {
                remapped.push(mapped);
                continue;
            }
            let raw = self.read(record)?;
            let ids = manifest_ids(&raw.data)?;
            for id in ids {
                if replacement.blobs.contains_key(&id) {
                    continue;
                }
                replacement
                    .blobs
                    .try_reserve(1)
                    .map_err(|_| Error::MemoryLimit)?;
                let old = self.blobs.get(&id).ok_or(Error::HistoryCorrupt)?;
                let old_record = old.record;
                let cached = old.cached.clone();
                let copied = self.copy_record(old_record, &mut replacement)?;
                replacement.blobs.insert(
                    id,
                    Blob {
                        record: copied,
                        cached,
                    },
                );
            }
            let mapped = self.copy_record(record, &mut replacement)?;
            manifests.try_reserve(1).map_err(|_| Error::MemoryLimit)?;
            manifests.insert(record, mapped);
            remapped.push(mapped);
        }
        replacement.file.sync().map_err(|_| Error::HistoryIo)?;
        records.copy_from_slice(&remapped);
        std::mem::swap(self, &mut replacement);
        Ok(())
    }
    fn copy_record(
        &mut self,
        record: StateRecord,
        destination: &mut Self,
    ) -> Result<StateRecord, Error> {
        let (raw_len, packed_len, expected_crc) = self.header(record)?;
        let mut buffer = Bytes::zeroed(packed_len.min(64 * 1024))?;
        let copied = StateRecord {
            offset: destination.end,
            bytes: record.bytes,
            raw_bytes: record.raw_bytes,
        };
        destination
            .file
            .seek(SeekFrom::Start(destination.end))
            .map_err(|_| Error::HistoryIo)?;
        destination
            .file
            .write_all(&(raw_len as u64).to_le_bytes())
            .map_err(|_| Error::HistoryIo)?;
        destination
            .file
            .write_all(&(packed_len as u64).to_le_bytes())
            .map_err(|_| Error::HistoryIo)?;
        destination
            .file
            .write_all(&expected_crc.to_le_bytes())
            .map_err(|_| Error::HistoryIo)?;
        let mut crc = crc32fast::Hasher::new();
        let mut remaining = packed_len;
        while remaining > 0 {
            let count = remaining.min(buffer.data.len());
            self.file
                .read_exact(&mut buffer.data[..count])
                .map_err(|_| Error::HistoryIo)?;
            crc.update(&buffer.data[..count]);
            destination
                .file
                .write_all(&buffer.data[..count])
                .map_err(|_| Error::HistoryIo)?;
            remaining -= count
        }
        if crc.finalize() != expected_crc {
            return Err(Error::HistoryCorrupt);
        }
        destination.end = destination
            .end
            .checked_add(record.bytes)
            .ok_or(Error::MemoryLimit)?;
        Ok(copied)
    }
}
fn manifest(state: &State) -> Result<Bytes, Error> {
    let mut size = 28usize;
    for layer in &state.layers {
        size = size
            .checked_add(14)
            .and_then(|n| n.checked_add(layer.name.len()))
            .and_then(|n| n.checked_add(layer.pixels.tile_positions().count().checked_mul(16)?))
            .ok_or(Error::MemoryLimit)?;
    }
    let mut raw = Bytes::capacity(size)?;
    raw.data.extend_from_slice(STATE_MAGIC);
    for value in [
        state.width,
        state.height,
        state.resolution,
        u32::try_from(state.selected).map_err(|_| Error::MemoryLimit)?,
        u32::try_from(state.layers.len()).map_err(|_| Error::MemoryLimit)?,
    ] {
        raw.number(value)
    }
    for layer in &state.layers {
        raw.number(u32::try_from(layer.name.len()).map_err(|_| Error::MemoryLimit)?);
        raw.data.extend_from_slice(layer.name.as_bytes());
        raw.data
            .extend_from_slice(&[u8::from(layer.visible), layer.opacity]);
        raw.color(layer.pixels.background());
        raw.number(
            u32::try_from(layer.pixels.tile_positions().count()).map_err(|_| Error::MemoryLimit)?,
        );
        for (x, y, tile) in layer.pixels.shared_tiles() {
            raw.number(x);
            raw.number(y);
            raw.id(tile.id());
        }
    }
    Ok(raw)
}
fn preamble<'a>(raw: &'a [u8]) -> Result<(Cursor<'a>, u32, u32, u32, usize, usize), Error> {
    let mut c = Cursor {
        bytes: raw,
        position: 0,
    };
    if c.take(8)? != STATE_MAGIC {
        return Err(Error::HistoryCorrupt);
    }
    let (width, height, resolution, selected, count) = (
        c.number()?,
        c.number()?,
        c.number()?,
        c.number()? as usize,
        c.number()? as usize,
    );
    Document::validate_image_size(width, height).map_err(|_| Error::HistoryCorrupt)?;
    Document::validate_resolution(f64::from(resolution) / 1000.)
        .map_err(|_| Error::HistoryCorrupt)?;
    if count == 0 || count > MAX_LAYERS || selected >= count {
        return Err(Error::HistoryCorrupt);
    }
    Ok((c, width, height, resolution, selected, count))
}
fn layer_header(
    c: &mut Cursor<'_>,
    width: u32,
    height: u32,
) -> Result<(String, bool, u8, Color, u32), Error> {
    let length = c.number()? as usize;
    let name = std::str::from_utf8(c.take(length)?)
        .map_err(|_| Error::HistoryCorrupt)?
        .to_owned();
    let props = c.take(2)?;
    if props[0] > 1 {
        return Err(Error::HistoryCorrupt);
    }
    let (visible, opacity) = (props[0] == 1, props[1]);
    let background = c.color()?;
    let count = c.number()?;
    if u64::from(count)
        > u64::from(width.div_ceil(TILE_SIZE)) * u64::from(height.div_ceil(TILE_SIZE))
    {
        return Err(Error::HistoryCorrupt);
    }
    Ok((name, visible, opacity, background, count))
}
fn parse_manifest(
    raw: &[u8],
    mut tile: impl FnMut(u64) -> Result<Arc<Tile>, Error>,
) -> Result<State, Error> {
    let (mut c, width, height, resolution, selected, count) = preamble(raw)?;
    let mut layers = Vec::new();
    layers
        .try_reserve_exact(count)
        .map_err(|_| Error::MemoryLimit)?;
    for _ in 0..count {
        let (name, visible, opacity, background, count) = layer_header(&mut c, width, height)?;
        let mut pixels = PixelStore::new(width, height, background);
        for _ in 0..count {
            let (x, y, id) = (c.number()?, c.number()?, c.id()?);
            pixels.insert_shared_tile(x, y, tile(id)?)?;
        }
        layers.push(Layer {
            name,
            visible,
            opacity,
            pixels: Arc::new(pixels),
        });
    }
    c.end()?;
    Ok(State {
        width,
        height,
        resolution,
        layers,
        selected,
    })
}
fn manifest_ids(raw: &[u8]) -> Result<Vec<u64>, Error> {
    let (mut c, width, height, _, _, count) = preamble(raw)?;
    let mut ids = Vec::new();
    for _ in 0..count {
        let (_, _, _, _, count) = layer_header(&mut c, width, height)?;
        let mut seen = HashSet::new();
        for _ in 0..count {
            let (x, y, id) = (c.number()?, c.number()?, c.id()?);
            if x >= width.div_ceil(TILE_SIZE) || y >= height.div_ceil(TILE_SIZE) || id == 0 {
                return Err(Error::HistoryCorrupt);
            }
            seen.try_reserve(1).map_err(|_| Error::MemoryLimit)?;
            if !seen.insert((x, y)) {
                return Err(Error::HistoryCorrupt);
            }
            ids.try_reserve(1).map_err(|_| Error::MemoryLimit)?;
            ids.push(id);
        }
    }
    c.end()?;
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
    #[derive(Default)]
    struct Faults {
        write_remaining: AtomicI64,
        sync: AtomicBool,
        read: AtomicBool,
        truncate: AtomicBool,
    }
    struct FaultFile {
        inner: Box<dyn ScratchFile>,
        faults: Arc<Faults>,
    }
    impl Read for FaultFile {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.faults.read.load(Ordering::Relaxed) {
                Err(std::io::Error::other("injected read failure"))
            } else {
                self.inner.read(buf)
            }
        }
    }
    impl Write for FaultFile {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let remaining = self.faults.write_remaining.load(Ordering::Relaxed);
            if remaining == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::StorageFull,
                    "injected disk full",
                ));
            }
            let count = if remaining < 0 {
                buf.len()
            } else {
                buf.len().min(remaining as usize)
            };
            let written = self.inner.write(&buf[..count])?;
            if remaining >= 0 {
                self.faults
                    .write_remaining
                    .fetch_sub(written as i64, Ordering::Relaxed);
            }
            Ok(written)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }
    impl Seek for FaultFile {
        fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(p)
        }
    }
    impl ScratchFile for FaultFile {
        fn truncate(&mut self, len: u64) -> std::io::Result<()> {
            if self.faults.truncate.load(Ordering::Relaxed) {
                Err(std::io::Error::other("injected truncate failure"))
            } else {
                self.inner.truncate(len)
            }
        }
        fn sync(&mut self) -> std::io::Result<()> {
            if self.faults.sync.load(Ordering::Relaxed) {
                Err(std::io::Error::other("injected sync failure"))
            } else {
                self.inner.sync()
            }
        }
    }
    fn fixture() -> State {
        let mut state = Document::new(129, 131).unwrap().state;
        let mut pixels = PixelStore::new(129, 131, Color::rgba(5, 7, 11, 0));
        pixels.set(0, 0, Color::rgba(127, 64, 239, 128)).unwrap();
        pixels
            .set(128, 130, Color::rgba(90, 120, 150, 255))
            .unwrap();
        state.layers[0].pixels = Arc::new(pixels);
        state
    }
    fn change(state: &State, color: Color) -> State {
        let mut next = state.clone();
        let mut pixels = (*next.layers[0].pixels).clone();
        pixels.set(0, 0, color).unwrap();
        next.layers[0].pixels = Arc::new(pixels);
        next
    }
    fn faulty() -> (HistoryStore, Arc<Faults>) {
        let faults = Arc::new(Faults::default());
        faults.write_remaining.store(-1, Ordering::Relaxed);
        let file = FaultFile {
            inner: Box::new(tempfile::tempfile().unwrap()),
            faults: faults.clone(),
        };
        (
            HistoryStore::with_file(Box::new(file), None).unwrap(),
            faults,
        )
    }
    fn inject(store: &mut HistoryStore) -> Arc<Faults> {
        let faults = Arc::new(Faults::default());
        faults.write_remaining.store(-1, Ordering::Relaxed);
        let inner = std::mem::replace(&mut store.file, Box::new(tempfile::tempfile().unwrap()));
        store.file = Box::new(FaultFile {
            inner,
            faults: faults.clone(),
        });
        faults
    }
    #[test]
    fn document_checkpoint_and_navigation_failures_preserve_pixels_preview_and_redo() {
        let mut document = Document::new(4, 4).unwrap();
        document.rename_layer(0, "One").unwrap();
        document.rename_layer(0, "Two").unwrap();
        document.undo().unwrap();
        let faults = inject(document.history.store_mut().unwrap());
        let before = document.snapshot();
        let count = document.history_steps().len();
        let position = document.history_position();
        faults.write_remaining.store(0, Ordering::Relaxed);
        assert_eq!(
            document.rename_layer(0, "Must not commit"),
            Err(Error::HistoryIo)
        );
        assert_eq!(document.snapshot(), before);
        assert_eq!(document.history_steps().len(), count);
        assert_eq!(document.history_position(), position);
        assert_eq!(document.redo_len(), 1);
        let shape = crate::Shape {
            kind: crate::ShapeKind::Rectangle,
            x: 0,
            y: 0,
            width: 2,
            height: 2,
            rotation_degrees: 0,
            style: crate::ShapeStyle::Fill(Color::rgba(255, 0, 0, 255)),
        };
        document.begin_shape(shape).unwrap();
        assert_eq!(document.commit_shape(), Err(Error::HistoryIo));
        assert_eq!(document.snapshot(), before);
        assert_eq!(document.active_shape(), Some(shape));
        assert_eq!(document.redo_len(), 1);
        document.cancel_shape();
        faults.write_remaining.store(-1, Ordering::Relaxed);
        document.redo().unwrap();
        assert_eq!(document.layers()[0].name(), "Two");
        let before = document.snapshot();
        let position = document.history_position();
        faults.read.store(true, Ordering::Relaxed);
        assert_eq!(document.go_to_history_step(0), Err(Error::HistoryIo));
        assert_eq!(document.snapshot(), before);
        assert_eq!(document.history_position(), position);
        faults.read.store(false, Ordering::Relaxed);
        document.go_to_history_step(0).unwrap();
        assert_eq!(document.layers()[0].name(), "Background");
    }
    #[test]
    fn selection_changes_do_not_make_undo_or_redo_write_to_a_full_disk() {
        let mut document = Document::new(4, 4).unwrap();
        document.add_layer("Top").unwrap();
        document.rename_layer(1, "Renamed").unwrap();
        document.select_layer(0).unwrap();
        let faults = inject(document.history.store_mut().unwrap());
        faults.write_remaining.store(0, Ordering::Relaxed);
        document.undo().unwrap();
        assert_eq!(document.layers()[1].name(), "Top");
        document.redo().unwrap();
        assert_eq!(document.layers()[1].name(), "Renamed");
        assert_eq!(document.selected_layer(), 0);
    }
    #[test]
    fn archived_tiles_are_deduplicated_compressed_and_released_from_ram() {
        let mut store = HistoryStore::new(None).unwrap();
        let first = fixture();
        let next = change(&first, Color::rgba(9, 8, 7, 255));
        let records = store.archive_many(&[&first, &next, &first]).unwrap();
        assert_eq!(
            store.blobs.len(),
            3,
            "the unchanged edge tile is stored once"
        );
        assert!(
            store.bytes() < 4096,
            "mostly uniform tiles should compress, not occupy three64KiBframes"
        );
        let restored = store.restore(records[0]).unwrap();
        assert_eq!(restored, first);
        assert!(Arc::ptr_eq(
            first.layers[0]
                .pixels
                .shared_tiles()
                .find(|(x, _, _)| *x == 0)
                .unwrap()
                .2,
            restored.layers[0]
                .pixels
                .shared_tiles()
                .find(|(x, _, _)| *x == 0)
                .unwrap()
                .2
        ));
        drop(restored);
        drop(first);
        drop(next);
        assert!(
            store
                .blobs
                .values()
                .all(|blob| blob.cached.strong_count() == 0),
            "history must not pin old tiles in RAM"
        );
        let restored = store.restore(records[1]).unwrap();
        assert_eq!(
            restored.layers[0].pixels.get(0, 0),
            Color::rgba(9, 8, 7, 255)
        );
        assert_eq!(
            restored.layers[0].pixels.get(128, 130),
            Color::rgba(90, 120, 150, 255)
        );
        assert_eq!(
            restored.layers[0].pixels.get(12, 12),
            Color::rgba(5, 7, 11, 0)
        );
    }
    #[test]
    fn failed_partial_writes_and_sync_leave_previous_records_readable() {
        let (mut store, faults) = faulty();
        let first = fixture();
        let record = store.archive_many(&[&first]).unwrap()[0];
        let before = store.bytes();
        let blobs = store.blobs.len();
        let next = change(&first, Color::rgba(1, 2, 3, 255));
        faults.write_remaining.store(27, Ordering::Relaxed);
        assert_eq!(store.archive_many(&[&next]).err(), Some(Error::HistoryIo));
        assert_eq!(store.bytes(), before);
        assert_eq!(store.blobs.len(), blobs);
        assert_eq!(store.restore(record).unwrap(), first);
        faults.write_remaining.store(-1, Ordering::Relaxed);
        faults.sync.store(true, Ordering::Relaxed);
        assert_eq!(store.archive_many(&[&next]).err(), Some(Error::HistoryIo));
        assert_eq!(store.bytes(), before);
        assert_eq!(store.blobs.len(), blobs);
        faults.sync.store(false, Ordering::Relaxed);
        let record = store.archive_many(&[&next]).unwrap()[0];
        assert_eq!(store.restore(record).unwrap(), next);
        faults.read.store(true, Ordering::Relaxed);
        assert_eq!(store.restore(record).err(), Some(Error::HistoryIo));
        faults.read.store(false, Ordering::Relaxed);
        assert_eq!(store.restore(record).unwrap(), next);
    }
    #[test]
    fn recovery_reclaims_a_tail_when_the_original_rollback_could_not_truncate() {
        let (mut store, faults) = faulty();
        let first = fixture();
        let record = store.archive_many(&[&first]).unwrap()[0];
        let before = store.bytes();
        let mut noisy = first.clone();
        let mut pixels = (*noisy.layers[0].pixels).clone();
        for y in 0..128 {
            for x in 0..128 {
                let i = y * 128 + x;
                pixels
                    .set(
                        x,
                        y,
                        Color::rgba(
                            (i.wrapping_mul(1973) >> 5) as u8,
                            (i.wrapping_mul(9277) >> 3) as u8,
                            (i.wrapping_mul(26699) >> 1) as u8,
                            (i >> 4) as u8,
                        ),
                    )
                    .unwrap();
            }
        }
        noisy.layers[0].pixels = Arc::new(pixels);
        faults.write_remaining.store(2048, Ordering::Relaxed);
        faults.truncate.store(true, Ordering::Relaxed);
        assert_eq!(store.archive_many(&[&noisy]).err(), Some(Error::HistoryIo));
        assert_eq!(store.bytes(), before);
        assert!(store.file.seek(SeekFrom::End(0)).unwrap() > before);
        assert_eq!(store.restore(record).unwrap(), first);
        faults.write_remaining.store(-1, Ordering::Relaxed);
        faults.truncate.store(false, Ordering::Relaxed);
        let mut recovered = first.clone();
        recovered.layers[0].name = "Recovered".into();
        let record = store.archive_many(&[&recovered]).unwrap()[0];
        assert_eq!(store.file.seek(SeekFrom::End(0)).unwrap(), store.bytes());
        assert_eq!(store.restore(record).unwrap(), recovered);
    }
    #[test]
    fn checksum_failures_do_not_rewrite_or_truncate_the_pack() {
        let mut store = HistoryStore::new(None).unwrap();
        let state = fixture();
        let record = store.archive_many(&[&state]).unwrap()[0];
        let bytes = store.bytes();
        store
            .file
            .seek(SeekFrom::Start(record.offset + HEADER as u64))
            .unwrap();
        let mut byte = [0];
        store.file.read_exact(&mut byte).unwrap();
        byte[0] ^= 0x80;
        store
            .file
            .seek(SeekFrom::Start(record.offset + HEADER as u64))
            .unwrap();
        store.file.write_all(&byte).unwrap();
        assert_eq!(store.restore(record).err(), Some(Error::HistoryCorrupt));
        assert_eq!(store.bytes(), bytes);
    }
    #[test]
    fn compaction_reclaims_discarded_branches_and_commits_offsets_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = HistoryStore::new(Some(dir.path())).unwrap();
        let first = fixture();
        let second = change(&first, Color::rgba(1, 2, 3, 255));
        let third = change(&second, Color::rgba(100, 200, 50, 255));
        let refs = store.archive_many(&[&first, &second, &third]).unwrap();
        let old = store.bytes();
        let mut retained = vec![refs[0], refs[0]];
        store.compact(&mut retained).unwrap();
        assert_eq!(retained[0], retained[1]);
        assert!(store.bytes() < old);
        assert_eq!(store.blobs.len(), 2);
        assert_eq!(store.restore(retained[0]).unwrap(), first);
        let before = store.bytes();
        let old_refs = retained.clone();
        store.directory = Some(dir.path().join("does-not-exist"));
        assert_eq!(store.compact(&mut retained), Err(Error::HistoryIo));
        assert_eq!(retained, old_refs);
        assert_eq!(store.bytes(), before);
        assert_eq!(store.restore(retained[0]).unwrap(), first);
        drop(store);
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "private session scratch is removed when its file closes"
        );
    }
}
