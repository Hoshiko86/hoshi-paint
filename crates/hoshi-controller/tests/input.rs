use hoshi_controller::{Controller, ControllerError, Tool};
use hoshi_core::Color;

fn draw(controller: &mut Controller) {
    controller.pointer_down(2.0, 3.0).unwrap();
    controller.pointer_move(7.0, 9.0).unwrap();
    controller.pointer_up().unwrap();
}

#[test]
fn release_leaves_adjustable_preview_and_commit_is_one_undo() {
    let mut controller = Controller::new(16, 16).unwrap();
    draw(&mut controller);
    assert_eq!(controller.document().undo_len(), 0);
    assert!(
        controller.document().layers()[0]
            .pixels()
            .iter()
            .all(|pixel| pixel.a == 0)
    );
    controller.nudge_shape(1, -1).unwrap();
    controller.set_shape_bounds(3, 2, 5, 6).unwrap();
    let preview = controller.composite();
    assert!(controller.commit().unwrap());
    assert_eq!(controller.document().undo_len(), 1);
    assert_eq!(controller.composite(), preview);
    controller.undo().unwrap();
    assert!(controller.composite().iter().all(|pixel| pixel.a == 0));
    controller.redo().unwrap();
    assert_eq!(controller.composite(), preview);
}

#[test]
fn zoom_is_pointer_anchored_and_physical_scale_is_applied_once() {
    let mut controller = Controller::new(16, 16).unwrap();
    controller.pan(23.0, -7.0).unwrap();
    let before = controller.viewport().to_document(101.0, 80.0);
    controller.zoom_at(101.0, 80.0, 2.5).unwrap();
    let after = controller.viewport().to_document(101.0, 80.0);
    assert!((before.0 - after.0).abs() < 1e-10);
    assert!((before.1 - after.1).abs() < 1e-10);
    assert_eq!(
        controller
            .viewport()
            .physical_to_document(202.0, 160.0, 2.0),
        Some(after)
    );
    let logical = controller.viewport().to_logical(after.0, after.1);
    assert_eq!(logical, (101.0, 80.0));
    assert_eq!(
        controller.viewport().physical_to_document(1.0, 2.0, 0.0),
        None
    );
}

#[test]
fn pan_and_zoom_do_not_modify_pixels_or_active_shape() {
    let mut controller = Controller::new(16, 16).unwrap();
    draw(&mut controller);
    let before = controller.composite();
    let shape = controller.document().active_shape();
    controller.set_tool(Tool::Pan).unwrap();
    controller.pointer_down(10.0, 10.0).unwrap();
    controller.pointer_move(14.0, 8.0).unwrap();
    controller.pointer_up().unwrap();
    assert_eq!(
        (
            controller.viewport().offset_x,
            controller.viewport().offset_y
        ),
        (4.0, -2.0)
    );
    controller.zoom_at(10.0, 10.0, 2.0).unwrap();
    assert_eq!(controller.composite(), before);
    assert_eq!(controller.document().active_shape(), shape);
    assert_eq!(controller.document().undo_len(), 0);
}

#[test]
fn move_drag_uses_document_coordinates_after_zoom() {
    let mut controller = Controller::new(128, 128).unwrap();
    controller.pointer_down(20.0, 20.0).unwrap();
    controller.pointer_move(80.0, 80.0).unwrap();
    controller.pointer_up().unwrap();
    controller.zoom_at(0.0, 0.0, 2.0).unwrap();
    controller.set_tool(Tool::MoveShape).unwrap();
    // Grab the interior, away from the new corner/edge control points.
    controller.pointer_down(100.0, 100.0).unwrap();
    controller.pointer_move(106.0, 104.0).unwrap();
    controller.pointer_up().unwrap();
    let shape = controller.document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (23, 22, 60, 60)
    );
}

#[test]
fn focus_loss_cancels_drag_and_preserves_committed_work() {
    let mut controller = Controller::new(16, 16).unwrap();
    draw(&mut controller);
    controller.commit().unwrap();
    let before = controller.composite();
    controller.pointer_down(1.0, 1.0).unwrap();
    controller.pointer_move(12.0, 12.0).unwrap();
    controller.cancel_pointer();
    assert!(!controller.is_dragging());
    assert!(controller.document().active_shape().is_none());
    assert_eq!(controller.document().undo_len(), 1);
    assert_eq!(controller.composite(), before);
    controller.pointer_move(15.0, 15.0).unwrap();
    assert_eq!(controller.composite(), before);
}

