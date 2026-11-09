use hoshi_controller::{BoundsField, Controller, PIVOT_HANDLE};
use hoshi_core::geometry::Point;
fn rectangle() -> Controller {
    let mut c = Controller::new(256, 256).unwrap();
    c.pointer_down(30., 40.).unwrap();
    c.pointer_move(110., 80.).unwrap();
    c.pointer_up().unwrap();
    c
}
#[test]
fn numeric_geometry_preserves_paints_rotation_and_one_commit_history() {
    let mut c = rectangle();
    c.set_shape_rotation(37).unwrap();
    let original = c.document().active_shape().unwrap();
    c.edit_shape_bound(BoundsField::X, -25, false).unwrap();
    c.edit_shape_bound(BoundsField::Y, 64, false).unwrap();
    c.edit_shape_bound(BoundsField::Width, 120, false).unwrap();
    c.edit_shape_bound(BoundsField::Height, 60, false).unwrap();
    let shape = c.document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (-25, 64, 120, 60)
    );
    assert_eq!(shape.rotation_degrees, 37);
    assert_eq!(shape.style, original.style);
    assert_eq!(shape.rendering, original.rendering);
    assert_eq!(c.document().undo_len(), 0);
    let preview = c.composite();
    c.commit().unwrap();
    assert_eq!(c.document().undo_len(), 1);
    c.undo().unwrap();
    c.redo().unwrap();
    assert_eq!(c.composite(), preview);
}
#[test]
fn locked_dimensions_round_atomically_and_invalid_edits_preserve_preview() {
    let mut c = rectangle();
    c.edit_shape_bound(BoundsField::Width, 160, true).unwrap();
    assert_eq!(c.document().active_shape().unwrap().height, 80);
    c.edit_shape_bound(BoundsField::Height, 30, true).unwrap();
    assert_eq!(c.document().active_shape().unwrap().width, 60);
    let before = c.document().active_shape();
    for (field, value) in [
        (BoundsField::Width, 0),
        (BoundsField::Height, -1),
        (BoundsField::Width, 262145),
        (BoundsField::X, i64::MAX),
        (BoundsField::Height, 262144),
    ] {
        assert!(c.edit_shape_bound(field, value, true).is_err());
        assert_eq!(c.document().active_shape(), before);
    }
}
#[test]
fn numeric_translation_moves_custom_pivot_and_stops_stale_drag() {
    let mut c = rectangle();
    let handle = c
        .handle_positions()
        .into_iter()
        .find(|p| p.kind == PIVOT_HANDLE)
        .unwrap();
    c.pointer_down(handle.x, handle.y).unwrap();
    c.pointer_move(140., 140.).unwrap();
    c.pointer_up().unwrap();
    let pivot = c.rotation_pivot().unwrap();
    c.pointer_down(110., 80.).unwrap();
    c.pointer_move(120., 90.).unwrap();
    c.edit_shape_bound(BoundsField::X, 50, false).unwrap();
    assert_eq!(c.rotation_pivot(), Some(pivot.translated(20., 0.)));
    let expected = c.document().active_shape();
    c.pointer_move(120., 90.).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(c.document().active_shape(), expected);
    let point = c.rotation_pivot().unwrap();
    c.edit_shape_bound(BoundsField::Width, 100, false).unwrap();
    assert_eq!(c.rotation_pivot(), Some(Point::new(point.x, point.y)));
}

#[test]
fn locked_ratio_does_not_drift_after_rounding_and_can_be_recaptured() {
    let mut c = rectangle();
    c.set_numeric_aspect_lock(true);
    c.edit_shape_bound(BoundsField::Width, 83, true).unwrap();
    assert_eq!(c.document().active_shape().unwrap().height, 42);
    c.edit_shape_bound(BoundsField::Height, 42, true).unwrap();
    assert_eq!(
        c.document().active_shape().unwrap().width,
        83,
        "submitting an untouched rounded dimension must be a no-op"
    );
    c.edit_shape_bound(BoundsField::Width, 160, true).unwrap();
    assert_eq!(c.document().active_shape().unwrap().height, 80);
    c.set_numeric_aspect_lock(false);
    c.edit_shape_bound(BoundsField::Height, 100, false).unwrap();
    c.set_numeric_aspect_lock(true);
    c.edit_shape_bound(BoundsField::Width, 80, true).unwrap();
    assert_eq!(c.document().active_shape().unwrap().height, 50);
}
