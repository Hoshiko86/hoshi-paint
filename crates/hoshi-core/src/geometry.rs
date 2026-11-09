//! Shape-independent editing geometry in document coordinates.
//!
//! A frame describes where a shape's local bounds appear on the canvas. It does
//! not know whether those bounds contain a rectangle, ellipse, line or polygon.
use crate::{MAX_DIMENSION, Shape};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
    pub fn translated(self, dx: f64, dy: f64) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}
impl ResizeHandle {
    pub const ALL: [Self; 8] = [
        Self::TopLeft,
        Self::Top,
        Self::TopRight,
        Self::Right,
        Self::BottomRight,
        Self::Bottom,
        Self::BottomLeft,
        Self::Left,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn signs(self) -> (f64, f64) {
        match self {
            Self::TopLeft => (-1., -1.),
            Self::Top => (0., -1.),
            Self::TopRight => (1., -1.),
            Self::Right => (1., 0.),
            Self::BottomRight => (1., 1.),
            Self::Bottom => (0., 1.),
            Self::BottomLeft => (-1., 1.),
            Self::Left => (-1., 0.),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResizeConstraints {
    pub preserve_aspect: bool,
    pub from_center: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub center: Point,
    pub width: f64,
    pub height: f64,
    pub angle_degrees: f64,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
}
impl Frame {
    pub fn from_shape(shape: Shape) -> Self {
        Self {
            center: Point::new(
                f64::from(shape.x) + f64::from(shape.width) / 2.,
                f64::from(shape.y) + f64::from(shape.height) / 2.,
            ),
            width: f64::from(shape.width),
            height: f64::from(shape.height),
            angle_degrees: f64::from(shape.rotation_degrees),
            flip_horizontal: shape.flip_horizontal,
            flip_vertical: shape.flip_vertical,
        }
    }
    pub fn axes(self) -> (Point, Point) {
        let (sin, cos) = rotation_axes(self.angle_degrees);
        (Point::new(cos, sin), Point::new(-sin, cos))
    }
    /// Rotate a vector from frame axes into document axes, without mirroring.
    pub fn vector_to_document(self, local: Point) -> Point {
        let (u, v) = self.axes();
        Point::new(u.x * local.x + v.x * local.y, u.y * local.x + v.y * local.y)
    }
    pub fn vector_to_local(self, world: Point) -> Point {
        let (u, v) = self.axes();
        Point::new(world.x * u.x + world.y * u.y, world.x * v.x + world.y * v.y)
    }
    pub fn local_to_document(self, local: Point) -> Point {
        let local = Point::new(
            if self.flip_horizontal {
                -local.x
            } else {
                local.x
            },
            if self.flip_vertical {
                -local.y
            } else {
                local.y
            },
        );
        let p = self.vector_to_document(local);
        self.center.translated(p.x, p.y)
    }
    pub fn handle(self, handle: ResizeHandle) -> Point {
        let (x, y) = handle.signs();
        let p = self.vector_to_document(Point::new(x * self.width / 2., y * self.height / 2.));
        self.center.translated(p.x, p.y)
    }
    pub fn contains(self, point: Point) -> bool {
        let p = self.vector_to_local(Point::new(point.x - self.center.x, point.y - self.center.y));
        p.x.abs() <= self.width / 2. && p.y.abs() <= self.height / 2.
    }
    pub fn rotated_about(self, pivot: Point, angle_degrees: f64) -> Self {
        let delta = (angle_degrees - self.angle_degrees).to_radians();
        let (sin, cos) = delta.sin_cos();
        let dx = self.center.x - pivot.x;
        let dy = self.center.y - pivot.y;
        Self {
            center: Point::new(pivot.x + cos * dx - sin * dy, pivot.y + sin * dx + cos * dy),
            angle_degrees,
            ..self
        }
    }
    /// Resize from the immutable drag-start frame. Re-evaluating this function
    /// when modifiers change avoids accumulating rounding or pointer drift.
    pub fn resized(
        self,
        handle: ResizeHandle,
        grab: Point,
        pointer: Point,
        constraints: ResizeConstraints,
    ) -> Self {
        let (sx, sy) = handle.signs();
        let moving = self.handle(handle);
        let fixed = if constraints.from_center {
            self.center
        } else {
            self.handle(ResizeHandle::ALL[(handle.index() + 4) % 8])
        };
        let delta = self.vector_to_local(Point::new(
            pointer.x + moving.x - grab.x - fixed.x,
            pointer.y + moving.y - grab.y - fixed.y,
        ));
        let factor = if constraints.from_center { 2. } else { 1. };
        let desired_w = if sx == 0. {
            self.width
        } else {
            sx * delta.x * factor
        };
        let desired_h = if sy == 0. {
            self.height
        } else {
            sy * delta.y * factor
        };
        let (width, height) = if constraints.preserve_aspect {
            let scale = if sx == 0. {
                desired_h.abs() / self.height
            } else if sy == 0. {
                desired_w.abs() / self.width
            } else {
                (desired_w.abs() * self.width + desired_h.abs() * self.height)
                    / (self.width * self.width + self.height * self.height)
            };
            let scale = scale.clamp(
                1. / self.width.min(self.height),
                f64::from(MAX_DIMENSION) / self.width.max(self.height),
            );
            (
                (self.width * scale).round().max(1.),
                (self.height * scale).round().max(1.),
            )
        } else {
            (
                desired_w.abs().round().clamp(1., f64::from(MAX_DIMENSION)),
                desired_h.abs().round().clamp(1., f64::from(MAX_DIMENSION)),
            )
        };
        let cross_x = sx != 0. && desired_w < 0.;
        let cross_y = sy != 0. && desired_h < 0.;
        let center = if constraints.from_center {
            self.center
        } else {
            let offset = self.vector_to_document(Point::new(
                sx * if cross_x { -width } else { width } / 2.,
                sy * if cross_y { -height } else { height } / 2.,
            ));
            fixed.translated(offset.x, offset.y)
        };
        Self {
            center,
            width,
            height,
            flip_horizontal: self.flip_horizontal ^ cross_x,
            flip_vertical: self.flip_vertical ^ cross_y,
            ..self
        }
    }
    /// Quantization belongs at the raster-model boundary, not in gesture math.
    pub fn apply_to(self, mut shape: Shape) -> Shape {
        shape.width = self.width.round().clamp(1., f64::from(MAX_DIMENSION)) as u32;
        shape.height = self.height.round().clamp(1., f64::from(MAX_DIMENSION)) as u32;
        shape.x = (self.center.x - f64::from(shape.width) / 2.).round() as i32;
        shape.y = (self.center.y - f64::from(shape.height) / 2.).round() as i32;
        shape.rotation_degrees = (self.angle_degrees.round() as i32).rem_euclid(360);
        shape.flip_horizontal = self.flip_horizontal;
        shape.flip_vertical = self.flip_vertical;
        shape
    }
}

/// Cached inverse transform for rasterizers and hit tests that sample many points.
#[derive(Clone, Copy)]
pub struct PreparedTransform {
    frame: Frame,
    sin: f64,
    cos: f64,
}
fn rotation_axes(angle: f64) -> (f64, f64) {
    match angle.rem_euclid(360.) {
        0. => (0., 1.),
        90. => (1., 0.),
        180. => (0., -1.),
        270. => (-1., 0.),
        _ => angle.to_radians().sin_cos(),
    }
}
impl Frame {
    pub fn prepare(self) -> PreparedTransform {
        let (sin, cos) = rotation_axes(self.angle_degrees);
        PreparedTransform {
            frame: self,
            sin,
            cos,
        }
    }
}
impl PreparedTransform {
    /// Return a point relative to the untransformed shape's center.
    pub fn to_local(self, point: Point) -> Point {
        let dx = point.x - self.frame.center.x;
        let dy = point.y - self.frame.center.y;
        let x = self.cos * dx + self.sin * dy;
        let y = -self.sin * dx + self.cos * dy;
        Point::new(
            if self.frame.flip_horizontal { -x } else { x },
            if self.frame.flip_vertical { -y } else { y },
        )
    }
}
