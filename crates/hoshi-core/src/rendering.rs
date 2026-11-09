//! Shared shape paints and pixel coverage. Geometry lives in local coordinates;
//! transforms, antialiasing and paint sampling are independent of primitive kind.
use crate::{Color, Error, MAX_DIMENSION};
mod paint;
mod raster;
pub use paint::{FillPattern, LineStyle};
pub(crate) use raster::RasterShape;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeKind {
    Rectangle,
    Ellipse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShapeStyle {
    Fill(Color),
    Outline {
        color: Color,
        width: f64,
    },
    Both {
        fill: Color,
        outline: Color,
        width: f64,
    },
}

/// Integer local bounds; outline is drawn inside the bounds. Rotation is about
/// the bounds' center, clockwise in screen coordinates, at whole-degree steps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub kind: ShapeKind,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub rotation_degrees: i32,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub style: ShapeStyle,
    pub rendering: RenderOptions,
}

impl Shape {
    pub fn new(
        kind: ShapeKind,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        style: ShapeStyle,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            width,
            height,
            rotation_degrees: 0,
            flip_horizontal: false,
            flip_vertical: false,
            style,
            rendering: RenderOptions::default(),
        }
    }

    pub(crate) fn validate(self) -> Result<(), Error> {
        if self.width == 0
            || self.height == 0
            || self.width > MAX_DIMENSION
            || self.height > MAX_DIMENSION
        {
            return Err(Error::InvalidShape);
        }
        let width = match self.style {
            ShapeStyle::Fill(_) => return Ok(()),
            ShapeStyle::Outline { width, .. } | ShapeStyle::Both { width, .. } => width,
        };
        if !width.is_finite() || !(0.01..=65_536.).contains(&width) {
            Err(Error::InvalidShape)
        } else {
            Ok(())
        }
    }

    fn contains(self, x: f64, y: f64, inset: f64) -> bool {
        let width = f64::from(self.width) - 2.0 * inset;
        let height = f64::from(self.height) - 2.0 * inset;
        if width <= 0.0 || height <= 0.0 {
            return false;
        }
        let left = f64::from(self.x) + inset;
        let top = f64::from(self.y) + inset;
        if x < left || y < top || x >= left + width || y >= top + height {
            return false;
        }
        match self.kind {
            ShapeKind::Rectangle => true,
            ShapeKind::Ellipse => {
                let dx = (x - left - width / 2.0) / (width / 2.0);
                let dy = (y - top - height / 2.0) / (height / 2.0);
                dx * dx + dy * dy <= 1.0
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderOptions {
    pub antialias: bool,
    pub line_style: LineStyle,
    pub fill_pattern: FillPattern,
    /// The other workspace color, used in the background of a patterned fill.
    pub pattern_color: Color,
}
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            antialias: true,
            line_style: LineStyle::Solid,
            fill_pattern: FillPattern::Solid,
            pattern_color: Color::rgba(255, 255, 255, 255),
        }
    }
}
