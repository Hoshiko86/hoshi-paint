use hoshi_controller::{Controller, InputModifiers, PIVOT_HANDLE};
use hoshi_core::ShapeKind;

#[test]
fn centered_creation_and_constraints_toggle_without_pointer_drift() {
    let mut c = Controller::new(256, 256).unwrap();
    c.pointer_down(100., 100.).unwrap();
    c.pointer_move(140., 120.).unwrap();
    c.set_input_modifiers(InputModifiers {
        alt: true,
        ..Default::default()
    })
    .unwrap();
    let shape = c.document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (60, 80, 80, 40)
    );
    c.set_input_modifiers(InputModifiers {
        alt: true,
        shift: true,
        ..Default::default()
    })
    .unwrap();
    let shape = c.document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (60, 60, 80, 80)
    );
    c.set_input_modifiers(InputModifiers::default()).unwrap();
    let shape = c.document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (100, 100, 40, 20)
    );
}

#[test]
fn ellipse_reuses_handles_pivot_and_keyboard_translation() {
    let mut c = Controller::new(256, 256).unwrap();
    c.set_shape_kind(ShapeKind::Ellipse).unwrap();
    c.pointer_down(40., 40.).unwrap();
    c.pointer_move(140., 100.).unwrap();
    c.pointer_up().unwrap();
    let handle = c
        .handle_positions()
        .into_iter()
        .find(|h| h.kind == PIVOT_HANDLE)
        .unwrap();
    let (x, y) = (handle.x, handle.y);
    c.pointer_down(x, y).unwrap();
    c.pointer_move(x + 20., y - 10.).unwrap();
    c.pointer_up().unwrap();
    let pivot = c.rotation_pivot().unwrap();
    assert_eq!(c.document().undo_len(), 0);
    c.keyboard_transform(1, -1, false, true).unwrap();
    assert_eq!(c.rotation_pivot().unwrap(), pivot.translated(10., -10.));
    assert_eq!(
        c.document().active_shape().unwrap().kind,
        ShapeKind::Ellipse
    );
    c.commit().unwrap();
    assert!(c.handle_positions().is_empty());
    assert_eq!(c.document().undo_len(), 1);
}

#[test]
fn keyboard_nudge_during_resize_survives_pointer_release() {
    let mut c = Controller::new(256, 256).unwrap();
    c.pointer_down(40., 40.).unwrap();
    c.pointer_move(140., 100.).unwrap();
    c.pointer_up().unwrap();
    c.pointer_down(140., 100.).unwrap();
    c.pointer_move(160., 110.).unwrap();
    c.keyboard_transform(1, 0, false, false).unwrap();
    let expected = c.document().active_shape().unwrap();
    c.pointer_move(160., 110.).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(c.document().active_shape().unwrap(), expected);
}
