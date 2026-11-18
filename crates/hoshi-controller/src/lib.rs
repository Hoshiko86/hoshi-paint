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
mod tools;
mod viewport;
use editing::{EditState, Gesture};
pub use hoshi_core::geometry::ResizeHandle;
pub use input::{EditCursor, InputModifiers, PointerButton};
pub use preview::PreviewImage;
pub use tool_id::ToolId;
pub use tools::color_picker::{AfterClick, PickerSampling, PickerSettings, SampleSize};
pub use tools::settings::{
    BrushSettings, BrushWidth, ColorSlot, ColorState, ShapeMode, ShapeSettings, ToolSettings,
};
use tools::{
    PointerInput, ToolCx, ToolState,
    color_picker::ColorPick,
    pan::PanTool,
    shapes::{ShapeGesture, ShapeTool},
};
pub use viewport::Viewport;

pub use hoshi_core::{
    Anchor, Color, FillPattern, HistoryLabel, HistoryStep, ImageTransform, LineStyle,
    RenderOptions, Resampling, ResizeOptions, ShapeStyle,
};
use hoshi_core::{Document, Error, Shape, ShapeKind};

pub const MAX_LAYER_NAME_CHARS: usize = 128;
const MAX_VIEW_OFFSET: f64 = 1_000_000_000.0;
const HANDLE_RADIUS: f64 = 8.0;

/// The editing operation represented by a shape control point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleKind {
    /// Resize the frame using this corner or edge.
    Resize(ResizeHandle),
    /// Rotate the frame around its pivot.
    Rotate,
    /// Move the frame's rotation pivot.
    Pivot,
}

/// Control points in logical viewport pixels: resize handles in
/// `ResizeHandle::ALL` order, followed by rotate and pivot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandlePoint {
    pub x: f64,
    pub y: f64,
    pub kind: HandleKind,
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

/// Exactly one legacy gesture or migrated tool may own the current drag.
enum ActiveDrag {
    Gesture(Gesture),
    Tool { state: ToolState, temporary: bool },
}