#[test]
fn drawing_clips_outside_drag_and_rejects_nonfinite_input_without_mutation() {
    let mut controller = Controller::new(16, 16).unwrap();
    controller.pointer_down(0.0, 0.0).unwrap();
    controller.pointer_move(1000.0, 1000.0).unwrap();
    controller.pointer_up().unwrap();
    let shape = controller.document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (0, 0, 16, 16)
    );
    let before = controller.viewport();
    assert_eq!(
        controller.zoom_at(0.0, 0.0, f64::NAN),
        Err(ControllerError::InvalidInput)
    );
    assert_eq!(
        controller.pan(f64::INFINITY, 0.0),
        Err(ControllerError::InvalidInput)
    );
    assert_eq!(
        controller.pointer_move(f64::NAN, 0.0),
        Err(ControllerError::InvalidInput)
    );
    assert_eq!(controller.viewport(), before);
    assert_eq!(controller.document().active_shape(), Some(shape));
}

#[test]
fn checkerboard_is_opaque_without_changing_document_pixels() {
    let mut controller = Controller::new(16, 16).unwrap();
    let empty = controller.preview_rgba();
    assert_eq!(&empty[0..4], &[224, 224, 224, 255]);
    assert_eq!(&empty[32..36], &[192, 192, 192, 255]);
    controller
        .set_primary_color(Color::rgba(255, 0, 0, 128))
        .unwrap();
    controller.pointer_down(0.0, 0.0).unwrap();
    controller.pointer_move(2.0, 2.0).unwrap();
    controller.pointer_up().unwrap();
    assert_eq!(&controller.preview_rgba()[0..4], &[240, 112, 112, 255]);
    assert_eq!(controller.composite()[0], Color::rgba(255, 0, 0, 128));
    assert_eq!(
        controller.document().layers()[0].pixels()[0],
        Color::TRANSPARENT
    );
}

#[test]
fn color_and_layer_name_editing_follow_document_history() {
    let mut controller = Controller::new(16, 16).unwrap();
    draw(&mut controller);
    let red = Color::rgba(255, 0, 0, 255);
    controller.set_primary_color(red).unwrap();
    assert_eq!(controller.composite()[3 * 16 + 2], red);
    controller.rename_layer("Ink").unwrap();
    assert!(controller.document().active_shape().is_none());
    assert_eq!(controller.document().layers()[0].name(), "Ink");
    assert_eq!(controller.document().undo_len(), 2);
    controller.undo().unwrap();
    assert_eq!(controller.document().layers()[0].name(), "Background");
    assert_eq!(controller.composite()[3 * 16 + 2], red);
}

#[test]
fn click_and_zero_area_drag_do_not_create_pixels_or_history() {
    let mut controller = Controller::new(16, 16).unwrap();
    controller.pointer_down(2.0, 3.0).unwrap();
    controller.pointer_up().unwrap();
    assert!(controller.document().active_shape().is_none());
    assert!(!controller.commit().unwrap());
    assert_eq!(controller.document().undo_len(), 0);

    controller.pointer_down(2.0, 3.0).unwrap();
    controller.pointer_move(8.0, 9.0).unwrap();
    assert!(controller.document().active_shape().is_some());
    controller.pointer_move(2.0, 9.0).unwrap();
    controller.pointer_up().unwrap();
    assert!(controller.document().active_shape().is_none());
    assert!(!controller.commit().unwrap());
    assert_eq!(controller.document().undo_len(), 0);
}

#[test]
fn workspace_click_does_not_commit_or_replace_active_shape() {
    let mut controller = Controller::new(16, 16).unwrap();
    draw(&mut controller);
    let shape = controller.document().active_shape();
    // Nearby workspace points may now intentionally select outlying handles.
    for (x, y) in [(-1000.0, 1.0), (1.0, -1000.0), (1000.0, 1.0), (1.0, 1000.0)] {
        controller.pointer_down(x, y).unwrap();
        controller.pointer_move(5.0, 6.0).unwrap();
        controller.pointer_up().unwrap();
        assert_eq!(controller.document().active_shape(), shape);
        assert_eq!(controller.document().undo_len(), 0);
    }
}

