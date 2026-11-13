//! Workspace rendering defaults (antialiasing, line style, fill pattern, stroke
//! width) and how they reach the live shape preview (#71).

use hoshi_controller::{
    Color, Controller, ControllerError, FillPattern, LineStyle, PointerButton, ShapeMode,
};
use hoshi_core::Error;

const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);

fn controller() -> Controller {
    let mut c = Controller::new(64, 64).unwrap();
    c.set_primary_color(RED).unwrap();
    c.set_secondary_color(BLUE).unwrap();
    c
}

fn draw(c: &mut Controller, button: PointerButton) {
    c.pointer_down_with_button(8., 8., button).unwrap();
    c.pointer_move(48., 40.).unwrap();
    c.pointer_up().unwrap();
}

#[test]
fn defaults_are_antialiased_solid_lines_and_solid_fill() {
    let c = controller();
    assert!(c.antialiasing());
    assert_eq!(c.line_style(), LineStyle::Solid);
    assert_eq!(c.fill_pattern(), FillPattern::Solid);
}

#[test]
fn settings_without_a_shape_apply_to_the_next_shape() {
    let mut c = controller();
    c.set_antialiasing(false).unwrap();
    c.set_line_style(LineStyle::DashDot).unwrap();
    c.set_fill_pattern(FillPattern::Checkerboard).unwrap();
    assert!(!c.antialiasing());
    assert_eq!(c.line_style(), LineStyle::DashDot);
    assert_eq!(c.fill_pattern(), FillPattern::Checkerboard);
    assert_eq!(c.document().undo_len(), 0);

    draw(&mut c, PointerButton::Primary);
    let rendering = c.document().active_shape().unwrap().rendering;
    assert!(!rendering.antialias);
    assert_eq!(rendering.line_style, LineStyle::DashDot);
    assert_eq!(rendering.fill_pattern, FillPattern::Checkerboard);
}

#[test]
fn settings_restyle_the_editable_shape_without_history_or_geometry_change() {
    let mut c = controller();
    draw(&mut c, PointerButton::Primary);
    let before = c.document().active_shape().unwrap();
    let preview = c.composite();

    c.set_line_style(LineStyle::Dash).unwrap();
    c.set_fill_pattern(FillPattern::Horizontal).unwrap();
    c.set_antialiasing(false).unwrap();

    let after = c.document().active_shape().unwrap();
    assert_eq!(after.rendering.line_style, LineStyle::Dash);
    assert_eq!(after.rendering.fill_pattern, FillPattern::Horizontal);
    assert!(!after.rendering.antialias);
    assert_eq!(
        (after.x, after.y, after.width, after.height),
        (before.x, before.y, before.width, before.height)
    );
    assert_ne!(c.composite(), preview, "the preview shows the new pattern");
    assert_eq!(c.document().undo_len(), 0, "style edits stay live");
    assert!(c.commit().unwrap());
    assert_eq!(c.document().undo_len(), 1);
}

#[test]
fn pattern_background_is_secondary_except_in_fill_and_outline_mode() {
    let mut c = controller();
    draw(&mut c, PointerButton::Primary);
    assert_eq!(
        c.document().active_shape().unwrap().rendering.pattern_color,
        BLUE
    );
    c.set_shape_mode(ShapeMode::Both).unwrap();
    assert_eq!(
        c.document().active_shape().unwrap().rendering.pattern_color,
        RED
    );
}

#[test]
fn secondary_button_shapes_swap_the_pattern_colors() {
    let mut c = controller();
    draw(&mut c, PointerButton::Secondary);
    assert_eq!(
        c.document().active_shape().unwrap().rendering.pattern_color,
        RED
    );
    c.set_shape_mode(ShapeMode::Both).unwrap();
    assert_eq!(
        c.document().active_shape().unwrap().rendering.pattern_color,
        BLUE
    );
}

#[test]
fn stroke_width_rejects_out_of_range_values_and_keeps_the_last_valid_one() {
    let mut c = controller();
    c.set_stroke_width_pixels(2.5).unwrap();
    for width in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.,
        0.001,
        -3.,
        65_537.,
    ] {
        assert_eq!(
            c.set_stroke_width_pixels(width),
            Err(ControllerError::Core(Error::InvalidShape)),
            "{width}"
        );
        assert_eq!(c.stroke_width_pixels(), 2.5);
    }
    c.set_stroke_width_pixels(65_536.).unwrap();
    assert_eq!(c.stroke_width_pixels(), 65_536.);
}
