use hoshi_canvas_spike::create_app_with_panels;
use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, Model};

#[test]
fn history_window_tracks_rows_navigates_redo_and_preserves_pending_shapes() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, colors, history) = create_app_with_panels().unwrap();
    ui.show().unwrap();
    for (x, y) in [(10.0, 10.0), (100.0, 100.0)] {
        let start = state.borrow().viewport().to_logical(x, y);
        let end = state.borrow().viewport().to_logical(x + 30.0, y + 30.0);
        ui.invoke_pointer_down(start.0 as f32, start.1 as f32);
        ui.invoke_pointer_move(end.0 as f32, end.1 as f32);
        ui.invoke_pointer_up(end.0 as f32, end.1 as f32);
        ui.invoke_action(2);
    }
    ui.invoke_show_history();
    assert!(history.window().is_visible());
    assert_eq!(history.get_rows().row_count(), 3);
    assert!(history.get_rows().row_data(2).unwrap().current);
    assert!(history.get_status_text().contains("Undo: 2 · Redo: 0"));
    history.window().take_snapshot().unwrap();
    let oldest =
        ElementHandle::find_by_accessible_label(history.as_ref(), "Oldest available state")
            .next()
            .expect("oldest retained state row");
    oldest.mock_single_click(slint::platform::PointerEventButton::Left);
    assert_eq!(state.borrow().history_position(), 0);
    assert!(state.borrow().composite().iter().all(|p| p.a == 0));
    assert!(history.get_rows().row_data(0).unwrap().current);
    assert!(history.get_rows().row_data(2).unwrap().redo);
    history.invoke_redo();
    assert_eq!(state.borrow().history_position(), 1);
    assert_eq!(state.borrow().composite()[15 * 512 + 15].a, 255);
    assert_eq!(state.borrow().composite()[110 * 512 + 110].a, 0);
    history.invoke_placement_selected(1);
    assert!(ui.get_history_visible());
    assert_eq!(ui.get_history_placement(), 1);
    assert!(!history.window().is_visible());
    ui.invoke_history_jump(2);
    assert_eq!(state.borrow().history_position(), 2);
    assert_eq!(state.borrow().composite()[110 * 512 + 110].a, 255);
    let start = state.borrow().viewport().to_logical(200., 200.);
    let end = state.borrow().viewport().to_logical(240., 240.);
    ui.invoke_pointer_down(start.0 as f32, start.1 as f32);
    ui.invoke_pointer_move(end.0 as f32, end.1 as f32);
    ui.invoke_pointer_up(end.0 as f32, end.1 as f32);
    let shape = state.borrow().document().active_shape().unwrap();
    assert!(!ui.get_history_can_jump() && !history.get_can_jump());
    ui.invoke_history_jump(0);
    assert_eq!(state.borrow().document().active_shape(), Some(shape));
    assert_eq!(state.borrow().history_position(), 2);
    ui.invoke_action(2);
    assert_eq!(history.get_rows().row_count(), 4);
    ui.invoke_show_colors();
    ui.invoke_set_history_placement(0);
    assert!(colors.window().is_visible() && history.window().is_visible());
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::CloseRequested);
    while ui.get_file_prompt_visible() {
        ui.invoke_file_prompt_response(1);
    }
    assert!(!colors.window().is_visible() && !history.window().is_visible());
}