pub struct Controller {
    document: Document,
    viewport: Viewport,
    tool: Tool,
    drag: Option<ActiveDrag>,
    settings: ToolSettings,
    colors: ColorState,
    modifiers: InputModifiers,
    edit_state: EditState,
    shape_kind: ShapeKind,
    last_pointer: Option<(f64, f64)>,
    pick: Option<ColorPick>,
    previous_tool: Option<Tool>,
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
            drag: None,
            settings: ToolSettings::default(),
            colors: ColorState::default(),
            modifiers: InputModifiers::default(),
            edit_state: EditState::default(),
            shape_kind: ShapeKind::Rectangle,
            last_pointer: None,
            pick: None,
            previous_tool: None,
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
        self.clear_gesture();
        self.pick = None;
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
        self.colors.primary
    }
    pub fn selected_color(&self) -> Color {
        self.colors.selected()
    }
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Tool changes preserve the editable shape. Starting a new rectangle outside
    /// its bounds commits it; sampling a color updates its current style.
    pub fn set_tool(&mut self, tool: Tool) -> Result<(), ControllerError> {
        self.finish_stroke()?;
        if tool != self.tool {
            self.clear_gesture();
            self.pick = None;
            if self.tool != Tool::Eyedropper {
                self.previous_tool = Some(self.tool);
            }
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
        if let Some(pick) = &mut self.pick
            && let Some(color) = pick.sample(&self.document, point)
        {
            match pick.target() {
                ColorSlot::Primary => self.set_primary_color(color)?,
                ColorSlot::Secondary => self.set_secondary_color(color)?,
            }
        }
        Ok(())
    }

    fn legacy_gesture(&self) -> Option<Gesture> {
        match &self.drag {
            Some(ActiveDrag::Gesture(gesture)) => Some(*gesture),
            _ => None,
        }
    }

    /// Starts `gesture`, ending any tool drag first (one drag at a time).
    fn begin_gesture(&mut self, gesture: Gesture) {
        self.clear_gesture();
        self.drag = Some(ActiveDrag::Gesture(gesture));
    }

    fn shape_gesture(&self) -> Option<&ShapeGesture> {
        match &self.drag {
            Some(ActiveDrag::Tool {
                state: ToolState::Shapes(tool),
                ..
            }) => Some(&tool.gesture),
            _ => None,
        }
    }

    fn begin_shape(&mut self, gesture: ShapeGesture) {
        self.clear_gesture();
        self.drag = Some(ActiveDrag::Tool {
            state: ToolState::Shapes(ShapeTool { gesture }),
            temporary: false,
        });
    }

    fn tool_cx(&mut self) -> ToolCx<'_> {
        let template = self.create_shape(0, 0, 1, 1);
        ToolCx {
            viewport: &mut self.viewport,
            document: &mut self.document,
            modifiers: self.modifiers,
            pivot: &mut self.edit_state.pivot,
            template,
        }
    }

    /// Ends every drag: legacy gestures and migrated tools.
    fn clear_gesture(&mut self) {
        if let Some(ActiveDrag::Tool { mut state, .. }) = self.drag.take() {
            state.pointer_up(&mut self.tool_cx());
        }
    }

    fn begin_tool(
        &mut self,
        mut state: ToolState,
        p: PointerInput,
        temporary: bool,
    ) -> Result<(), ControllerError> {
        state.pointer_down(&mut self.tool_cx(), p)?;
        self.clear_gesture();
        self.drag = Some(ActiveDrag::Tool { state, temporary });
        Ok(())
    }

    fn begin_pan(&mut self, p: PointerInput, temporary: bool) -> Result<(), ControllerError> {
        self.begin_tool(ToolState::Pan(PanTool::default()), p, temporary)
    }

    /// Temporarily navigate without selecting the Pan tool or committing the
    /// current preview. Coordinates are logical view pixels, like pointer_move.
    pub fn begin_temporary_pan(&mut self, x: f64, y: f64) -> Result<(), ControllerError> {
        self.document_point(x, y)?;
        self.pointer_up()?;
        self.modifiers = InputModifiers::default();
        self.last_pointer = Some((x, y));
        self.begin_pan(PointerInput { x, y }, true)?;
        Ok(())
    }

    pub fn end_temporary_pan(&mut self) -> Result<(), ControllerError> {
        if matches!(
            self.drag,
            Some(ActiveDrag::Tool {
                temporary: true,
                ..
            })
        ) {
            self.pointer_up()?;
        }
        Ok(())
    }

    /// Release commits a stroke and stops dragging, leaving shapes editable.
    pub fn pointer_up(&mut self) -> Result<(), ControllerError> {
        let sampled = matches!(self.legacy_gesture(), Some(Gesture::Sample))
            && self.pick.as_ref().is_some_and(ColorPick::sampled);
        self.finish_stroke()?;
        self.clear_gesture();
        self.pick = None;
        if sampled
            && self.settings.picker.after_click == AfterClick::PreviousTool
            && let Some(previous) = self.previous_tool
        {
            self.set_tool(previous)?;
        }
        Ok(())
    }

    /// Used for lost pointer capture / focus loss: never leave an active drag.
    /// Interrupting sampling or viewport panning preserves the editable shape
    /// and selected color. Interrupted strokes commit; shape gestures discard
    /// their preview. Stroke commit errors are returned after resetting input.
    pub fn cancel_pointer(&mut self) -> Result<(), ControllerError> {
        let result = if matches!(
            self.drag,
            Some(ActiveDrag::Tool {
                state: ToolState::Brush(_),
                ..
            })
        ) {
            self.finish_stroke().map(|_| ())
        } else if matches!(self.legacy_gesture(), Some(Gesture::Sample))
            || matches!(
                self.drag,
                Some(ActiveDrag::Tool {
                    state: ToolState::Pan(_),
                    ..
                })
            )
        {
            self.clear_gesture();
            self.pick = None;
            Ok(())
        } else {
            self.cancel();
            Ok(())
        };
        self.modifiers = InputModifiers::default();
        self.last_pointer = None;
        result
    }

    pub fn commit(&mut self) -> Result<bool, ControllerError> {
        let stroke_changed = self.finish_stroke()?;
        self.clear_gesture();
        self.pick = None;
        let changed = self.document.commit_shape()? || stroke_changed;
        self.edit_state = EditState::default();
        Ok(changed)
    }

    pub fn cancel(&mut self) {
        self.clear_gesture();
        self.pick = None;
        self.document.cancel_shape();
        self.edit_state = EditState::default();
    }

    pub fn undo(&mut self) -> Result<bool, ControllerError> {
        self.finish_stroke()?;
        self.clear_gesture();
        self.pick = None;
        Ok(self.document.undo()?)
    }

    pub fn redo(&mut self) -> Result<bool, ControllerError> {
        self.finish_stroke()?;
        self.clear_gesture();
        self.pick = None;
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
        self.clear_gesture();
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
        self.colors.slot
    }
    pub fn secondary_color(&self) -> Color {
        self.colors.secondary
    }
    pub fn shape_mode(&self) -> ShapeMode {
        self.settings.shape.mode
    }
    pub fn stroke_width(&self) -> u32 {
        self.settings.brush.width.get().ceil() as u32
    }
    pub fn set_color_slot(&mut self, target: ColorSlot) {
        self.colors.slot = target;
    }

    fn drawing_style(&self) -> ShapeStyle {
        let (primary, secondary) = if self.edit_state.secondary_colors {
            (self.colors.secondary, self.colors.primary)
        } else {
            (self.colors.primary, self.colors.secondary)
        };
        match self.settings.shape.mode {
            ShapeMode::Fill => ShapeStyle::Fill(primary),
            ShapeMode::Outline => ShapeStyle::Outline {
                color: primary,
                width: self.settings.brush.width.get(),
            },
            ShapeMode::Both => ShapeStyle::Both {
                fill: secondary,
                outline: primary,
                width: self.settings.brush.width.get(),
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
        self.set_settings(ToolSettings {
            shape: ShapeSettings {
                mode,
                ..self.settings.shape
            },
            ..self.settings
        })
    }

    /// Integer compatibility adapter; decimal-aware callers use set_stroke_width_pixels.
    pub fn set_stroke_width(&mut self, width: u32) -> Result<(), ControllerError> {
        self.set_stroke_width_pixels(f64::from(width))
    }

    /// Set Primary regardless of which workspace slot is selected.
    pub fn set_primary_color(&mut self, color: Color) -> Result<(), ControllerError> {
        self.set_colors(ColorState {
            primary: color,
            ..self.colors
        })
    }
    pub fn set_secondary_color(&mut self, color: Color) -> Result<(), ControllerError> {
        self.set_colors(ColorState {
            secondary: color,
            ..self.colors
        })
    }
    pub fn swap_colors(&mut self) -> Result<(), ControllerError> {
        self.set_colors(self.colors.swapped())
    }
    pub fn reset_colors(&mut self) -> Result<(), ControllerError> {
        self.set_colors(ColorState {
            slot: self.colors.slot,
            ..ColorState::default()
        })
    }
    /// Color inputs and sampling edit this slot without changing tool semantics.
    pub fn set_selected_color(&mut self, color: Color) -> Result<(), ControllerError> {
        let mut colors = self.colors;
        colors.set_selected(color);
        self.set_colors(colors)
    }

    /// Return the validated shared toolbar values.
    pub fn settings(&self) -> ToolSettings {
        self.settings
    }
    /// Restyle the preview, restoring toolbar values if adjustment fails.
    pub fn set_settings(&mut self, settings: ToolSettings) -> Result<(), ControllerError> {
        let previous = std::mem::replace(&mut self.settings, settings);
        if let Err(error) = self.refresh_shape_style() {
            self.settings = previous;
            return Err(error);
        }
        Ok(())
    }
    /// Return the independent workspace color state.
    pub fn colors(&self) -> ColorState {
        self.colors
    }
    /// Recolor the preview, restoring colors if adjustment fails.
    pub fn set_colors(&mut self, colors: ColorState) -> Result<(), ControllerError> {
        let previous = std::mem::replace(&mut self.colors, colors);
        if let Err(error) = self.refresh_shape_style() {
            self.colors = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn flatten_for_export(&mut self, matte: Option<Color>) -> Result<(), ControllerError> {
        self.pointer_up()?;
        self.commit()?;
        self.document.flatten_for_save(matte)?;
        Ok(())
    }
    pub fn replace_document(&mut self, document: Document) {
        self.document = document;
        self.clear_gesture();
        self.pick = None;
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
        self.clear_gesture();
        self.pick = None;
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

#[cfg(test)]
mod picker_initial_state_tests {
    use super::*;

    #[test]
    fn a_successful_pick_without_a_previous_tool_keeps_the_picker() {
        let mut controller = Controller::new(1, 1).unwrap();
        // Public tool selection records Rectangle; seed a picker-first startup
        // state here to exercise the otherwise unreachable no-previous branch.
        controller.tool = Tool::Eyedropper;
        controller.settings.picker.after_click = AfterClick::PreviousTool;
        let history = controller.document().undo_len();
        controller.pointer_down(0., 0.).unwrap();
        controller.pointer_up().unwrap();
        assert_eq!(controller.tool(), Tool::Eyedropper);
        assert_eq!(controller.document().undo_len(), history);
    }
}