fn large_controller() -> Controller {
    let mut c = Controller::new(256, 256).unwrap();
    c.pointer_down(20.0, 30.0).unwrap();
    c.pointer_move(100.0, 90.0).unwrap();
    c.pointer_up().unwrap();
    c
}

fn handle(c: &Controller, kind: i32) -> (f64, f64) {
    let h = c
        .handle_positions()
        .into_iter()
        .find(|h| h.kind == kind)
        .unwrap();
    (h.x, h.y)
}

#[test]
fn corners_and_edges_resize_preview_under_zoom_without_committing() {
    let mut c = large_controller();
    c.zoom_at(0.0, 0.0, 2.0).unwrap();
    c.pan(17.0, -8.0).unwrap();
    let fixed = handle(&c, 0);
    let br = handle(&c, 4);
    c.pointer_down(br.0, br.1).unwrap();
    c.pointer_move(br.0 + 20.0, br.1 + 40.0).unwrap();
    c.pointer_up().unwrap();
    let shape = c.document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (20, 30, 90, 80)
    );
    assert_eq!(handle(&c, 0), fixed);
    assert_eq!(c.document().undo_len(), 0);

    let right = handle(&c, 3);
    c.pointer_down(right.0 + 3.0, right.1 + 2.0).unwrap();
    // Grab offset does not cause the handle to jump; tangential motion is ignored.
    c.pointer_move(right.0 + 23.0, right.1 + 50.0).unwrap();
    c.pointer_up().unwrap();
    let shape = c.document().active_shape().unwrap();
    assert_eq!((shape.width, shape.height), (100, 80));
    assert_eq!(handle(&c, 0), fixed);
}

#[test]
fn all_eight_handles_resize_their_axes_and_preserve_opposite_points() {
    let signs = [
        (-1.0, -1.0),
        (0.0, -1.0),
        (1.0, -1.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.0, 1.0),
        (-1.0, 1.0),
        (-1.0, 0.0),
    ];
    for (kind, (sx, sy)) in signs.into_iter().enumerate() {
        let mut c = large_controller();
        let selected = handle(&c, kind as i32);
        let opposite = handle(&c, ((kind + 4) % 8) as i32);
        c.pointer_down(selected.0, selected.1).unwrap();
        c.pointer_move(selected.0 + sx * 20.0, selected.1 + sy * 30.0)
            .unwrap();
        c.pointer_up().unwrap();
        let shape = c.document().active_shape().unwrap();
        assert_eq!(shape.width, if sx == 0.0 { 80 } else { 100 });
        assert_eq!(shape.height, if sy == 0.0 { 60 } else { 90 });
        assert_eq!(handle(&c, ((kind + 4) % 8) as i32), opposite);
        assert_eq!(c.document().undo_len(), 0);
    }
}

#[test]
fn rotated_resize_keeps_the_opposite_anchor_fixed() {
    let mut c = large_controller();
    c.set_shape_rotation(37).unwrap();
    let fixed = handle(&c, 4);
    let corner = handle(&c, 0);
    c.pointer_down(corner.0, corner.1).unwrap();
    c.pointer_move(corner.0 - 30.0, corner.1 - 30.0).unwrap();
    c.pointer_up().unwrap();
    let after = handle(&c, 4);
    // Integer raster bounds permit at most half a pixel of rounding per axis.
    assert!((after.0 - fixed.0).hypot(after.1 - fixed.1) <= 0.71);
    assert_eq!(c.document().active_shape().unwrap().rotation_degrees, 37);
    let edge_fixed = handle(&c, 7);
    let edge = handle(&c, 3);
    let height = c.document().active_shape().unwrap().height;
    c.pointer_down(edge.0, edge.1).unwrap();
    c.pointer_move(edge.0 + 30.0, edge.1 + 20.0).unwrap();
    c.pointer_up().unwrap();
    let after = handle(&c, 7);
    assert!((after.0 - edge_fixed.0).hypot(after.1 - edge_fixed.1) <= 0.71);
    assert_eq!(c.document().active_shape().unwrap().height, height);
}

