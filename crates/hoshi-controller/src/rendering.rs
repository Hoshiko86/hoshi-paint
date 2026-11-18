//! Workspace paint defaults and live preview styling, separate from gestures.
use super::{BrushSettings, BrushWidth, ShapeSettings, ToolSettings};
use super::{Controller, ControllerError, FillPattern, LineStyle, RenderOptions, Shape, ShapeMode};
impl Controller {
    pub fn stroke_width_pixels(&self) -> f64 {
        self.settings.brush.width.get()
    }
    /// Strokes are inset inside the bounds; wide strokes can consume the fill.
    pub fn set_stroke_width_pixels(&mut self, width: f64) -> Result<(), ControllerError> {
        let width = BrushWidth::new(width)?;
        self.set_settings(ToolSettings {
            brush: BrushSettings { width },
            ..self.settings
        })
    }
    pub fn antialiasing(&self) -> bool {
        self.settings.antialias
    }
    pub fn line_style(&self) -> LineStyle {
        self.settings.shape.line_style
    }
    pub fn fill_pattern(&self) -> FillPattern {
        self.settings.fill
    }
    pub fn set_antialiasing(&mut self, enabled: bool) -> Result<(), ControllerError> {
        self.set_settings(ToolSettings {
            antialias: enabled,
            ..self.settings
        })
    }
    pub fn set_line_style(&mut self, style: LineStyle) -> Result<(), ControllerError> {
        self.set_settings(ToolSettings {
            shape: ShapeSettings {
                line_style: style,
                ..self.settings.shape
            },
            ..self.settings
        })
    }
    pub fn set_fill_pattern(&mut self, pattern: FillPattern) -> Result<(), ControllerError> {
        self.set_settings(ToolSettings {
            fill: pattern,
            ..self.settings
        })
    }
    pub(super) fn drawing_render_options(&self) -> RenderOptions {
        let (primary, secondary) = if self.edit_state.secondary_colors {
            (self.colors.secondary, self.colors.primary)
        } else {
            (self.colors.primary, self.colors.secondary)
        };
        RenderOptions {
            pattern_color: if self.settings.shape.mode == ShapeMode::Both {
                primary
            } else {
                secondary
            },
            antialias: self.settings.antialias,
            line_style: self.settings.shape.line_style,
            fill_pattern: self.settings.fill,
        }
    }
    pub(super) fn create_shape(&self, x: i32, y: i32, width: u32, height: u32) -> Shape {
        let mut shape = Shape::new(self.shape_kind, x, y, width, height, self.drawing_style());
        shape.rendering = self.drawing_render_options();
        shape
    }
}
