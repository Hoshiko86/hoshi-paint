//! Narrow tool capabilities and behavior, introduced one tool at a time.
use super::{ControllerError, Viewport};
use hoshi_core::Document;

/// Pointer position in logical view pixels (same units as Controller::pointer_move).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerInput {
    pub x: f64,
    pub y: f64,
}

/// Narrow capabilities a tool may touch (ADR 0003 §2).
pub(crate) struct ToolCx<'a> {
    pub viewport: &'a mut Viewport,
    pub document: &'a mut Document,
}

/// Pointer behavior shared by migrated tools, without access to the controller.
pub(crate) trait ToolBehavior {
    fn pointer_down(&mut self, cx: &mut ToolCx<'_>, p: PointerInput)
    -> Result<(), ControllerError>;
    fn pointer_move(&mut self, cx: &mut ToolCx<'_>, p: PointerInput)
    -> Result<(), ControllerError>;
    /// Release, capture loss or focus loss.
    fn pointer_up(&mut self, cx: &mut ToolCx<'_>);
}

pub(crate) mod brush;
pub(crate) mod pan;

/// Closed dispatch over migrated tools.
pub(crate) enum ToolState {
    Pan(pan::PanTool),
    Brush(brush::BrushTool),
}

impl ToolState {
    /// Starts the selected tool using only its granted capabilities.
    pub(crate) fn pointer_down(
        &mut self,
        cx: &mut ToolCx<'_>,
        p: PointerInput,
    ) -> Result<(), ControllerError> {
        match self {
            Self::Pan(tool) => tool.pointer_down(cx, p),
            Self::Brush(tool) => tool.pointer_down(cx, p),
        }
    }

    /// Extends the active tool drag in logical view coordinates.
    pub(crate) fn pointer_move(
        &mut self,
        cx: &mut ToolCx<'_>,
        p: PointerInput,
    ) -> Result<(), ControllerError> {
        match self {
            Self::Pan(tool) => tool.pointer_move(cx, p),
            Self::Brush(tool) => tool.pointer_move(cx, p),
        }
    }

    /// Releases tool-local input; fallible document commits stay in the controller.
    pub(crate) fn pointer_up(&mut self, cx: &mut ToolCx<'_>) {
        match self {
            Self::Pan(tool) => tool.pointer_up(cx),
            Self::Brush(tool) => tool.pointer_up(cx),
        }
    }
}
