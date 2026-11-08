//! Sparse copy-on-write tiles. Uniform canvas regions allocate no pixel buffers.
use crate::{Color, Error, MemoryReservation};
use std::{collections::HashMap, ops::Index, sync::Arc};
pub const TILE_SIZE: u32 = 128;
#[derive(Debug)]
struct Tile {
    pixels: Vec<Color>,
    width: u32,
    height: u32,
    _reservation: MemoryReservation,
}
impl Tile {
    fn filled(color: Color, width: u32, height: u32) -> Result<Self, Error> {
        let count = width as usize * height as usize;
        let reservation = MemoryReservation::new(count as u64 * 4)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| Error::MemoryLimit)?;
        pixels.resize(count, color);
        Ok(Self {
            pixels,
            width,
            height,
            _reservation: reservation,
        })
    }
    fn copied(&self) -> Result<Self, Error> {
        let mut tile = Self::filled(Color::TRANSPARENT, self.width, self.height)?;
        tile.pixels.copy_from_slice(&self.pixels);
        Ok(tile)
    }
}
#[derive(Clone, Debug)]
pub struct PixelStore {
    width: u32,
    height: u32,
    background: Color,
    tiles: HashMap<(u32, u32), Arc<Tile>>,
}
impl PixelStore {
    pub fn new(width: u32, height: u32, background: Color) -> Self {
        Self {
            width,
            height,
            background,
            tiles: HashMap::new(),
        }
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn len(&self) -> usize {
        self.width as usize * self.height as usize
    }
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
    pub fn background(&self) -> Color {
        self.background
    }
    pub fn is_uniform(&self) -> bool {
        self.tiles.is_empty()
    }
    pub fn tile_positions(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.tiles.keys().copied()
    }
    pub fn allocated_rects(&self) -> impl Iterator<Item = (u32, u32, u32, u32)> + '_ {
        self.tile_positions().map(|(tx, ty)| {
            let x = tx * TILE_SIZE;
            let y = ty * TILE_SIZE;
            (
                x,
                y,
                (x + TILE_SIZE).min(self.width),
                (y + TILE_SIZE).min(self.height),
            )
        })
    }
    pub fn allocated_bytes(&self) -> u64 {
        self.tiles
            .values()
            .map(|tile| tile.pixels.len() as u64 * 4)
            .sum()
    }
    pub fn get(&self, x: u32, y: u32) -> Color {
        *self.pixel_ref(x, y)
    }
    fn pixel_ref(&self, x: u32, y: u32) -> &Color {
        assert!(x < self.width && y < self.height);
        self.tiles
            .get(&(x / TILE_SIZE, y / TILE_SIZE))
            .map(|t| &t.pixels[((y % TILE_SIZE) * t.width + x % TILE_SIZE) as usize])
            .unwrap_or(&self.background)
    }
    pub fn set(&mut self, x: u32, y: u32, color: Color) -> Result<(), Error> {
        if x >= self.width || y >= self.height {
            return Err(Error::InvalidDimensions);
        }
        if self.get(x, y) == color {
            return Ok(());
        }
        let key = (x / TILE_SIZE, y / TILE_SIZE);
        if !self.tiles.contains_key(&key) {
            self.tiles.try_reserve(1).map_err(|_| Error::MemoryLimit)?;
            self.tiles.insert(
                key,
                Arc::new(Tile::filled(
                    self.background,
                    (self.width - key.0 * TILE_SIZE).min(TILE_SIZE),
                    (self.height - key.1 * TILE_SIZE).min(TILE_SIZE),
                )?),
            );
        }
        let tile = self.tiles.get_mut(&key).expect("inserted tile");
        if Arc::get_mut(tile).is_none() {
            *tile = Arc::new(tile.copied()?);
        }
        let tile = Arc::get_mut(tile).expect("unique tile");
        tile.pixels[((y % TILE_SIZE) * tile.width + x % TILE_SIZE) as usize] = color;
        Ok(())
    }
    pub fn from_vec(width: u32, height: u32, pixels: Vec<Color>) -> Result<Self, Error> {
        if pixels.len() != width as usize * height as usize || pixels.is_empty() {
            return Err(Error::InvalidDimensions);
        }
        // An isolated corner pixel must not turn a nearly uniform import into
        // a dense canvas. A majority color, when present, wins in constant space.
        let mut background = pixels[0];
        let mut votes = 0usize;
        for &color in &pixels {
            if votes == 0 {
                background = color;
                votes = 1
            } else if color == background {
                votes += 1
            } else {
                votes -= 1
            }
        }
        let mut result = Self::new(width, height, background);
        for (index, color) in pixels.into_iter().enumerate() {
            if color != result.background {
                result.set(
                    (index % width as usize) as u32,
                    (index / width as usize) as u32,
                    color,
                )?;
            }
        }
        Ok(result)
    }
    pub fn iter(&self) -> impl Iterator<Item = &Color> {
        (0..self.len()).map(|i| &self[i])
    }
    /// Compatibility for callers needing a small dense buffer. Large application
    /// paths stream rows or sample the viewport instead.
    pub fn try_to_vec(&self) -> Result<Vec<Color>, Error> {
        let _memory = MemoryReservation::new(self.len() as u64 * 4)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(self.len())
            .map_err(|_| Error::MemoryLimit)?;
        output.extend(self.iter().copied());
        Ok(output)
    }
    pub fn to_vec(&self) -> Vec<Color> {
        self.try_to_vec()
            .expect("dense pixels exceed memory budget; stream rows instead")
    }
}
impl Index<usize> for PixelStore {
    type Output = Color;
    fn index(&self, index: usize) -> &Color {
        assert!(index < self.len());
        self.pixel_ref(
            (index % self.width as usize) as u32,
            (index / self.width as usize) as u32,
        )
    }
}
impl PartialEq for PixelStore {
    fn eq(&self, other: &Self) -> bool {
        if (self.width, self.height) != (other.width, other.height) {
            return false;
        }
        if self.background != other.background {
            let total_tiles = u64::from(self.width.div_ceil(TILE_SIZE))
                * u64::from(self.height.div_ceil(TILE_SIZE));
            if ((self.tiles.len() + other.tiles.len()) as u64) < total_tiles {
                return false;
            }
            let covered: std::collections::HashSet<_> = self
                .tile_positions()
                .chain(other.tile_positions())
                .collect();
            if (covered.len() as u64) < total_tiles {
                return false;
            }
        }
        for (tx, ty) in self.tile_positions().chain(other.tile_positions()) {
            let a = self.tiles.get(&(tx, ty));
            let b = other.tiles.get(&(tx, ty));
            if let (Some(a), Some(b)) = (a, b)
                && Arc::ptr_eq(a, b)
            {
                continue;
            }
            for y in ty * TILE_SIZE..((ty + 1) * TILE_SIZE).min(self.height) {
                for x in tx * TILE_SIZE..((tx + 1) * TILE_SIZE).min(self.width) {
                    if self.get(x, y) != other.get(x, y) {
                        return false;
                    }
                }
            }
        }
        true
    }
}
impl Eq for PixelStore {}
impl PartialEq<Vec<Color>> for PixelStore {
    fn eq(&self, other: &Vec<Color>) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == b)
    }
}
impl PartialEq<Vec<Color>> for &PixelStore {
    fn eq(&self, other: &Vec<Color>) -> bool {
        *self == other
    }
}
impl<'a> IntoIterator for &'a PixelStore {
    type Item = &'a Color;
    type IntoIter = std::iter::Map<std::ops::Range<usize>, Box<dyn FnMut(usize) -> &'a Color + 'a>>;
    fn into_iter(self) -> Self::IntoIter {
        (0..self.len()).map(Box::new(move |i| &self[i]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edge_tiles_copy_on_write_and_background_equality_preserve_pixels() {
        let black = Color::rgba(0, 0, 0, 255);
        let white = Color::rgba(255, 255, 255, 255);
        let red = Color::rgba(255, 0, 0, 255);
        let mut a = PixelStore::new(384, 128, black);
        let mut b = PixelStore::new(384, 128, white);
        for y in 0..128 {
            for x in 0..256 {
                a.set(x, y, red).unwrap();
                b.set(x, y, red).unwrap();
            }
        }
        assert_ne!(
            a, b,
            "overlapping stored tiles do not cover differing implicit backgrounds"
        );
        let mut thin = PixelStore::new(1, crate::MAX_DIMENSION, white);
        thin.set(0, crate::MAX_DIMENSION - 1, red).unwrap();
        assert_eq!(thin.allocated_bytes(), 128 * 4);
        let before = thin.clone();
        thin.set(0, crate::MAX_DIMENSION - 1, black).unwrap();
        assert_eq!(before.get(0, crate::MAX_DIMENSION - 1), red);
        assert_eq!(thin.get(0, crate::MAX_DIMENSION - 1), black);
        let mut colors = vec![white; 1024];
        colors[0] = black;
        let imported = PixelStore::from_vec(1024, 1, colors).unwrap();
        assert_eq!(imported.background(), white);
        assert_eq!(imported.allocated_bytes(), 128 * 4);
    }
}
