//! Validated brush styles and sparse, maximum-combined stroke coverage.

use crate::{Color, Error, MemoryReservation, TILE_SIZE, geometry::Point};
use std::collections::HashMap;

/// A finite circular brush width in document pixels and its edge sampling mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushTip {
    width: f64,
    antialias: bool,
}

impl BrushTip {
    /// Smallest supported diameter in document pixels.
    pub const MIN_WIDTH: f64 = 1.0;
    /// Largest supported diameter in document pixels.
    pub const MAX_WIDTH: f64 = 2000.0;
    /// A two-pixel circular tip with antialiasing enabled.
    pub const DEFAULT: Self = Self {
        width: 2.0,
        antialias: true,
    };

    /// Rejects nonfinite widths and widths outside the inclusive supported range.
    pub fn new(width: f64, antialias: bool) -> Result<Self, Error> {
        if !width.is_finite() || !(Self::MIN_WIDTH..=Self::MAX_WIDTH).contains(&width) {
            return Err(Error::InvalidStroke);
        }
        Ok(Self { width, antialias })
    }

    /// Returns the validated diameter in document pixels.
    pub const fn width(self) -> f64 {
        self.width
    }

    /// Whether capsule edges use fractional pixel-center coverage.
    pub const fn antialias(self) -> bool {
        self.antialias
    }
}

/// Paint uses straight-alpha source-over; erase preserves RGB until alpha is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeBlend {
    /// Paint this straight-alpha color with the supplied coverage.
    Paint(Color),
    /// Reduce destination alpha by the supplied coverage.
    Erase,
}

impl StrokeBlend {
    /// Zero coverage preserves the destination, including invisible RGB.
    pub(crate) fn apply(self, destination: Color, coverage: u8) -> Color {
        if coverage == 0 {
            return destination;
        }
        match self {
            Self::Paint(mut source) => {
                source.a = ((u32::from(source.a) * u32::from(coverage) + 127) / 255) as u8;
                if source.a == 0 {
                    return destination;
                }
                source.over(destination, 255)
            }
            Self::Erase => {
                let alpha =
                    ((u32::from(destination.a) * u32::from(255 - coverage) + 127) / 255) as u8;
                if alpha == 0 {
                    Color::TRANSPARENT
                } else {
                    Color {
                        a: alpha,
                        ..destination
                    }
                }
            }
        }
    }
}

/// Validated circular tip and blend operation for one stroke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    /// Diameter and sampling mode shared by every segment in the stroke.
    pub tip: BrushTip,
    /// Paint or erase operation applied to accumulated coverage.
    pub blend: StrokeBlend,
}

struct CoverageTile {
    values: Vec<u8>,
    _reservation: MemoryReservation,
}

impl CoverageTile {
    fn new() -> Result<Self, Error> {
        let count = (TILE_SIZE * TILE_SIZE) as usize;
        let reservation = MemoryReservation::new(count as u64)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::MemoryLimit)?;
        values.resize(count, 0);
        Ok(Self {
            values,
            _reservation: reservation,
        })
    }
}

/// Sparse coverage clipped to `width` by `height` document pixels. Tile buffers
/// own budget reservations; revisiting a pixel combines coverage with maximum.
pub(crate) struct CoverageMask {
    width: u32,
    height: u32,
    tiles: HashMap<(u32, u32), CoverageTile>,
}

impl CoverageMask {
    /// Starts empty without allocating any pixel buffers, including for zero sizes.
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            tiles: HashMap::new(),
        }
    }

    /// Stamps a finite capsule, clipping off-canvas endpoints. All new buffers
    /// are reserved before existing coverage is changed; failure is atomic.
    pub(crate) fn stamp_segment(
        &mut self,
        from: Point,
        to: Point,
        tip: BrushTip,
    ) -> Result<(), Error> {
        let segment = Segment::new(from, to, tip)?;
        let mut pending = HashMap::new();
        segment.visit(self.width, self.height, |x, y, _| {
            let key = (x / TILE_SIZE, y / TILE_SIZE);
            if !self.tiles.contains_key(&key) && !pending.contains_key(&key) {
                pending.try_reserve(1).map_err(|_| Error::MemoryLimit)?;
                pending.insert(key, CoverageTile::new()?);
            }
            Ok(())
        })?;
        self.tiles
            .try_reserve(pending.len())
            .map_err(|_| Error::MemoryLimit)?;
        self.tiles.extend(pending);
        segment.visit(self.width, self.height, |x, y, coverage| {
            if let Some(tile) = self.tiles.get_mut(&(x / TILE_SIZE, y / TILE_SIZE))
                && let Some(value) = tile.values.get_mut(pixel_index(x, y))
            {
                *value = (*value).max(coverage);
            }
            Ok(())
        })
    }

    /// Returns zero outside the document and in untouched tiles.
    pub(crate) fn coverage(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        self.tiles
            .get(&(x / TILE_SIZE, y / TILE_SIZE))
            .and_then(|tile| tile.values.get(pixel_index(x, y)))
            .copied()
            .unwrap_or(0)
    }

    /// Yields occupied tile rectangles as `(left, top, right, bottom)`, with
    /// exclusive right/bottom bounds clipped to the document; order is unspecified.
    pub(crate) fn covered_rects(&self) -> impl Iterator<Item = (u32, u32, u32, u32)> + '_ {
        self.tiles.keys().map(|&(tx, ty)| {
            let x = tx * TILE_SIZE;
            let y = ty * TILE_SIZE;
            (
                x,
                y,
                x.saturating_add(TILE_SIZE).min(self.width),
                y.saturating_add(TILE_SIZE).min(self.height),
            )
        })
    }

    /// Whether every document pixel still has zero coverage.
    pub(crate) fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