#[test]
fn rotation_handle_keeps_center_and_commits_as_one_undoable_action() {
    let mut c = large_controller();
    let knob = handle(&c, 8);
    assert_eq!(c.handle_positions().len(), 9);
    assert_eq!(knob, (60.0, 2.0));
    c.pointer_down(knob.0, knob.1).unwrap();
    c.pointer_move(118.0, 60.0).unwrap();
    c.pointer_up().unwrap();
    let shape = c.document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (20, 30, 80, 60)
    );
    assert_eq!(shape.rotation_degrees, 90);
    let knob = handle(&c, 8);
    assert!((knob.0 - 118.0).abs() < 1e-10 && (knob.1 - 60.0).abs() < 1e-10);
    let preview = c.composite();
    assert!(c.commit().unwrap());
    assert!(c.handle_positions().is_empty());
    assert_eq!(c.document().undo_len(), 1);
    assert_eq!(c.composite(), preview);
    c.undo().unwrap();
    assert!(c.composite().iter().all(|p| p.a == 0));
    c.redo().unwrap();
    assert_eq!(c.composite(), preview);
}

#[test]
fn rotation_wraps_without_jumping_and_ignores_the_undefined_center_angle() {
    let mut c = large_controller();
    c.set_shape_rotation(359).unwrap();
    let knob = handle(&c, 8);
    c.pointer_down(knob.0, knob.1).unwrap();
    c.pointer_move(60.0, 60.0).unwrap();
    assert_eq!(c.document().active_shape().unwrap().rotation_degrees, 359);
    let theta = (-89.0_f64).to_radians();
    c.pointer_move(60.0 + 58.0 * theta.cos(), 60.0 + 58.0 * theta.sin())
        .unwrap();
    c.pointer_up().unwrap();
    assert_eq!(c.document().active_shape().unwrap().rotation_degrees, 1);
}

#[test]
fn rotation_knob_offset_is_zoom_independent_and_pan_ignores_handles() {
    let mut c = large_controller();
    c.zoom_at(0.0, 0.0, 3.0).unwrap();
    let top = handle(&c, 1);
    let knob = handle(&c, 8);
    assert!(((top.0 - knob.0).hypot(top.1 - knob.1) - 28.0).abs() < 1e-10);
    let before = c.document().active_shape();
    c.set_tool(Tool::Pan).unwrap();
    let corner = handle(&c, 0);
    c.pointer_down(corner.0, corner.1).unwrap();
    c.pointer_move(corner.0 + 15.0, corner.1 - 5.0).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(c.document().active_shape(), before);
    assert_eq!((c.viewport().offset_x, c.viewport().offset_y), (15.0, -5.0));
}

#[test]
fn crossing_an_opposite_edge_clamps_safely_and_cancel_discards_preview() {
    let mut c = large_controller();
    let edge = handle(&c, 3);
    c.pointer_down(edge.0, edge.1).unwrap();
    c.pointer_move(-500.0, edge.1).unwrap();
    c.pointer_up().unwrap();
    let shape = c.document().active_shape().unwrap();
    assert_eq!((shape.x, shape.width), (20, 1));
    c.cancel();
    assert!(c.document().active_shape().is_none());
    assert_eq!(c.document().undo_len(), 0);
    assert!(c.composite().iter().all(|p| p.a == 0));
}

#[test]
fn invalid_layer_name_does_not_commit_preview_and_names_are_bounded() {
    let mut controller = Controller::new(16, 16).unwrap();
    draw(&mut controller);
    let shape = controller.document().active_shape();
    for name in [" ".to_string(), "x".repeat(129)] {
        assert_eq!(
            controller.rename_layer(&name),
            Err(ControllerError::InvalidLayerName)
        );
        assert_eq!(controller.document().active_shape(), shape);
        assert_eq!(controller.document().undo_len(), 0);
    }
    let name = "絵".repeat(128);
    controller.rename_layer(&name).unwrap();
    assert_eq!(controller.document().layers()[0].name(), name);
}

#[test]
fn huge_finite_view_changes_are_rejected_before_float_conversion_for_ui() {
    let mut controller = Controller::new(16, 16).unwrap();
    let view = controller.viewport();
    assert_eq!(
        controller.pan(f64::MAX, 0.0),
        Err(ControllerError::InvalidInput)
    );
    assert_eq!(
        controller.zoom_at(f64::MAX / 2.0, 0.0, 2.0),
        Err(ControllerError::InvalidInput)
    );
    assert_eq!(controller.viewport(), view);
    controller.zoom_at(10.0, 10.0, f64::MAX).unwrap();
    assert_eq!(controller.viewport().zoom, 32.0);
    controller.zoom_at(10.0, 10.0, f64::MIN_POSITIVE).unwrap();
    assert_eq!(controller.viewport().zoom, 1.0 / 16.0);
}
