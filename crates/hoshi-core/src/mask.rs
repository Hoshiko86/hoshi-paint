//! Sparse, maximum-combined masks with an implicit value for unstored tiles.
use crate::{Error, MemoryReservation, TILE_SIZE};
use std::collections::HashMap;

struct MaskTile {
    values: Vec<u8>,
    _reservation: MemoryReservation,
}

impl MaskTile {
    fn new(background: u8) -> Result<Self, Error> {
        let count = (TILE_SIZE * TILE_SIZE) as usize;
        let reservation = MemoryReservation::new(count as u64)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::MemoryLimit)?;
        values.resize(count, background);
        Ok(Self {
            values,
            _reservation: reservation,
        })
    }

    fn set_max(&mut self, x: u32, y: u32, value: u8) -> Result<bool, Error> {
        let pixel = self
            .values
            .get_mut(pixel_index(x, y))
            .ok_or(Error::MemoryLimit)?;
        let changed = value > *pixel;
        *pixel = (*pixel).max(value);
        Ok(changed)
    }
}

/// A raw tile's stored bytes or implicit background; callers clip edge-tile padding.
#[derive(Clone, Copy)]
pub(crate) enum TileView<'a> {
    Stored(&'a [u8]),
    Background(u8),
}

impl TileView<'_> {
    /// Reads tile-local coordinates; coordinates outside a 128px tile return zero.
    pub(crate) fn get(self, x: u32, y: u32) -> u8 {
        if x >= TILE_SIZE || y >= TILE_SIZE {
            return 0;
        }
        match self {
            Self::Stored(values) => values.get(pixel_index(x, y)).copied().unwrap_or(0),
            Self::Background(value) => value,
        }
    }
}

/// Document-sized sparse coverage; each allocated tile owns its memory reservation.
pub(crate) struct MaskStore {
    width: u32,
    height: u32,
    background: u8,
    tiles: HashMap<(u32, u32), MaskTile>,
    bounds: Option<(u32, u32, u32, u32)>,
}

impl MaskStore {
    /// Creates an unallocated mask; a nonzero background covers the whole document.
    pub(crate) fn new(width: u32, height: u32, background: u8) -> Self {
        Self {
            width,
            height,
            background,
            tiles: HashMap::new(),
            bounds: (background != 0 && width != 0 && height != 0).then_some((0, 0, width, height)),
        }
    }

    /// Reads document coordinates; anything outside the document is zero.
    pub(crate) fn get(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        self.tile(x / TILE_SIZE, y / TILE_SIZE)
            .get(x % TILE_SIZE, y % TILE_SIZE)
    }

    /// Combines by maximum, reserving a missing tile before any pixel or bounds changes.
    pub(crate) fn set_max(&mut self, x: u32, y: u32, value: u8) -> Result<(), Error> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }
        let key = (x / TILE_SIZE, y / TILE_SIZE);
        let changed = if let Some(tile) = self.tiles.get_mut(&key) {
            tile.set_max(x, y, value)?
        } else {
            if value <= self.background {
                return Ok(());
            }
            let mut pending = self.reserve_tiles();
            pending.reserve_pixel(x, y)?;
            pending.commit()?;
            self.tiles
                .get_mut(&key)
                .ok_or(Error::MemoryLimit)?
                .set_max(x, y, value)?
        };
        if !changed {
            return Ok(());
        }
        self.bounds = Some(match self.bounds {
            Some((left, top, right, bottom)) => {
                (left.min(x), top.min(y), right.max(x + 1), bottom.max(y + 1))
            }
            None => (x, y, x + 1, y + 1),
        });
        Ok(())
    }

    /// Borrows a raw tile; tiles wholly outside the document have background zero.
    pub(crate) fn tile(&self, tx: u32, ty: u32) -> TileView<'_> {
        if tx >= self.width.div_ceil(TILE_SIZE) || ty >= self.height.div_ceil(TILE_SIZE) {
            return TileView::Background(0);
        }
        match self.tiles.get(&(tx, ty)) {
            Some(tile) => TileView::Stored(&tile.values),
            None => TileView::Background(self.background),
        }
    }

    /// Iterates stored tile coordinates and bytes; order is unspecified.
    pub(crate) fn stored_tiles(&self) -> impl Iterator<Item = ((u32, u32), &[u8])> {
        self.tiles
            .iter()
            .map(|(&position, tile)| (position, tile.values.as_slice()))
    }

    /// Exact nonzero bounds, with exclusive right/bottom; a nonzero background is full-canvas.
    pub(crate) fn covered_bounds(&self) -> Option<(u32, u32, u32, u32)> {
        self.bounds
    }

    /// Whether every in-document pixel is zero, including the implicit background.
    pub(crate) fn is_empty(&self) -> bool {
        self.covered_bounds().is_none()
    }

    /// Stages missing tiles so a whole stroke stamp can reserve atomically before writing.
    pub(crate) fn reserve_tiles(&mut self) -> TileReservations<'_> {
        TileReservations {
            store: self,
            pending: HashMap::new(),
        }
    }

    /// Stored tile count retained for unchanged coverage-mask tests.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.tiles.len()
    }
}

/// Missing tiles held separately until every allocation for an edit has succeeded.
pub(crate) struct TileReservations<'a> {
    store: &'a mut MaskStore,
    pending: HashMap<(u32, u32), MaskTile>,
}

