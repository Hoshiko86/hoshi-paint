//! Bind rendering controls without putting renderer decisions in the UI layer.
use crate::{AppWindow, SharedController, update, workspace::Workspace};
use hoshi_controller::{FillPattern, LineStyle};
use slint::ComponentHandle;

// Dropdown order is a presentation detail; all editor APIs use named variants.
const LINES: [LineStyle; 5] = [
    LineStyle::Solid,
    LineStyle::Dash,
    LineStyle::Dot,
    LineStyle::DashDot,
    LineStyle::DashDotDot,
];
const FILLS: [FillPattern; 8] = [
    FillPattern::Solid,
    FillPattern::Horizontal,
    FillPattern::Vertical,
    FillPattern::ForwardDiagonal,
    FillPattern::BackwardDiagonal,
    FillPattern::Cross,
    FillPattern::DiagonalCross,
    FillPattern::Checkerboard,
];
pub(super) fn refresh(ui: &AppWindow, state: &Workspace) {
    ui.set_stroke_width_pixels(state.stroke_width_pixels() as f32);
    ui.set_shape_antialiasing(state.antialiasing());
    ui.set_line_style(
        LINES
            .iter()
            .position(|v| *v == state.line_style())
            .unwrap_or(0) as i32,
    );
    ui.set_fill_pattern(
        FILLS
            .iter()
            .position(|v| *v == state.fill_pattern())
            .unwrap_or(0) as i32,
    );
}
pub(super) fn bind(ui: &AppWindow, state: SharedController) {
    let (weak, controller) = (ui.as_weak(), state.clone());
    ui.on_edit_stroke_width_pixels(move |width| {
        // Slint transports floats as f32. Its shortest decimal representation
        // preserves entries such as 0.01 instead of exposing binary-rounding
        // noise to the controller's canonical f64 range validation.
        let width = width.to_string().parse::<f64>().unwrap_or(f64::NAN);
        update(&weak, &controller, |c| c.set_stroke_width_pixels(width));
    });
    let (weak, controller) = (ui.as_weak(), state.clone());
    ui.on_adjust_outline_width(move |delta| {
        update(&weak, &controller, |c| {
            c.set_stroke_width_pixels(
                (c.stroke_width_pixels() + f64::from(delta)).clamp(0.01, 65536.),
            )
        });
    });
    let (weak, controller) = (ui.as_weak(), state.clone());
    ui.on_toggle_shape_antialiasing(move |enabled| {
        update(&weak, &controller, |c| c.set_antialiasing(enabled));
    });
    let (weak, controller) = (ui.as_weak(), state.clone());
    ui.on_select_line_style(move |index| {
        if let Some(style) = LINES.get(index as usize).copied() {
            update(&weak, &controller, |c| c.set_line_style(style));
        }
    });
    let weak = ui.as_weak();
    ui.on_select_fill_pattern(move |index| {
        if let Some(pattern) = FILLS.get(index as usize).copied() {
            update(&weak, &state, |c| c.set_fill_pattern(pattern));
        }
    });
}