fn pixel_index(x: u32, y: u32) -> usize {
    ((y % TILE_SIZE) * TILE_SIZE + x % TILE_SIZE) as usize
}

struct Segment {
    from: Point,
    to: Point,
    unit: Point,
    offset: f64,
    tip: BrushTip,
}

impl Segment {
    fn new(from: Point, to: Point, tip: BrushTip) -> Result<Self, Error> {
        if ![from.x, from.y, to.x, to.y].into_iter().all(f64::is_finite) {
            return Err(Error::InvalidStroke);
        }
        // A capsule is symmetric, so put the endpoints in a canonical order.
        // Reversed segments then run identical arithmetic and stamp identical
        // coverage, keeping boundary pixels deterministic.
        let (from, to) = if from
            .x
            .total_cmp(&to.x)
            .then(from.y.total_cmp(&to.y))
            .is_gt()
        {
            (to, from)
        } else {
            (from, to)
        };
        // Normalize before subtraction so opposing finite endpoints cannot
        // overflow their direction vector (for example -f64::MAX to MAX).
        let scale = from
            .x
            .abs()
            .max(from.y.abs())
            .max(to.x.abs())
            .max(to.y.abs())
            .max(1.0);
        let dx = to.x / scale - from.x / scale;
        let dy = to.y / scale - from.y / scale;
        let length = dx.hypot(dy);
        let unit = if length == 0.0 {
            Point::default()
        } else {
            Point::new(dx / length, dy / length)
        };
        let offset = from.x * unit.y - from.y * unit.x;
        Ok(Self {
            from,
            to,
            unit,
            offset,
            tip,
        })
    }

    fn distance(&self, point: Point) -> f64 {
        let from = Point::new(point.x - self.from.x, point.y - self.from.y);
        let to = Point::new(point.x - self.to.x, point.y - self.to.y);
        if self.unit == Point::default() || from.x * self.unit.x + from.y * self.unit.y <= 0.0 {
            from.x.hypot(from.y)
        } else if to.x * self.unit.x + to.y * self.unit.y >= 0.0 {
            to.x.hypot(to.y)
        } else {
            (point.x * self.unit.y - point.y * self.unit.x - self.offset).abs()
        }
    }

    fn coverage(&self, x: u32, y: u32) -> u8 {
        let distance = self.distance(Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5));
        let radius = self.tip.width / 2.0;
        if self.tip.antialias {
            ((radius + 0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8
        } else {
            u8::from(distance <= radius) * 255
        }
    }

    fn visit(
        &self,
        width: u32,
        height: u32,
        mut visitor: impl FnMut(u32, u32, u8) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let reach = self.tip.width / 2.0 + if self.tip.antialias { 0.5 } else { 0.0 };
        let first = (self.from.y.min(self.to.y) - reach)
            .floor()
            .clamp(0.0, f64::from(height)) as u32;
        let end = (self.from.y.max(self.to.y) + reach)
            .ceil()
            .clamp(0.0, f64::from(height)) as u32;
        for y in first..end {
            let (left, right) = self.row_bounds(f64::from(y) + 0.5, reach);
            let start = left.floor().clamp(0.0, f64::from(width)) as u32;
            let end = right.ceil().clamp(0.0, f64::from(width)) as u32;
            for x in start..end {
                let coverage = self.coverage(x, y);
                if coverage != 0 {
                    visitor(x, y, coverage)?;
                }
            }
        }
        Ok(())
    }

    fn row_bounds(&self, y: f64, reach: f64) -> (f64, f64) {
        if self.from.y == self.to.y {
            return (
                self.from.x.min(self.to.x) - reach,
                self.from.x.max(self.to.x) + reach,
            );
        }
        let scale = self.from.y.abs().max(self.to.y.abs()).max(1.0);
        let dy = self.to.y / scale - self.from.y / scale;
        let at = |edge: f64| {
            let t = ((edge / scale - self.from.y / scale) / dy).clamp(0.0, 1.0);
            self.from.x * (1.0 - t) + self.to.x * t
        };
        let a = at(y - reach);
        let b = at(y + reach);
        // Conservatively include interpolation roundoff for distant endpoints.
        let padding = reach + 8.0 * f64::EPSILON * self.from.x.abs().max(self.to.x.abs());
        (a.min(b) - padding, a.max(b) + padding)
    }
}

#[cfg(test)]
mod tests;
