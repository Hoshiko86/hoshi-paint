//! Brush and eraser stroke preview input; commit policy belongs to the controller.
use super::{ControllerError, PointerInput, ToolBehavior, ToolCx};
use hoshi_core::{StrokeStyle, geometry::Point};

/// Fixed style shared by every segment of one brush or eraser stroke.
pub(crate) struct BrushTool {
    style: StrokeStyle,
}

impl BrushTool {
    /// Retains a validated style for a stroke without starting document edits.
    pub(crate) fn new(style: StrokeStyle) -> Self {
        Self { style }
    }
}

impl ToolBehavior for BrushTool {
    fn pointer_down(
        &mut self,
        cx: &mut ToolCx<'_>,
        p: PointerInput,
    ) -> Result<(), ControllerError> {
        let (x, y) = cx.viewport.to_document(p.x, p.y);
        cx.document.begin_stroke(self.style, Point::new(x, y))?;
        Ok(())
    }

    fn pointer_move(
        &mut self,
        cx: &mut ToolCx<'_>,
        p: PointerInput,
    ) -> Result<(), ControllerError> {
        let (x, y) = cx.viewport.to_document(p.x, p.y);
        cx.document.extend_stroke(Point::new(x, y))?;
        Ok(())
    }

    fn pointer_up(&mut self, _cx: &mut ToolCx<'_>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Viewport;
    use hoshi_core::{BrushTip, Color, Document, Error, StrokeBlend};

    fn brush() -> BrushTool {
        BrushTool::new(StrokeStyle {
            tip: BrushTip::new(3.0, false).unwrap(),
            blend: StrokeBlend::Paint(Color::rgba(200, 40, 60, 255)),
        })
    }

    #[test]
    fn logical_drag_paints_only_the_stroke_preview() {
        let mut document = Document::new(16, 16).unwrap();
        let mut viewport = Viewport {
            zoom: 2.0,
            offset_x: 10.0,
            offset_y: 20.0,
        };
        let mut tool = brush();
        let mut cx = ToolCx {
            viewport: &mut viewport,
            document: &mut document,
        };
        tool.pointer_down(&mut cx, PointerInput { x: 19.0, y: 29.0 })
            .unwrap();
        tool.pointer_move(&mut cx, PointerInput { x: 27.0, y: 29.0 })
            .unwrap();
        let color = Color::rgba(200, 40, 60, 255);
        assert_eq!(cx.document.composite_pixel(4, 4, true), color);
        assert_eq!(cx.document.composite_pixel(8, 4, true), color);
        assert_eq!(cx.document.composite_pixel(4, 4, false), Color::TRANSPARENT);
        assert_eq!(cx.document.composite_pixel(8, 4, false), Color::TRANSPARENT);
        assert_eq!(cx.document.undo_len(), 0);
        tool.pointer_up(&mut cx);
        assert!(cx.document.has_active_stroke());
        assert_eq!(cx.document.undo_len(), 0);
    }

    #[test]
    fn invalid_move_returns_an_error_and_preserves_the_preview() {
        let mut document = Document::new(16, 16).unwrap();
        let mut viewport = Viewport::default();
        let mut tool = brush();
        let mut cx = ToolCx {
            viewport: &mut viewport,
            document: &mut document,
        };
        tool.pointer_down(&mut cx, PointerInput { x: 4.5, y: 4.5 })
            .unwrap();
        let before = cx.document.composite(true);
        for x in [f64::NAN, f64::INFINITY] {
            assert_eq!(
                tool.pointer_move(&mut cx, PointerInput { x, y: 4.5 }),
                Err(ControllerError::Core(Error::InvalidStroke))
            );
            assert_eq!(cx.document.composite(true), before);
        }
        tool.pointer_move(&mut cx, PointerInput { x: 8.5, y: 4.5 })
            .unwrap();
        assert_ne!(cx.document.composite(true), before);
    }
}