impl TileReservations<'_> {
    /// Reserves the pixel's tile; duplicates and out-of-document pixels are no-ops.
    pub(crate) fn reserve_pixel(&mut self, x: u32, y: u32) -> Result<(), Error> {
        if x >= self.store.width || y >= self.store.height {
            return Ok(());
        }
        let key = (x / TILE_SIZE, y / TILE_SIZE);
        if !self.store.tiles.contains_key(&key) && !self.pending.contains_key(&key) {
            self.pending
                .try_reserve(1)
                .map_err(|_| Error::MemoryLimit)?;
            self.pending
                .insert(key, MaskTile::new(self.store.background)?);
        }
        Ok(())
    }

    /// Publishes all reserved tiles, or leaves the mask unchanged on allocation failure.
    pub(crate) fn commit(self) -> Result<(), Error> {
        self.store
            .tiles
            .try_reserve(self.pending.len())
            .map_err(|_| Error::MemoryLimit)?;
        self.store.tiles.extend(self.pending);
        Ok(())
    }
}

fn pixel_index(x: u32, y: u32) -> usize {
    ((y % TILE_SIZE) * TILE_SIZE + x % TILE_SIZE) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implicit_background_and_document_edges_need_no_tiles() {
        for background in [0, 255] {
            let mut mask = MaskStore::new(131, 129, background);
            for x in [0, 127, 128, 130] {
                for y in [0, 127, 128] {
                    assert_eq!(mask.get(x, y), background);
                }
            }
            assert_eq!(mask.get(131, 0), 0);
            assert_eq!(mask.get(0, 129), 0);
            assert_eq!(mask.tile(1, 1).get(127, 127), background);
            assert_eq!(mask.tile(2, 2).get(0, 0), 0);
            assert_eq!(mask.tile(0, 0).get(128, 0), 0);
            assert_eq!(mask.stored_tiles().count(), 0);
            assert_eq!(mask.is_empty(), background == 0);
            assert_eq!(
                mask.covered_bounds(),
                (background != 0).then_some((0, 0, 131, 129))
            );
            mask.set_max(0, 0, background).unwrap();
            mask.set_max(131, 0, 255).unwrap();
            assert_eq!(mask.stored_tiles().count(), 0);
        }
        assert!(MaskStore::new(0, 5, 255).is_empty());
        assert!(MaskStore::new(5, 0, 255).is_empty());
    }

    #[test]
    fn maximum_combination_preserves_background_and_existing_coverage() {
        let mut mask = MaskStore::new(200, 200, 17);
        mask.set_max(4, 5, 100).unwrap();
        mask.set_max(4, 5, 50).unwrap();
        assert_eq!(mask.get(4, 5), 100);
        assert_eq!(mask.get(5, 5), 17);
        mask.set_max(4, 5, 201).unwrap();
        assert_eq!(mask.get(4, 5), 201);
        assert_eq!(mask.tile(0, 0).get(4, 5), 201);
        assert_eq!(mask.covered_bounds(), Some((0, 0, 200, 200)));
        assert_eq!(mask.stored_tiles().count(), 1);
        let mut full = MaskStore::new(200, 200, 255);
        full.set_max(4, 5, 254).unwrap();
        assert_eq!(full.get(4, 5), 255);
        assert_eq!(full.stored_tiles().count(), 0);
    }

    #[test]
    fn wide_mask_stores_one_tile_and_tracks_exact_pixel_bounds() {
        let mut mask = MaskStore::new(262_144, 7, 0);
        mask.set_max(262_143, 6, 93).unwrap();
        assert_eq!(mask.get(262_143, 6), 93);
        assert_eq!(mask.get(0, 6), 0);
        assert_eq!(mask.stored_tiles().count(), 1);
        assert_eq!(mask.covered_bounds(), Some((262_143, 6, 262_144, 7)));
        assert!(!mask.is_empty());
        let (position, values) = mask.stored_tiles().next().unwrap();
        assert_eq!(position, (2047, 0));
        assert_eq!(values.len(), (TILE_SIZE * TILE_SIZE) as usize);
        mask.set_max(3, 4, 1).unwrap();
        assert_eq!(mask.covered_bounds(), Some((3, 4, 262_144, 7)));
    }

    #[test]
    fn memory_failure_preserves_mask_and_releases_pending_reservations() {
        const CHILD: &str = "HOSHI_MASK_ATOMICITY_TEST";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "mask::tests::memory_failure_preserves_mask_and_releases_pending_reservations",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "{stdout}\n{stderr}");
            assert!(
                stdout.lines().any(|line| line.trim() == "running 1 test"),
                "{stdout}"
            );
            return;
        }
        let bytes = u64::from(TILE_SIZE * TILE_SIZE);
        crate::configure_memory_budget(bytes).unwrap();
        let mut mask = MaskStore::new(TILE_SIZE * 4, 4, 0);
        mask.set_max(0, 0, 50).unwrap();
        let bounds = mask.covered_bounds();
        assert_eq!(mask.set_max(TILE_SIZE, 0, 99), Err(Error::MemoryLimit));
        assert_eq!(mask.get(0, 0), 50);
        assert_eq!(mask.get(TILE_SIZE, 0), 0);
        assert_eq!(mask.covered_bounds(), bounds);
        assert_eq!(mask.stored_tiles().count(), 1);
        assert_eq!(crate::memory_usage(), bytes);

        crate::configure_memory_budget(bytes * 2).unwrap();
        {
            let mut pending = mask.reserve_tiles();
            pending.reserve_pixel(TILE_SIZE, 0).unwrap();
            assert_eq!(
                pending.reserve_pixel(TILE_SIZE * 2, 0),
                Err(Error::MemoryLimit)
            );
        }
        assert_eq!(mask.covered_bounds(), bounds);
        assert_eq!(mask.stored_tiles().count(), 1);
        assert_eq!(crate::memory_usage(), bytes);
        mask.set_max(TILE_SIZE, 0, 99).unwrap();
        assert_eq!(mask.get(TILE_SIZE, 0), 99);
        assert_eq!(mask.stored_tiles().count(), 2);
        drop(mask);
        assert_eq!(crate::memory_usage(), 0);
    }
}
