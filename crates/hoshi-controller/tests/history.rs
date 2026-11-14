use hoshi_controller::{Controller, ControllerError, HistoryLabel};

#[test]
fn history_navigation_requires_explicit_preview_completion() {
    let mut controller = Controller::new(16, 16).unwrap();
    controller.pointer_down(2.0, 2.0).unwrap();
    controller.pointer_move(8.0, 8.0).unwrap();
    let shape = controller.document().active_shape();
    assert_eq!(
        controller.go_to_history_step(0),
        Err(ControllerError::Core(hoshi_core::Error::ActiveShape))
    );
    assert_eq!(controller.document().active_shape(), shape);
    assert!(
        controller.is_dragging(),
        "rejected navigation must not mutate the active gesture"
    );
    controller.pointer_up().unwrap();
    controller.commit().unwrap();
    assert_eq!(controller.history_position(), 1);
    assert_eq!(controller.history_steps()[1].label, HistoryLabel::Rectangle);
    assert!(controller.go_to_history_step(0).unwrap());
    assert_eq!(controller.document().composite(false)[3 * 16 + 3].a, 0);
    assert!(controller.go_to_history_step(1).unwrap());
    assert!(controller.document().composite(false)[3 * 16 + 3].a > 0);
}
