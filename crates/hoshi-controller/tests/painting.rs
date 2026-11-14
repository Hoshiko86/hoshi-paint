use hoshi_controller::{Color, Controller, EditCursor, PointerButton, Tool};

fn painter(tool: Tool) -> Controller {
    let mut c = Controller::new(32, 32).unwrap();
    c.set_tool(tool).unwrap();
    c.set_antialiasing(false).unwrap();
    c.set_brush_width(3.).unwrap();
    c
}

fn drag(c: &mut Controller) {
    c.pointer_down(4.5, 8.5).unwrap();
    for x in [8.5, 12.5, 20.5] {
        c.pointer_move(x, 8.5).unwrap();
    }
}

#[test]
fn stroke_preview_is_isolated_and_release_records_one_brush_entry() {
    let mut c = painter(Tool::Brush);
    drag(&mut c);
    assert_eq!(
        c.document().layers()[0].pixels().get(12, 8),
        Color::TRANSPARENT
    );
    assert_eq!(c.composite()[8 * 32 + 12], c.primary_color());
    assert_eq!(c.document().undo_len(), 0);
    c.pointer_up().unwrap();
    assert_eq!(c.document().undo_len(), 1);
    assert_eq!(c.history_steps().last().unwrap().label, "Brush");
    assert!(!c.is_dragging());
}

#[test]
fn outside_drag_keeps_unclipped_segments_and_release_commits() {
    let mut c = painter(Tool::Brush);
    c.pointer_down(4.5, 8.5).unwrap();
    c.pointer_move(1000., 8.5).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(
        c.document().composite(false)[8 * 32 + 31],
        c.primary_color()
    );
    assert_eq!(c.document().undo_len(), 1);
}

#[test]
fn secondary_button_uses_secondary_even_when_primary_is_selected() {
    let mut c = painter(Tool::Brush);
    let red = Color::rgba(255, 0, 0, 255);
    c.set_secondary_color(red).unwrap();
    c.pointer_down_with_button(4.5, 8.5, PointerButton::Secondary)
        .unwrap();
    c.pointer_move(20.5, 8.5).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(c.document().composite(false)[8 * 32 + 12], red);
}

#[test]
fn eraser_removes_opaque_background() {
    let mut c = painter(Tool::Eraser);
    c.new_image(32, 32, 96., Color::rgba(255, 255, 255, 255))
        .unwrap();
    c.set_tool(Tool::Eraser).unwrap();
    drag(&mut c);
    c.pointer_up().unwrap();
    assert_eq!(c.document().composite(false)[8 * 32 + 12].a, 0);
    assert_eq!(c.history_steps().last().unwrap().label, "Eraser");
}

#[test]
fn capture_loss_commits_once_and_resets_drag() {
    let mut c = painter(Tool::Brush);
    drag(&mut c);
    c.cancel_pointer().unwrap();
    c.cancel_pointer().unwrap();
    c.pointer_up().unwrap();
    assert_eq!(c.document().undo_len(), 1);
    assert_eq!(
        c.document().composite(false)[8 * 32 + 12],
        c.primary_color()
    );
    assert!(!c.document().has_active_stroke());
    assert!(!c.is_dragging());
}

#[test]
fn capture_loss_still_discards_shape_gesture() {
    let mut c = painter(Tool::Rectangle);
    c.pointer_down(4., 4.).unwrap();
    c.pointer_move(20., 20.).unwrap();
    c.cancel_pointer().unwrap();
    assert!(c.document().active_shape().is_none());
    assert_eq!(c.document().undo_len(), 0);
}

#[test]
fn brush_commits_existing_shape_then_its_own_entry() {
    let mut c = painter(Tool::Rectangle);
    c.pointer_down(4., 4.).unwrap();
    c.pointer_move(20., 20.).unwrap();
    c.pointer_up().unwrap();
    c.set_tool(Tool::Brush).unwrap();
    assert_eq!(c.hover_cursor(4., 4.), EditCursor::Draw);
    assert!(c.document().active_shape().is_some());
    c.pointer_down(24.5, 24.5).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(c.document().undo_len(), 2);
}

#[test]
fn undo_mid_drag_finishes_then_undoes_stroke() {
    let mut c = painter(Tool::Brush);
    drag(&mut c);
    assert!(c.undo().unwrap());
    assert_eq!(c.document().redo_len(), 1);
    assert!(!c.document().has_active_stroke());
    c.pointer_up().unwrap();
    assert_eq!(c.document().undo_len(), 0);
}

#[test]
fn width_is_shared_and_invalid_values_leave_it_unchanged() {
    let mut c = painter(Tool::Brush);
    c.set_brush_width(7.).unwrap();
    assert_eq!(c.stroke_width_pixels(), 7.);
    for width in [f64::NAN, f64::INFINITY, -1., 0., 0.5, 2001.] {
        assert!(c.set_brush_width(width).is_err());
        assert_eq!(c.brush_width(), 7.);
    }
    c.set_stroke_width_pixels(9.).unwrap();
    assert_eq!(c.brush_width(), 9.);
}

#[test]
fn dot_diameter_follows_shared_width() {
    for width in [1., 3., 7.] {
        let mut c = painter(Tool::Brush);
        c.set_brush_width(width).unwrap();
        c.pointer_down(16.5, 16.5).unwrap();
        c.pointer_up().unwrap();
        let pixels = c.document().composite(false);
        let diameter = (0..32).filter(|x| pixels[16 * 32 + x].a != 0).count();
        assert_eq!(diameter, width as usize);
    }
}

#[test]
fn tool_switch_commit_and_redo_finish_active_strokes() {
    let mut c = painter(Tool::Brush);
    drag(&mut c);
    c.set_tool(Tool::Pan).unwrap();
    assert_eq!(c.document().undo_len(), 1);
    c.set_tool(Tool::Brush).unwrap();
    c.pointer_down(24.5, 24.5).unwrap();
    assert!(c.commit().unwrap());
    c.pointer_down(28.5, 28.5).unwrap();
    assert!(!c.redo().unwrap());
    assert_eq!(c.document().undo_len(), 3);
    assert!(!c.document().has_active_stroke());
}

#[test]
fn capture_loss_reports_commit_failure_and_resets_drag() {
    // Budget reservations are global: isolate exhaustion from parallel tests.
    crate::support::run_isolated(
        "painting::capture_loss_reports_commit_failure_and_resets_drag",
        || {
            let mut c = painter(Tool::Brush);
            drag(&mut c);
            let reservation = hoshi_core::MemoryReservation::new(
                hoshi_core::memory_budget() - hoshi_core::memory_usage(),
            )
            .unwrap();
            assert_eq!(
                c.cancel_pointer(),
                Err(hoshi_controller::ControllerError::Core(
                    hoshi_core::Error::MemoryLimit
                ))
            );
            drop(reservation);
            assert_eq!(c.document().undo_len(), 0);
            assert!(!c.document().has_active_stroke());
            assert!(!c.is_dragging());
            assert_eq!(
                c.document().layers()[0].pixels().get(12, 8),
                Color::TRANSPARENT
            );
            c.cancel_pointer().unwrap();
            c.pointer_up().unwrap();
            assert_eq!(c.document().undo_len(), 0);
        },
    );
}

#[test]
#[should_panic(expected = "ran no test")]
fn isolation_helper_rejects_a_name_that_matches_no_test() {
    crate::support::run_isolated("no_such_test_in_this_binary", || {
        unreachable!("the parent never runs the body");
    });
}
