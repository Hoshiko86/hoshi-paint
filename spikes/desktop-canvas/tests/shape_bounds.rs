use hoshi_canvas_spike::{AppWindow, create_app};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalSize};
fn key(ui: &AppWindow, text: impl Into<slint::SharedString>) {
    let text = text.into();
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}
fn type_value(ui: &AppWindow, label: &str, text: &str) {
    i_slint_backend_testing::ElementHandle::find_by_accessible_label(ui, label)
        .next()
        .unwrap()
        .mock_single_click(PointerEventButton::Left);
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    key(ui, "a");
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
    for character in text.chars() {
        key(ui, character.to_string());
    }
    key(ui, Key::Return);
}
#[test]
fn numeric_fields_apply_on_enter_preserve_edit_mode_and_track_mouse_edits() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state) = create_app().unwrap();
    ui.window().set_size(LogicalSize::new(1120., 760.));
    ui.show().unwrap();
    ui.window().take_snapshot().unwrap();
    ui.invoke_pointer_down(30., 40.);
    ui.invoke_pointer_move(110., 80.);
    ui.invoke_pointer_up(110., 80.);
    ui.window().take_snapshot().unwrap();
    assert_eq!(ui.get_shape_width_text(), "80");
    type_value(&ui, "Shape X position", "-42");
    assert_eq!(state.borrow().document().active_shape().unwrap().x, -42);
    assert_eq!(
        state.borrow().document().undo_len(),
        0,
        "Enter in a field must not commit the shape"
    );
    type_value(&ui, "Shape Y position", "55");
    assert_eq!(state.borrow().document().active_shape().unwrap().y, 55);
    i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        &ui,
        "Lock shape aspect ratio",
    )
    .next()
    .unwrap()
    .mock_single_click(PointerEventButton::Left);
    type_value(&ui, "Shape width", "83");
    let rounded = state.borrow().document().active_shape();
    i_slint_backend_testing::ElementHandle::find_by_accessible_label(&ui, "Shape height")
        .next()
        .unwrap()
        .mock_single_click(PointerEventButton::Left);
    key(&ui, Key::Tab);
    assert_eq!(
        state.borrow().document().active_shape(),
        rounded,
        "tabbing through an untouched field must not round its partner again"
    );
    type_value(&ui, "Shape width", "200");
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!((shape.width, shape.height), (200, 100));
    type_value(&ui, "Shape height", "0");
    assert_eq!(state.borrow().document().active_shape(), Some(shape));
    type_value(&ui, "Shape height", "60");
    assert_eq!(state.borrow().document().active_shape().unwrap().width, 120);
    ui.invoke_edit_shape_bound(0, "16777217".into(), false);
    assert_eq!(
        state.borrow().document().active_shape().unwrap().x,
        16777217,
        "coordinates must not lose precision through f32"
    );
    ui.invoke_edit_shape_bound(0, "20".into(), false);
    ui.invoke_action(3);
    assert_eq!(ui.get_shape_width_text(), "");
    ui.invoke_edit_shape_bound(2, "500".into(), false);
    assert!(state.borrow().document().active_shape().is_none());
    ui.hide().unwrap();
}
