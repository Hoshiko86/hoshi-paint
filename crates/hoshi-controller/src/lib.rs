//! Logical-coordinate input boundary for the disposable desktop integration spike.
//!
//! UI adapters pass logical window coordinates, never physical device pixels.
//! This controller owns document mutations; views receive immutable state and
//! an RGBA image. It has no windowing, rendering-framework, or filesystem dependency.

pub mod color;

pub use hoshi_core::Color;
use hoshi_core::{Document, Error, Shape, ShapeKind, ShapeStyle};

pub const MAX_LAYER_NAME_CHARS: usize = 128;
const MAX_VIEW_OFFSET: f64 = 1_000_000_000.0;
const HANDLE_RADIUS: f64 = 8.0;

/// Control points in logical viewport pixels: corners/edges clockwise from
/// top-left (0..=7), followed by the rotation point (8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandlePoint {
    pub x: f64,
    pub y: f64,
    pub kind: i32,
}

const HANDLE_SIGNS: [(f64, f64); 8] = [
    (-1.0, -1.0),
    (0.0, -1.0),
    (1.0, -1.0),
    (1.0, 0.0),
    (1.0, 1.0),
    (0.0, 1.0),
    (-1.0, 1.0),
    (-1.0, 0.0),
];

fn axes(shape: Shape) -> ((f64, f64), (f64, f64)) {
    let (sin, cos) = f64::from(shape.rotation_degrees.rem_euclid(360))
        .to_radians()
        .sin_cos();
    ((cos, sin), (-sin, cos))
}

fn center(shape: Shape) -> (f64, f64) {
    (
        f64::from(shape.x) + f64::from(shape.width) / 2.0,
        f64::from(shape.y) + f64::from(shape.height) / 2.0,
    )
}

