use hoshi_controller::{
    Controller, ControllerError, EditCursor, InputModifiers, PIVOT_HANDLE, PointerButton,
    ROTATION_HANDLE, Tool,
};
use hoshi_core::ShapeKind;

/// A 100x60 rectangle from (40, 40), left editable. At the default viewport
/// logical and document coordinates coincide.
fn editable() -> Controller {
    let mut c = Controller::new(256, 256).unwrap();
    c.pointer_down(40., 40.).unwrap();
    c.pointer_move(140., 100.).unwrap();
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

fn rotation(c: &Controller) -> i32 {
    c.document().active_shape().unwrap().rotation_degrees
}

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

#[test]
fn hover_cursor_reports_each_region_of_an_editable_shape() {
    let c = Controller::new(256, 256).unwrap();
    assert_eq!(c.hover_cursor(90., 70.), EditCursor::Draw, "no shape");
    let c = editable();
    assert_eq!(c.hover_cursor(90., 80.), EditCursor::Move, "inside");
    assert_eq!(
        c.hover_cursor(150., 80.),
        EditCursor::Rotate,
        "just outside"
    );
    assert_eq!(c.hover_cursor(220., 220.), EditCursor::Draw, "far away");
    let (x, y) = handle(&c, ROTATION_HANDLE);
    assert_eq!(c.hover_cursor(x, y), EditCursor::Rotate);
    let (x, y) = handle(&c, PIVOT_HANDLE);
    assert_eq!(c.hover_cursor(x, y), EditCursor::Pivot);
    // ResizeHandle order: TopLeft, Top, TopRight, Right, BottomRight, Bottom,
    // BottomLeft, Left.
    let expected = [
        EditCursor::ResizeDiagonalBackward,
        EditCursor::ResizeVertical,
        EditCursor::ResizeDiagonalForward,
        EditCursor::ResizeHorizontal,
        EditCursor::ResizeDiagonalBackward,
        EditCursor::ResizeVertical,
        EditCursor::ResizeDiagonalForward,
        EditCursor::ResizeHorizontal,
    ];
    for (kind, cursor) in expected.into_iter().enumerate() {
        let (x, y) = handle(&c, kind as i32);
        assert_eq!(c.hover_cursor(x, y), cursor, "handle {kind}");
    }
}

#[test]
fn resize_cursors_follow_the_shape_rotation() {
    let mut c = editable();
    c.set_shape_rotation(90).unwrap();
    let (x, y) = handle(&c, 3);
    assert_eq!(c.hover_cursor(x, y), EditCursor::ResizeVertical, "Right");
    let (x, y) = handle(&c, 1);
    assert_eq!(c.hover_cursor(x, y), EditCursor::ResizeHorizontal, "Top");
}

#[test]
fn hover_cursor_is_draw_for_tools_that_cannot_edit_shapes() {
    let mut c = editable();
    for tool in [Tool::Pan, Tool::Eyedropper, Tool::Brush, Tool::Eraser] {
        c.set_tool(tool).unwrap();
        assert_eq!(c.hover_cursor(90., 80.), EditCursor::Draw, "{tool:?}");
    }
    c.set_tool(Tool::MoveShape).unwrap();
    assert_eq!(c.hover_cursor(90., 80.), EditCursor::Move);
}

#[test]
fn active_rotate_and_pivot_drags_keep_their_cursor_anywhere() {
    let mut c = editable();
    let (x, y) = handle(&c, ROTATION_HANDLE);
    c.pointer_down(x, y).unwrap();
    assert_eq!(c.hover_cursor(220., 220.), EditCursor::Rotate);
    c.pointer_up().unwrap();
    let (x, y) = handle(&c, PIVOT_HANDLE);
    c.pointer_down(x, y).unwrap();
    assert_eq!(c.hover_cursor(220., 220.), EditCursor::Pivot);
}

#[test]
fn secondary_drag_rotates_and_shift_snaps_to_15_degrees() {
    let mut c = editable();
    // Pivot is the centre (90, 70); start due east of it.
    c.pointer_down_with_button(130., 70., PointerButton::Secondary)
        .unwrap();
    c.pointer_move(90., 110.).unwrap();
    assert_eq!(rotation(&c), 90);
    c.set_input_modifiers(InputModifiers {
        shift: true,
        ..Default::default()
    })
    .unwrap();
    c.pointer_move(130., 59.).unwrap(); // about -15.4 degrees
    assert_eq!(rotation(&c).rem_euclid(15), 0, "{}", rotation(&c));
    c.pointer_up().unwrap();
    assert_eq!(c.document().undo_len(), 0, "rotation stays editable");
}

#[test]
fn arrow_keys_with_secondary_held_rotate_by_one_or_fifteen_degrees() {
    let mut c = editable();
    // Right button held east of the pivot (90, 70): a rotate drag at 0 degrees.
    c.pointer_down_with_button(130., 70., PointerButton::Secondary)
        .unwrap();
    c.keyboard_transform(1, 0, true, false).unwrap();
    assert_eq!(rotation(&c), 1);
    c.keyboard_transform(0, 1, true, false).unwrap();
    assert_eq!(rotation(&c), 2);
    // Shift replays the held drag with snapping: 2 degrees snaps to 0.
    c.set_aspect_lock(true).unwrap();
    assert_eq!(rotation(&c), 0);
    c.keyboard_transform(1, 0, true, false).unwrap();
    assert_eq!(rotation(&c), 15);
    // The key step re-anchors the drag, so a still pointer keeps the new angle.
    c.pointer_move(130., 70.).unwrap();
    assert_eq!(rotation(&c), 15);
    c.pointer_up().unwrap();
    assert_eq!(c.document().undo_len(), 0);
}

#[test]
fn keyboard_nudge_during_move_rebases_the_drag_anchor() {
    let mut c = editable();
    // Inside the frame but away from the centre pivot handle.
    c.pointer_down(60., 55.).unwrap();
    c.keyboard_transform(0, 1, false, true).unwrap();
    let nudged = c.document().active_shape().unwrap();
    assert_eq!((nudged.x, nudged.y), (40, 50));
    c.pointer_move(70., 55.).unwrap();
    let moved = c.document().active_shape().unwrap();
    assert_eq!(
        (moved.x, moved.y),
        (50, 50),
        "drag continues from the nudge"
    );
}

#[test]
fn keyboard_transform_without_a_shape_changes_nothing() {
    let mut c = Controller::new(64, 64).unwrap();
    c.keyboard_transform(1, 1, false, false).unwrap();
    c.keyboard_transform(1, 1, true, false).unwrap();
    assert!(c.document().active_shape().is_none());
    assert_eq!(c.document().undo_len(), 0);
}

#[test]
fn shape_kind_switch_restyles_the_editable_shape_in_place() {
    let mut c = editable();
    let before = c.document().active_shape().unwrap();
    c.set_shape_kind(ShapeKind::Ellipse).unwrap();
    let after = c.document().active_shape().unwrap();
    assert_eq!(after.kind, ShapeKind::Ellipse);
    assert_eq!(
        (after.x, after.y, after.width, after.height),
        (before.x, before.y, before.width, before.height)
    );
    assert_eq!(c.document().undo_len(), 0);
}

#[test]
fn unchanged_modifiers_do_not_replay_the_pointer() {
    let mut c = Controller::new(256, 256).unwrap();
    c.pointer_down(100., 100.).unwrap();
    c.pointer_move(140., 120.).unwrap();
    let shape = c.document().active_shape().unwrap();
    c.set_input_modifiers(InputModifiers::default()).unwrap();
    assert_eq!(c.document().active_shape().unwrap(), shape);
}

#[test]
fn move_tool_without_a_shape_reports_no_active_shape() {
    let mut c = Controller::new(64, 64).unwrap();
    c.set_tool(Tool::MoveShape).unwrap();
    assert_eq!(
        c.pointer_down(10., 10.),
        Err(ControllerError::NoActiveShape)
    );
    assert!(!c.is_dragging());
}
