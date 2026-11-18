//! Validated shared toolbar values and independent workspace colors.
use hoshi_core::{Color, Error, FillPattern, LineStyle};

/// Drawing style retained for the next shape as well as the active preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeMode {
    /// Fill the shape interior.
    Fill,
    /// Draw only the outline.
    Outline,
    /// Draw the interior and outline.
    Both,
}
/// Workspace color slot selected for color inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSlot {
    /// Primary color slot.
    Primary,
    /// Secondary color slot.
    Secondary,
}

/// Finite brush/outline diameter in document pixels, within 0.01..=65_536.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushWidth(f64);
impl BrushWidth {
    /// Default two-pixel diameter.
    pub const DEFAULT: Self = Self(2.0);
    /// Validate a diameter before it enters toolbar state.
    pub fn new(px: f64) -> Result<Self, Error> {
        if !px.is_finite() || !(0.01..=65_536.).contains(&px) {
            return Err(Error::InvalidShape);
        }
        Ok(Self(px))
    }
    /// Return the validated diameter in document pixels.
    pub fn get(self) -> f64 {
        self.0
    }
}
/// Shared brush options.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushSettings {
    /// Validated brush and outline diameter.
    pub width: BrushWidth,
}
/// Shared shape options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeSettings {
    /// Fill/outline combination.
    pub mode: ShapeMode,
    /// Outline dash style.
    pub line_style: LineStyle,
}
/// Validated toolbar values shared by tools, independent of colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolSettings {
    /// Brush options.
    pub brush: BrushSettings,
    /// Fill pattern.
    pub fill: FillPattern,
    /// Smooth raster edges when enabled.
    pub antialias: bool,
    /// Shape options.
    pub shape: ShapeSettings,
}
impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            brush: BrushSettings {
                width: BrushWidth::DEFAULT,
            },
            fill: FillPattern::Solid,
            antialias: true,
            shape: ShapeSettings {
                mode: ShapeMode::Fill,
                line_style: LineStyle::Solid,
            },
        }
    }
}
/// Independent workspace colors and selected editing slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorState {
    /// Primary drawing color.
    pub primary: Color,
    /// Secondary drawing color.
    pub secondary: Color,
    /// Slot edited by color inputs.
    pub slot: ColorSlot,
}
impl Default for ColorState {
    fn default() -> Self {
        Self {
            primary: Color::rgba(0, 0, 0, 255),
            secondary: Color::rgba(255, 255, 255, 255),
            slot: ColorSlot::Primary,
        }
    }
}
impl ColorState {
    /// Return the selected slot's color.
    pub fn selected(self) -> Color {
        match self.slot {
            ColorSlot::Primary => self.primary,
            ColorSlot::Secondary => self.secondary,
        }
    }
    /// Write only the selected slot.
    pub fn set_selected(&mut self, color: Color) {
        match self.slot {
            ColorSlot::Primary => self.primary = color,
            ColorSlot::Secondary => self.secondary = color,
        }
    }
    /// Exchange colors while preserving the selected slot.
    pub fn swapped(self) -> Self {
        Self {
            primary: self.secondary,
            secondary: self.primary,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn width_boundaries() {
        for value in [0.01, 65_536.] {
            assert_eq!(BrushWidth::new(value).unwrap().get(), value);
        }
        for value in [
            0.,
            0.009,
            65_536.01,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            assert_eq!(BrushWidth::new(value), Err(Error::InvalidShape));
        }
    }
    #[test]
    fn selected_and_swapped_preserve_other_state() {
        for slot in [ColorSlot::Primary, ColorSlot::Secondary] {
            let mut colors = ColorState {
                slot,
                ..ColorState::default()
            };
            let previous = colors;
            let color = Color::rgba(1, 2, 3, 4);
            colors.set_selected(color);
            assert_eq!(colors.selected(), color);
            assert_eq!(colors.slot, slot);
            match slot {
                ColorSlot::Primary => assert_eq!(colors.secondary, previous.secondary),
                ColorSlot::Secondary => assert_eq!(colors.primary, previous.primary),
            }
            let swapped = colors.swapped();
            assert_eq!(swapped.slot, slot);
            assert_eq!(swapped.primary, colors.secondary);
            assert_eq!(swapped.secondary, colors.primary);
        }
    }
}
