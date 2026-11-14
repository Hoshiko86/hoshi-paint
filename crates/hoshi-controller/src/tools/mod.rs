//! Narrow tool capabilities and behavior, introduced one tool at a time.
use super::{ControllerError, Viewport};

/// Pointer position in logical view pixels (same units as Controller::pointer_move).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerInput {
    pub x: f64,
    pub y: f64,
}

/// Narrow capabilities a tool may touch (ADR 0003 §2).
pub(crate) struct ToolCx<'a> {
    pub viewport: &'a mut Viewport,
}

/// Pointer behavior shared by migrated tools, without access to the controller.
pub(crate) trait ToolBehavior {
    fn pointer_down(&mut self, cx: &mut ToolCx<'_>, p: PointerInput)
    -> Result<(), ControllerError>;
    fn pointer_move(&mut self, cx: &mut ToolCx<'_>, p: PointerInput)
    -> Result<(), ControllerError>;
    /// Release, capture loss or focus loss.
    fn pointer_up(&mut self, cx: &mut ToolCx<'_>);
    fn is_dragging(&self) -> bool;
}

pub(crate) mod pan;
