//! Shared application controller and logical-coordinate input boundary.
//!
//! UI adapters pass logical window coordinates, never physical device pixels.
//! This controller owns document mutations; views receive immutable state and
//! an RGBA image. It has no windowing, rendering-framework, or filesystem dependency.

mod bounds;
pub use bounds::BoundsField;
pub mod color;
mod editing;
mod input;
pub mod named_colors;
mod painting;
pub mod palette;
mod preview;
mod rendering;
mod tool_id;
mod viewport;
use editing::{EditState, Gesture};
pub use input::{EditCursor, InputModifiers, PointerButton};
pub use preview::PreviewImage;
pub use tool_id::ToolId;
pub use viewport::Viewport;
pub const ROTATION_HANDLE: i32 = 8;
pub const PIVOT_HANDLE: i32 = 9;

pub use hoshi_core::{
    Anchor, Color, FillPattern, HistoryStep, ImageTransform, LineStyle, RenderOptions, Resampling,
    ResizeOptions, ShapeStyle,
};
use hoshi_core::{Document, DocumentSnapshot, Error, Shape, ShapeKind};

/// Drawing style retained for the next shape as well as the active preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeMode {
    Fill,
    Outline,
    Both,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSlot {
    Primary,
    Secondary,
}

pub const MAX_LAYER_NAME_CHARS: usize = 128;
const MAX_VIEW_OFFSET: f64 = 1_000_000_000.0;
const HANDLE_RADIUS: f64 = 8.0;

/// Control points in logical viewport pixels: corners/edges clockwise from
/// top-left (0..=7), followed by the rotation point (8) and pivot (9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandlePoint {
    pub x: f64,
    pub y: f64,
    pub kind: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Rectangle,
    MoveShape,
    Pan,
    Eyedropper,
    /// Paint a circular brush stroke.
    Brush,
    /// Erase alpha with a circular brush stroke.
    Eraser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerError {
    InvalidInput,
    InvalidLayerName,
    NoActiveShape,
    Core(Error),
}

impl std::fmt::Display for ControllerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core(e) => std::fmt::Display::fmt(e, f),
            _ => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for ControllerError {}

impl From<Error> for ControllerError {
    fn from(value: Error) -> Self {
        Self::Core(value)
    }
}