fn control_point(shape: Shape, kind: usize) -> (f64, f64) {
    let (sx, sy) = HANDLE_SIGNS[kind];
    let (u, v) = axes(shape);
    let c = center(shape);
    (
        c.0 + sx * f64::from(shape.width) / 2.0 * u.0 + sy * f64::from(shape.height) / 2.0 * v.0,
        c.1 + sx * f64::from(shape.width) / 2.0 * u.1 + sy * f64::from(shape.height) / 2.0 * v.1,
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub zoom: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl Viewport {
    pub fn to_document(self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.offset_x) / self.zoom,
            (y - self.offset_y) / self.zoom,
        )
    }

    pub fn to_logical(self, x: f64, y: f64) -> (f64, f64) {
        (x * self.zoom + self.offset_x, y * self.zoom + self.offset_y)
    }

    /// For adapters receiving physical pixels. Slint pointer positions are
    /// already logical, so Slint adapters must not apply this conversion again.
    pub fn physical_to_document(self, x: f64, y: f64, scale_factor: f64) -> Option<(f64, f64)> {
        if !x.is_finite() || !y.is_finite() || !scale_factor.is_finite() || scale_factor <= 0.0 {
            return None;
        }
        let point = self.to_document(x / scale_factor, y / scale_factor);
        (point.0.is_finite() && point.1.is_finite()).then_some(point)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Rectangle,
    MoveShape,
    Pan,
    Eyedropper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerError {
    InvalidInput,
    InvalidLayerName,
    NoActiveShape,
    Core(Error),
}

impl From<Error> for ControllerError {
    fn from(value: Error) -> Self {
        Self::Core(value)
    }
}

#[derive(Clone, Copy)]
enum Gesture {
    Sample,
    Rectangle {
        anchor: (f64, f64),
    },
    Move {
        anchor: (f64, f64),
        original: Shape,
    },
    Pan {
        last: (f64, f64),
    },
    Resize {
        original: Shape,
        kind: usize,
        pointer_anchor: (f64, f64),
    },
    Rotate {
        original: Shape,
        anchor_angle: f64,
    },
}

pub struct Controller {
    document: Document,
    viewport: Viewport,
    tool: Tool,
    gesture: Option<Gesture>,
    primary_color: Color,
    aspect_lock: bool,
    last_pointer: Option<(f64, f64)>,
}

impl Default for Controller {
    fn default() -> Self {
        Self::new(512, 512).expect("fixed spike dimensions are valid")
    }
}

impl Controller {
    pub fn new(width: u32, height: u32) -> Result<Self, Error> {
        Ok(Self {
            document: Document::new(width, height)?,
            viewport: Viewport::default(),
            tool: Tool::Rectangle,
            gesture: None,
            primary_color: Color::rgba(72, 136, 240, 255),
            aspect_lock: false,
            last_pointer: None,
        })
    }

    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }
    pub fn tool(&self) -> Tool {
        self.tool
    }
    pub fn primary_color(&self) -> Color {
        self.primary_color
    }
    pub fn is_dragging(&self) -> bool {
        self.gesture.is_some()
    }

    pub fn aspect_lock(&self) -> bool {
        self.aspect_lock
    }

    /// Shift can be pressed/released during a corner drag, without moving again.
    pub fn set_aspect_lock(&mut self, pressed: bool) -> Result<(), ControllerError> {
        if self.aspect_lock == pressed {
            return Ok(());
        }
        self.aspect_lock = pressed;
        if matches!(self.gesture, Some(Gesture::Resize { .. }))
            && let Some((x, y)) = self.last_pointer
        {
            self.pointer_move(x, y)?;
        }
        Ok(())
    }

    /// Handle dimensions stay constant in logical pixels as the canvas zooms.
    pub fn handle_positions(&self) -> Vec<HandlePoint> {
        let Some(shape) = self.document.active_shape() else {
            return Vec::new();
        };
        let mut points: Vec<_> = (0..8)
            .map(|kind| {
                let p = control_point(shape, kind);
                let (x, y) = self.viewport.to_logical(p.0, p.1);
                HandlePoint {
                    x,
                    y,
                    kind: kind as i32,
                }
            })
            .collect();
        let (_, v) = axes(shape);
        let top = points[1];
        points.push(HandlePoint {
            x: top.x - v.0 * 28.0,
            y: top.y - v.1 * 28.0,
            kind: 8,
        });
        points
    }

    /// MoveShape and Pan are temporary interaction modes that preserve the active
    /// shape. Rectangle and Eyedropper commit it before starting another action.
    pub fn set_tool(&mut self, tool: Tool) -> Result<(), ControllerError> {
        if tool != self.tool {
            if matches!(tool, Tool::Rectangle | Tool::Eyedropper) {
                self.commit()?;
            }
            self.gesture = None;
            self.tool = tool;
        }
        Ok(())
    }

    fn document_point(&self, x: f64, y: f64) -> Result<(f64, f64), ControllerError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(ControllerError::InvalidInput);
        }
        let point = self.viewport.to_document(x, y);
        if !point.0.is_finite() || !point.1.is_finite() {
            return Err(ControllerError::InvalidInput);
        }
        Ok(point)
    }

    fn clip_point(&self, point: (f64, f64)) -> (f64, f64) {
        (
            point.0.clamp(0.0, f64::from(self.document.width())),
            point.1.clamp(0.0, f64::from(self.document.height())),
        )
    }

    pub fn pointer_down(&mut self, x: f64, y: f64) -> Result<(), ControllerError> {
        let point = self.document_point(x, y)?;
        self.last_pointer = Some((x, y));
        if matches!(self.tool, Tool::Rectangle | Tool::MoveShape)
            && let Some(handle) = self
                .handle_positions()
                .into_iter()
                .filter(|h| (h.x - x).hypot(h.y - y) <= HANDLE_RADIUS)
                .min_by(|a, b| {
                    (a.x - x)
                        .hypot(a.y - y)
                        .total_cmp(&(b.x - x).hypot(b.y - y))
                })
        {
            let original = self
                .document
                .active_shape()
                .expect("handles require an active shape");
            self.gesture = Some(if handle.kind == 8 {
                let c = center(original);
                Gesture::Rotate {
                    original,
                    anchor_angle: (point.1 - c.1).atan2(point.0 - c.0),
                }
            } else {
                Gesture::Resize {
                    original,
                    kind: handle.kind as usize,
                    pointer_anchor: point,
                }
            });
            return Ok(());
        }
        match self.tool {
            Tool::Rectangle => {
                // Clicking the surrounding workspace must not paint an edge
                // pixel or commit an existing preview.
                if point.0 < 0.0
                    || point.1 < 0.0
                    || point.0 >= f64::from(self.document.width())
                    || point.1 >= f64::from(self.document.height())
                {
                    self.gesture = None;
                    return Ok(());
                }
                self.commit()?;
                let point = self.clip_point(point);
                self.gesture = Some(Gesture::Rectangle { anchor: point });
            }
            Tool::MoveShape => {
                let original = self
                    .document
                    .active_shape()
                    .ok_or(ControllerError::NoActiveShape)?;
                self.gesture = Some(Gesture::Move {
                    anchor: point,
                    original,
                });
            }
            Tool::Pan => self.gesture = Some(Gesture::Pan { last: (x, y) }),
            Tool::Eyedropper => {
                self.sample_color(point);
                self.gesture = Some(Gesture::Sample);
            }
        }
        Ok(())
    }

    fn sample_color(&mut self, point: (f64, f64)) {
        if point.0 >= 0.0
            && point.1 >= 0.0
            && point.0 < f64::from(self.document.width())
            && point.1 < f64::from(self.document.height())
        {
            let index = point.1.floor() as usize * self.document.width() as usize
                + point.0.floor() as usize;
            self.primary_color = self.document.composite(false)[index];
        }
    }

    fn rectangle(&self, anchor: (f64, f64), point: (f64, f64)) -> Shape {
        // Bounds stay inside this small document. The gesture handler only calls
        // this for a nonzero-area drag; a click alone creates no shape.
        let left = anchor
            .0
            .min(point.0)
            .floor()
            .min(f64::from(self.document.width() - 1));
        let top = anchor
            .1
            .min(point.1)
            .floor()
            .min(f64::from(self.document.height() - 1));
        let right = anchor.0.max(point.0).ceil();
        let bottom = anchor.1.max(point.1).ceil();
        Shape {
            kind: ShapeKind::Rectangle,
            x: left as i32,
            y: top as i32,
            width: ((right - left) as u32).max(1),
            height: ((bottom - top) as u32).max(1),
            rotation_degrees: 0,
            style: ShapeStyle::Fill(self.primary_color),
        }
    }

    pub fn pointer_move(&mut self, x: f64, y: f64) -> Result<(), ControllerError> {
        let point = self.document_point(x, y)?;
        self.last_pointer = Some((x, y));
        match self.gesture {
            Some(Gesture::Sample) => self.sample_color(point),
            Some(Gesture::Rectangle { anchor }) => {
                let point = self.clip_point(point);
                if point.0 == anchor.0 || point.1 == anchor.1 {
                    self.document.cancel_shape();
                } else {
                    let shape = self.rectangle(anchor, point);
                    if self.document.active_shape().is_some() {
                        self.document.adjust_shape(shape)?;
                    } else {
                        self.document.begin_shape(shape)?;
                    }
                }
            }
            Some(Gesture::Move {
                anchor,
                mut original,
            }) => {
                original.x = original
                    .x
                    .saturating_add((point.0 - anchor.0).round() as i32);
                original.y = original
                    .y
                    .saturating_add((point.1 - anchor.1).round() as i32);
                self.document.adjust_shape(original)?;
            }
            Some(Gesture::Pan { last }) => {
                self.pan(x - last.0, y - last.1)?;
                self.gesture = Some(Gesture::Pan { last: (x, y) });
            }
            Some(Gesture::Resize {
                original,
                kind,
                pointer_anchor,
            }) => {
                let (sx, sy) = HANDLE_SIGNS[kind];
                let (u, v) = axes(original);
                let c = center(original);
                let fixed = (
                    c.0 - sx * f64::from(original.width) / 2.0 * u.0
                        - sy * f64::from(original.height) / 2.0 * v.0,
                    c.1 - sx * f64::from(original.width) / 2.0 * u.1
                        - sy * f64::from(original.height) / 2.0 * v.1,
                );
                // Preserve the offset at which the user grabbed the handle.
                let handle = control_point(original, kind);
                let delta = (
                    point.0 + handle.0 - pointer_anchor.0 - fixed.0,
                    point.1 + handle.1 - pointer_anchor.1 - fixed.1,
                );
                let mut width = if sx == 0.0 {
                    original.width
                } else {
                    (sx * (delta.0 * u.0 + delta.1 * u.1))
                        .round()
                        .clamp(1.0, 65_536.0) as u32
                };
                let mut height = if sy == 0.0 {
                    original.height
                } else {
                    (sy * (delta.0 * v.0 + delta.1 * v.1))
                        .round()
                        .clamp(1.0, 65_536.0) as u32
                };
                if self.aspect_lock && sx != 0.0 && sy != 0.0 {
                    let w = f64::from(original.width);
                    let h = f64::from(original.height);
                    let desired_w = sx * (delta.0 * u.0 + delta.1 * u.1);
                    let desired_h = sy * (delta.0 * v.0 + delta.1 * v.1);
                    // Closest point on the original corner diagonal: one scale
                    // in shape-local axes, also after rotation and zoom.
                    let scale = ((desired_w * w + desired_h * h) / (w * w + h * h))
                        .clamp(1.0 / w.min(h), 65_536.0 / w.max(h));
                    width = (w * scale).round().clamp(1.0, 65_536.0) as u32;
                    height = (h * scale).round().clamp(1.0, 65_536.0) as u32;
                }
                let new_center = (
                    fixed.0
                        + sx * f64::from(width) / 2.0 * u.0
                        + sy * f64::from(height) / 2.0 * v.0,
                    fixed.1
                        + sx * f64::from(width) / 2.0 * u.1
                        + sy * f64::from(height) / 2.0 * v.1,
                );
                let mut shape = original;
                shape.width = width;
                shape.height = height;
                shape.x = (new_center.0 - f64::from(width) / 2.0).round() as i32;
                shape.y = (new_center.1 - f64::from(height) / 2.0).round() as i32;
                self.document.adjust_shape(shape)?;
            }
            Some(Gesture::Rotate {
                mut original,
                anchor_angle,
            }) => {
                let c = center(original);
                let delta = (point.0 - c.0, point.1 - c.1);
                if delta.0.hypot(delta.1) > f64::EPSILON {
                    let degrees = f64::from(original.rotation_degrees)
                        + (delta.1.atan2(delta.0) - anchor_angle).to_degrees();
                    original.rotation_degrees = (degrees.round() as i32).rem_euclid(360);
                    self.document.adjust_shape(original)?;
                }
            }
            None => (),
        }
        Ok(())
    }

    /// Release stops dragging but deliberately leaves the shape editable.
    pub fn pointer_up(&mut self) -> Result<(), ControllerError> {
        self.gesture = None;
        Ok(())
    }

    /// Used for lost pointer capture / focus loss: never leave an active drag.
    /// The uncommitted shape is discarded; existing document/history survive.
    pub fn cancel_pointer(&mut self) {
        self.cancel();
        self.aspect_lock = false;
        self.last_pointer = None;
    }

    pub fn commit(&mut self) -> Result<bool, ControllerError> {
        self.gesture = None;
        Ok(self.document.commit_shape()?)
    }

    pub fn cancel(&mut self) {
        self.gesture = None;
        self.document.cancel_shape();
    }

    pub fn undo(&mut self) -> Result<bool, ControllerError> {
        self.gesture = None;
        Ok(self.document.undo()?)
    }

    pub fn redo(&mut self) -> Result<bool, ControllerError> {
        self.gesture = None;
        Ok(self.document.redo()?)
    }

    pub fn pan(&mut self, dx: f64, dy: f64) -> Result<(), ControllerError> {
        let x = self.viewport.offset_x + dx;
        let y = self.viewport.offset_y + dy;
        if !x.is_finite()
            || !y.is_finite()
            || x.abs() > MAX_VIEW_OFFSET
            || y.abs() > MAX_VIEW_OFFSET
        {
            return Err(ControllerError::InvalidInput);
        }
        self.viewport.offset_x = x;
        self.viewport.offset_y = y;
        Ok(())
    }

    /// Keeps the document coordinate beneath a logical pointer fixed.
    pub fn zoom_at(&mut self, x: f64, y: f64, factor: f64) -> Result<(), ControllerError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(ControllerError::InvalidInput);
        }
        let anchor = self.document_point(x, y)?;
        let zoom = (self.viewport.zoom * factor).clamp(1.0 / 16.0, 32.0);
        let next = Viewport {
            zoom,
            offset_x: x - anchor.0 * zoom,
            offset_y: y - anchor.1 * zoom,
        };
        if !next.offset_x.is_finite()
            || !next.offset_y.is_finite()
            || next.offset_x.abs() > MAX_VIEW_OFFSET
            || next.offset_y.abs() > MAX_VIEW_OFFSET
        {
            return Err(ControllerError::InvalidInput);
        }
        self.viewport = next;
        Ok(())
    }

    pub fn reset_view(&mut self) {
        self.viewport = Viewport::default();
    }

    pub fn nudge_shape(&mut self, dx: i32, dy: i32) -> Result<(), ControllerError> {
        let mut shape = self
            .document
            .active_shape()
            .ok_or(ControllerError::NoActiveShape)?;
        shape.x = shape.x.saturating_add(dx);
        shape.y = shape.y.saturating_add(dy);
        self.document.adjust_shape(shape)?;
        Ok(())
    }

    pub fn set_shape_bounds(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<(), ControllerError> {
        let mut shape = self
            .document
            .active_shape()
            .ok_or(ControllerError::NoActiveShape)?;
        shape.x = x;
        shape.y = y;
        shape.width = width;
        shape.height = height;
        self.document.adjust_shape(shape)?;
        Ok(())
    }

    pub fn set_shape_rotation(&mut self, degrees: i32) -> Result<(), ControllerError> {
        let mut shape = self
            .document
            .active_shape()
            .ok_or(ControllerError::NoActiveShape)?;
        shape.rotation_degrees = degrees.rem_euclid(360);
        self.document.adjust_shape(shape)?;
        Ok(())
    }

    pub fn set_primary_color(&mut self, color: Color) -> Result<(), ControllerError> {
        if let Some(mut shape) = self.document.active_shape() {
            shape.style = match shape.style {
                ShapeStyle::Fill(_) => ShapeStyle::Fill(color),
                ShapeStyle::Outline { width, .. } => ShapeStyle::Outline { color, width },
                ShapeStyle::Both { outline, width, .. } => ShapeStyle::Both {
                    fill: color,
                    outline,
                    width,
                },
            };
            self.document.adjust_shape(shape)?;
        }
        self.primary_color = color;
        Ok(())
    }

    pub fn rename_layer(&mut self, name: &str) -> Result<(), ControllerError> {
        // Validate before committing an active shape or allocating a new name.
        if name.trim().is_empty()
            || name.chars().take(MAX_LAYER_NAME_CHARS + 1).count() > MAX_LAYER_NAME_CHARS
        {
            return Err(ControllerError::InvalidLayerName);
        }
        self.commit()?;
        self.document
            .rename_layer(self.document.selected_layer(), name)?;
        Ok(())
    }

    pub fn composite(&self) -> Vec<Color> {
        self.document.composite(true)
    }

    /// One full-image CPU composite and one RGBA allocation per call. This is
    /// intentionally a reference upload path, not a claim of GPU acceleration.
    pub fn preview_rgba(&self) -> Vec<u8> {
        let width = self.document.width() as usize;
        let pixels = self.composite();
        let mut output = Vec::with_capacity(pixels.len() * 4);
        for (index, pixel) in pixels.into_iter().enumerate() {
            let background = if ((index % width) / 8 + (index / width) / 8).is_multiple_of(2) {
                224
            } else {
                192
            };
            let channel = |value: u8| {
                ((u32::from(value) * u32::from(pixel.a)
                    + background * (255 - u32::from(pixel.a))
                    + 127)
                    / 255) as u8
            };
            output.extend_from_slice(&[channel(pixel.r), channel(pixel.g), channel(pixel.b), 255]);
        }
        output
    }
}
