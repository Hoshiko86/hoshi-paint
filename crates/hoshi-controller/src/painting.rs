//! Brush gestures use the document's isolated, single-entry stroke preview.
use super::tools::{PointerInput, ToolState, brush::BrushTool};
use super::{ActiveDrag, Controller, ControllerError, PointerButton, Tool};
use hoshi_core::{BrushTip, StrokeBlend, StrokeStyle};

impl Controller {
    /// Circular brush diameter, shared with shape outlines, in document pixels.
    pub fn brush_width(&self) -> f64 {
        self.stroke_width_pixels()
    }

    /// Set the shared outline width after validating the circular brush limits.
    pub fn set_brush_width(&mut self, width: f64) -> Result<(), ControllerError> {
        BrushTip::new(width, self.rendering.antialias)?;
        self.set_stroke_width_pixels(width)
    }

    pub(super) fn begin_paint(
        &mut self,
        p: PointerInput,
        button: PointerButton,
    ) -> Result<(), ControllerError> {
        self.commit()?;
        let tip = BrushTip::new(self.brush_width(), self.rendering.antialias)?;
        let blend = if self.tool == Tool::Eraser {
            StrokeBlend::Erase
        } else {
            StrokeBlend::Paint(match button {
                PointerButton::Primary => self.primary_color,
                PointerButton::Secondary => self.secondary_color,
            })
        };
        self.begin_tool(
            ToolState::Brush(BrushTool::new(StrokeStyle { tip, blend })),
            p,
            false,
        )
    }

    pub(super) fn finish_stroke(&mut self) -> Result<bool, ControllerError> {
        if matches!(
            self.drag,
            Some(ActiveDrag::Tool {
                state: ToolState::Brush(_),
                ..
            })
        ) {
            self.clear_gesture();
        }
        match self.document.commit_stroke() {
            Ok(changed) => Ok(changed),
            // A failed commit discards the stroke, leaving the document as it
            // was before the stroke (Paint.NET parity; owner decision, #77).
            // The core keeps a failed stroke active only so callers can choose.
            Err(error) => {
                self.document.cancel_stroke();
                Err(error.into())
            }
        }
    }
}
