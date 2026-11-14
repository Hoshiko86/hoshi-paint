//! Viewport panning with tool-local drag state.
use super::{ControllerError, PointerInput, ToolBehavior, ToolCx};

/// Previous pointer for a Pan tool owned by the controller's active drag.
#[derive(Default)]
pub(crate) struct PanTool {
    last: (f64, f64),
}

impl ToolBehavior for PanTool {
    fn pointer_down(
        &mut self,
        _cx: &mut ToolCx<'_>,
        p: PointerInput,
    ) -> Result<(), ControllerError> {
        self.last = (p.x, p.y);
        Ok(())
    }

    fn pointer_move(
        &mut self,
        cx: &mut ToolCx<'_>,
        p: PointerInput,
    ) -> Result<(), ControllerError> {
        cx.viewport.pan_by(p.x - self.last.0, p.y - self.last.1)?;
        self.last = (p.x, p.y);
        Ok(())
    }

    fn pointer_up(&mut self, _cx: &mut ToolCx<'_>) {}
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
    }

    #[test]
    fn pointer_down_records_the_anchor_without_moving_the_viewport() {
        let mut viewport = Viewport::default();
        let before = viewport;
        let mut pan = PanTool::default();
        pan.pointer_down(
            &mut ToolCx {
                viewport: &mut viewport,
            },
            PointerInput { x: 20.0, y: 30.0 },
        )
        .unwrap();
        assert_eq!(viewport, before);
        pan.pointer_move(
            &mut ToolCx {
                viewport: &mut viewport,
            },
            PointerInput { x: 20.0, y: 30.0 },
        )
        .unwrap();
        assert_eq!(viewport, before);
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
    fn pointer_up_drops_the_controllers_active_pan() {
        let mut controller = crate::Controller::new(32, 32).unwrap();
        controller.set_tool(crate::Tool::Pan).unwrap();
        controller.pointer_down(10.0, 20.0).unwrap();
        controller.pointer_move(12.0, 23.0).unwrap();
        assert!(controller.is_dragging());
        let before = controller.viewport();
        controller.pointer_up().unwrap();
        assert!(!controller.is_dragging());
        controller.pointer_move(20.0, 30.0).unwrap();
        assert_eq!(controller.viewport(), before);
    }
}
