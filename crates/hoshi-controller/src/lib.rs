//! Logical-coordinate input boundary for the disposable desktop integration spike.
//!
//! UI adapters pass logical window coordinates, never physical device pixels.
//! This controller owns document mutations; views receive immutable state and
//! an RGBA image. It has no windowing, rendering-framework, or filesystem dependency.

pub use hoshi_core::Color;
use hoshi_core::{Document, Error, Shape, ShapeKind, ShapeStyle};

pub const MAX_LAYER_NAME_CHARS: usize = 128;
const MAX_VIEW_OFFSET: f64 = 1_000_000_000.0;

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
    Rectangle { anchor: (f64, f64) },
    Move { anchor: (f64, f64), original: Shape },
    Pan { last: (f64, f64) },
}

pub struct Controller {
    document: Document,
    viewport: Viewport,
    tool: Tool,
    gesture: Option<Gesture>,
    primary_color: Color,
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

    /// MoveShape and Pan are temporary interaction modes that preserve the active
    /// shape. Returning to Rectangle commits it before starting another drawing.
    pub fn set_tool(&mut self, tool: Tool) -> Result<(), ControllerError> {
        if tool != self.tool {
            if tool == Tool::Rectangle {
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
        }
        Ok(())
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
            style: ShapeStyle::Fill(self.primary_color),
        }
    }

    pub fn pointer_move(&mut self, x: f64, y: f64) -> Result<(), ControllerError> {
        let point = self.document_point(x, y)?;
        match self.gesture {
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
