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
    let mut controller = Controller::new(16, 16).unwrap();
    draw(&mut controller);
    controller.zoom_at(0.0, 0.0, 2.0).unwrap();
    controller.set_tool(Tool::MoveShape).unwrap();
    controller.pointer_down(4.0, 6.0).unwrap();
    controller.pointer_move(10.0, 10.0).unwrap();
    controller.pointer_up().unwrap();
    let shape = controller.document().active_shape().unwrap();
    assert_eq!((shape.x, shape.y, shape.width, shape.height), (5, 5, 5, 6));
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
    for (x, y) in [(-1.0, 1.0), (1.0, -1.0), (16.0, 1.0), (1.0, 16.0)] {
        controller.pointer_down(x, y).unwrap();
        controller.pointer_move(5.0, 6.0).unwrap();
        controller.pointer_up().unwrap();
        assert_eq!(controller.document().active_shape(), shape);
        assert_eq!(controller.document().undo_len(), 0);
    }
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