pub struct Controller {
    document: Document,
    viewport: Viewport,
    tool: Tool,
    gesture: Option<Gesture>,
    primary_color: Color,
    secondary_color: Color,
    color_slot: ColorSlot,
    shape_mode: ShapeMode,
    stroke_width: f64,
    rendering: RenderOptions,
    modifiers: InputModifiers,
    edit_state: EditState,
    shape_kind: ShapeKind,
    last_pointer: Option<(f64, f64)>,
    sample_snapshot: Option<(DocumentSnapshot, Option<Shape>)>,
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
            primary_color: Color::rgba(0, 0, 0, 255),
            secondary_color: Color::rgba(255, 255, 255, 255),
            color_slot: ColorSlot::Primary,
            shape_mode: ShapeMode::Fill,
            stroke_width: 2.,
            rendering: RenderOptions::default(),
            modifiers: InputModifiers::default(),
            edit_state: EditState::default(),
            shape_kind: ShapeKind::Rectangle,
            last_pointer: None,
            sample_snapshot: None,
        })
    }

    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn history_steps(&self) -> Vec<HistoryStep> {
        self.document.history_steps()
    }

    pub fn history_position(&self) -> usize {
        self.document.history_position()
    }

    pub fn go_to_history_step(&mut self, index: usize) -> Result<bool, ControllerError> {
        // Reject live previews before ending any active gesture. History row
        // navigation is explicit, unlike the toolbar's commit-then-undo action.
        let changed = self.document.go_to_history_step(index)?;
        self.gesture = None;
        self.sample_snapshot = None;
        self.modifiers = InputModifiers::default();
        self.last_pointer = None;
        self.edit_state = EditState::default();
        Ok(changed)
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
    pub fn selected_color(&self) -> Color {
        match self.color_slot {
            ColorSlot::Primary => self.primary_color,
            ColorSlot::Secondary => self.secondary_color,
        }
    }
    pub fn is_dragging(&self) -> bool {
        self.gesture.is_some()
    }

    /// Tool changes preserve the editable shape. Starting a new rectangle outside
    /// its bounds commits it; sampling a color updates its current style.
    pub fn set_tool(&mut self, tool: Tool) -> Result<(), ControllerError> {
        self.finish_stroke()?;
        if tool != self.tool {
            self.gesture = None;
            self.sample_snapshot = None;
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

    fn sample_color(&mut self, point: (f64, f64)) -> Result<(), ControllerError> {
        if point.0 >= 0.0
            && point.1 >= 0.0
            && point.0 < f64::from(self.document.width())
            && point.1 < f64::from(self.document.height())
            && let Some((snapshot, preview)) = &self.sample_snapshot
        {
            let color =
                snapshot.composite_pixel(point.0.floor() as u32, point.1.floor() as u32, *preview);
            self.set_selected_color(color)?;
        }
        Ok(())
    }

    /// Temporarily navigate without selecting the Pan tool or committing the
    /// current preview. Coordinates are logical view pixels, like pointer_move.
    pub fn begin_temporary_pan(&mut self, x: f64, y: f64) -> Result<(), ControllerError> {
        self.document_point(x, y)?;
        self.pointer_up()?;
        self.modifiers = InputModifiers::default();
        self.last_pointer = Some((x, y));
        self.gesture = Some(Gesture::TemporaryPan { last: (x, y) });
        Ok(())
    }

    pub fn end_temporary_pan(&mut self) -> Result<(), ControllerError> {
        if matches!(self.gesture, Some(Gesture::TemporaryPan { .. })) {
            self.pointer_up()?;
        }
        Ok(())
    }

    /// Release commits a stroke and stops dragging, leaving shapes editable.
    pub fn pointer_up(&mut self) -> Result<(), ControllerError> {
        self.finish_stroke()?;
        self.gesture = None;
        self.sample_snapshot = None;
        Ok(())
    }

    /// Used for lost pointer capture / focus loss: never leave an active drag.
    /// Interrupting sampling or viewport panning preserves the editable shape
    /// and selected color. Interrupted strokes commit; shape gestures discard
    /// their preview.
    pub fn cancel_pointer(&mut self) {
        if matches!(self.gesture, Some(Gesture::Stroke)) {
            let _ = self.finish_stroke();
        } else if matches!(
            self.gesture,
            Some(Gesture::Sample | Gesture::Pan { .. } | Gesture::TemporaryPan { .. })
        ) {
            self.gesture = None;
            self.sample_snapshot = None;
        } else {
            self.cancel();
        }
        self.modifiers = InputModifiers::default();
        self.last_pointer = None;
    }

    pub fn commit(&mut self) -> Result<bool, ControllerError> {
        let stroke_changed = self.finish_stroke()?;
        self.gesture = None;
        self.sample_snapshot = None;
        let changed = self.document.commit_shape()? || stroke_changed;
        self.edit_state = EditState::default();
        Ok(changed)
    }

    pub fn cancel(&mut self) {
        self.gesture = None;
        self.sample_snapshot = None;
        self.document.cancel_shape();
        self.edit_state = EditState::default();
    }

    pub fn undo(&mut self) -> Result<bool, ControllerError> {
        self.finish_stroke()?;
        self.gesture = None;
        self.sample_snapshot = None;
        Ok(self.document.undo()?)
    }

    pub fn redo(&mut self) -> Result<bool, ControllerError> {
        self.finish_stroke()?;
        self.gesture = None;
        self.sample_snapshot = None;
        Ok(self.document.redo()?)
    }

    pub fn nudge_shape(&mut self, dx: i32, dy: i32) -> Result<(), ControllerError> {
        let mut shape = self
            .document
            .active_shape()
            .ok_or(ControllerError::NoActiveShape)?;
        shape.x = shape.x.saturating_add(dx);
        shape.y = shape.y.saturating_add(dy);
        self.document.adjust_shape(shape)?;
        self.edit_state.pivot = self
            .edit_state
            .pivot
            .map(|p| p.translated(f64::from(dx), f64::from(dy)));
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
        let dx = f64::from(x) - f64::from(shape.x);
        let dy = f64::from(y) - f64::from(shape.y);
        shape.x = x;
        shape.y = y;
        shape.width = width;
        shape.height = height;
        self.document.adjust_shape(shape)?;
        self.edit_state.pivot = self.edit_state.pivot.map(|p| p.translated(dx, dy));
        // Numeric edits replace an unfinished pointer gesture, so its stale
        // drag-start frame cannot overwrite the entered bounds on release.
        self.gesture = None;
        self.edit_state.numeric_ratio = None;
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

    pub fn color_slot(&self) -> ColorSlot {
        self.color_slot
    }
    pub fn secondary_color(&self) -> Color {
        self.secondary_color
    }
    pub fn shape_mode(&self) -> ShapeMode {
        self.shape_mode
    }
    pub fn stroke_width(&self) -> u32 {
        self.stroke_width.ceil() as u32
    }
    pub fn set_color_slot(&mut self, target: ColorSlot) {
        self.color_slot = target;
    }

    fn drawing_style(&self) -> ShapeStyle {
        let (primary, secondary) = if self.edit_state.secondary_colors {
            (self.secondary_color, self.primary_color)
        } else {
            (self.primary_color, self.secondary_color)
        };
        match self.shape_mode {
            ShapeMode::Fill => ShapeStyle::Fill(primary),
            ShapeMode::Outline => ShapeStyle::Outline {
                color: primary,
                width: self.stroke_width,
            },
            ShapeMode::Both => ShapeStyle::Both {
                fill: secondary,
                outline: primary,
                width: self.stroke_width,
            },
        }
    }

    fn refresh_shape_style(&mut self) -> Result<(), ControllerError> {
        if let Some(mut shape) = self.document.active_shape() {
            shape.style = self.drawing_style();
            shape.rendering = self.drawing_render_options();
            self.document.adjust_shape(shape)?;
        }
        Ok(())
    }

    pub fn set_shape_mode(&mut self, mode: ShapeMode) -> Result<(), ControllerError> {
        self.shape_mode = mode;
        self.refresh_shape_style()
    }

    /// Integer compatibility adapter; decimal-aware callers use set_stroke_width_pixels.
    pub fn set_stroke_width(&mut self, width: u32) -> Result<(), ControllerError> {
        self.set_stroke_width_pixels(f64::from(width))
    }

    /// Set Primary regardless of which workspace slot is selected.
    pub fn set_primary_color(&mut self, color: Color) -> Result<(), ControllerError> {
        self.primary_color = color;
        self.refresh_shape_style()
    }
    pub fn set_secondary_color(&mut self, color: Color) -> Result<(), ControllerError> {
        self.secondary_color = color;
        self.refresh_shape_style()
    }
    pub fn swap_colors(&mut self) -> Result<(), ControllerError> {
        std::mem::swap(&mut self.primary_color, &mut self.secondary_color);
        self.refresh_shape_style()
    }
    pub fn reset_colors(&mut self) -> Result<(), ControllerError> {
        self.primary_color = Color::rgba(0, 0, 0, 255);
        self.secondary_color = Color::rgba(255, 255, 255, 255);
        self.refresh_shape_style()
    }
    /// Color inputs and sampling edit this slot without changing tool semantics.
    pub fn set_selected_color(&mut self, color: Color) -> Result<(), ControllerError> {
        match self.color_slot {
            ColorSlot::Primary => self.primary_color = color,
            ColorSlot::Secondary => self.secondary_color = color,
        }
        self.refresh_shape_style()
    }

    pub fn flatten_for_export(&mut self, matte: Option<Color>) -> Result<(), ControllerError> {
        self.pointer_up()?;
        self.commit()?;
        self.document.flatten_for_save(matte)?;
        Ok(())
    }
    pub fn replace_document(&mut self, document: Document) {
        self.document = document;
        self.gesture = None;
        self.sample_snapshot = None;
        self.last_pointer = None;
        self.modifiers = InputModifiers::default();
        self.tool = Tool::Rectangle;
        self.edit_state = EditState::default();
        self.reset_view();
    }
    pub fn new_image(
        &mut self,
        width: u32,
        height: u32,
        resolution: f64,
        background: Color,
    ) -> Result<(), ControllerError> {
        let document = Document::new_with_background(width, height, resolution, background)?;
        self.document = document;
        self.gesture = None;
        self.sample_snapshot = None;
        self.last_pointer = None;
        self.modifiers = InputModifiers::default();
        self.tool = Tool::Rectangle;
        self.edit_state = EditState::default();
        self.reset_view();
        Ok(())
    }
    pub fn resize_image(
        &mut self,
        width: u32,
        height: u32,
        resolution: f64,
        options: impl Into<ResizeOptions>,
    ) -> Result<(), ControllerError> {
        Document::validate_image_size(width, height)?;
        Document::validate_resolution(resolution)?;
        self.pointer_up()?;
        self.commit()?;
        self.document
            .resize_image(width, height, resolution, options)?;
        Ok(())
    }
    pub fn resize_canvas(
        &mut self,
        width: u32,
        height: u32,
        anchor: Anchor,
        fill: Color,
    ) -> Result<(), ControllerError> {
        Document::validate_image_size(width, height)?;
        if anchor.column > 2 || anchor.row > 2 {
            return Err(Error::InvalidDimensions.into());
        }
        self.pointer_up()?;
        self.commit()?;
        self.document.resize_canvas(width, height, anchor, fill)?;
        Ok(())
    }
    pub fn transform_image(&mut self, transform: ImageTransform) -> Result<(), ControllerError> {
        self.pointer_up()?;
        self.commit()?;
        self.document.transform_image(transform)?;
        Ok(())
    }
    pub fn flatten(&mut self) -> Result<(), ControllerError> {
        let layers = self.document.layers();
        if layers.len() == 1 && layers[0].visible() && layers[0].opacity() == 255 {
            return Ok(());
        }
        self.pointer_up()?;
        self.commit()?;
        self.document.flatten()?;
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
}
