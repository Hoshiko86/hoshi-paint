//! Viewport panning with tool-local drag state.
use super::{ControllerError, PointerInput, ToolBehavior, ToolCx};

/// Pan drag state; no recorded pointer means no drag is active.
#[derive(Default)]
pub(crate) struct PanTool {
    last: Option<(f64, f64)>,
}

impl ToolBehavior for PanTool {
    fn pointer_down(
        &mut self,
        _cx: &mut ToolCx<'_>,
        p: PointerInput,
    ) -> Result<(), ControllerError> {
        self.last = Some((p.x, p.y));
        Ok(())
    }

    fn pointer_move(
        &mut self,
        cx: &mut ToolCx<'_>,
        p: PointerInput,
    ) -> Result<(), ControllerError> {
        if let Some(last) = self.last {
            cx.viewport.pan_by(p.x - last.0, p.y - last.1)?;
            self.last = Some((p.x, p.y));
        }
        Ok(())
    }

    fn pointer_up(&mut self, _cx: &mut ToolCx<'_>) {
        self.last = None;
    }

    fn is_dragging(&self) -> bool {
        self.last.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Viewport;

    #[test]
    fn drag_applies_the_sum_of_logical_deltas() {
        let mut viewport = Viewport::default();
        let mut cx = ToolCx {
            viewport: &mut viewport,
        };
        let mut pan = PanTool::default();
        pan.pointer_down(&mut cx, PointerInput { x: 10.0, y: 20.0 })
            .unwrap();
        pan.pointer_move(&mut cx, PointerInput { x: 14.0, y: 18.0 })
            .unwrap();
        pan.pointer_move(&mut cx, PointerInput { x: 25.0, y: 30.0 })
            .unwrap();
        assert_eq!(cx.viewport.offset_x, 15.0);
        assert_eq!(cx.viewport.offset_y, 10.0);
        assert!(pan.is_dragging());
    }

    #[test]
    fn move_without_a_drag_preserves_the_viewport() {
        let mut viewport = Viewport::default();
        let before = viewport;
        let mut pan = PanTool::default();
        pan.pointer_move(
            &mut ToolCx {
                viewport: &mut viewport,
            },
            PointerInput { x: 20.0, y: 30.0 },
        )
        .unwrap();
        assert_eq!(viewport, before);
        assert!(!pan.is_dragging());
    }

    #[test]
    fn failed_delta_preserves_viewport_and_previous_pointer() {
        let mut viewport = Viewport::default();
        let before = viewport;
        let mut cx = ToolCx {
            viewport: &mut viewport,
        };
        let mut pan = PanTool::default();
        pan.pointer_down(&mut cx, PointerInput { x: 10.0, y: 20.0 })
            .unwrap();
        assert_eq!(
            pan.pointer_move(
                &mut cx,
                PointerInput {
                    x: f64::MAX,
                    y: 25.0
                }
            ),
            Err(ControllerError::InvalidInput)
        );
        assert_eq!(*cx.viewport, before);
        pan.pointer_move(&mut cx, PointerInput { x: 12.0, y: 23.0 })
            .unwrap();
        assert_eq!(cx.viewport.offset_x, 2.0);
        assert_eq!(cx.viewport.offset_y, 3.0);
    }

    #[test]
    fn pointer_up_ends_the_drag() {
        let mut viewport = Viewport::default();
        let before = viewport;
        let mut cx = ToolCx {
            viewport: &mut viewport,
        };
        let mut pan = PanTool::default();
        pan.pointer_down(&mut cx, PointerInput { x: 10.0, y: 20.0 })
            .unwrap();
        pan.pointer_up(&mut cx);
        assert!(!pan.is_dragging());
        pan.pointer_move(&mut cx, PointerInput { x: 20.0, y: 30.0 })
            .unwrap();
        assert_eq!(*cx.viewport, before);
    }
}
