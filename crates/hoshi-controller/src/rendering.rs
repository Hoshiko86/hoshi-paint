//! Workspace paint defaults and live preview styling, separate from gestures.
use super::{Controller, ControllerError, FillPattern, LineStyle, RenderOptions, Shape, ShapeMode};
use hoshi_core::Error;
impl Controller {
    pub fn stroke_width_pixels(&self) -> f64 {
        self.stroke_width
    }
    /// Strokes are inset inside the bounds; wide strokes can consume the fill.
    pub fn set_stroke_width_pixels(&mut self, width: f64) -> Result<(), ControllerError> {
        if !width.is_finite() || !(0.01..=65_536.).contains(&width) {
            return Err(Error::InvalidShape.into());
        }
        self.stroke_width = width;
        self.refresh_shape_style()
    }
    pub fn antialiasing(&self) -> bool {
        self.rendering.antialias
    }
    pub fn line_style(&self) -> LineStyle {
        self.rendering.line_style
    }
    pub fn fill_pattern(&self) -> FillPattern {
        self.rendering.fill_pattern
    }
    pub fn set_antialiasing(&mut self, enabled: bool) -> Result<(), ControllerError> {
        self.rendering.antialias = enabled;
        self.refresh_shape_style()
    }
    pub fn set_line_style(&mut self, style: LineStyle) -> Result<(), ControllerError> {
        self.rendering.line_style = style;
        self.refresh_shape_style()
    }
    pub fn set_fill_pattern(&mut self, pattern: FillPattern) -> Result<(), ControllerError> {
        self.rendering.fill_pattern = pattern;
        self.refresh_shape_style()
    }
    pub(super) fn drawing_render_options(&self) -> RenderOptions {
        let (primary, secondary) = if self.edit_state.secondary_colors {
            (self.secondary_color, self.primary_color)
        } else {
            (self.primary_color, self.secondary_color)
        };
        RenderOptions {
            pattern_color: if self.shape_mode == ShapeMode::Both {
                primary
            } else {
                secondary
            },
            ..self.rendering
        }
    }
    pub(super) fn create_shape(&self, x: i32, y: i32, width: u32, height: u32) -> Shape {
        let mut shape = Shape::new(self.shape_kind, x, y, width, height, self.drawing_style());
        shape.rendering = self.drawing_render_options();
        shape
    }
}
