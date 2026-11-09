//! Platform-neutral editing intentions. UI adapters translate native events once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputModifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PointerButton {
    #[default]
    Primary,
    Secondary,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditCursor {
    Draw,
    Move,
    ResizeHorizontal,
    ResizeVertical,
    ResizeDiagonalForward,
    ResizeDiagonalBackward,
    Rotate,
    Pivot,
}
