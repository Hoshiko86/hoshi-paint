use hoshi_canvas_spike::{AppWindow, create_app};
use hoshi_controller::Tool;
use slint::platform::{Key, PointerEventButton as Button, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn point(ui: &AppWindow, x: f32, y: f32) -> LogicalPosition {
    LogicalPosition::new(ui.get_canvas_left() + x, ui.get_canvas_top() + y)
}
fn down(ui: &AppWindow, x: f32, y: f32, button: Button) {
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: point(ui, x, y),
        button,
    });
}
fn moved(ui: &AppWindow, x: f32, y: f32) {
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: point(ui, x, y),
    });
}
fn up(ui: &AppWindow, x: f32, y: f32, button: Button) {
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position: point(ui, x, y),
        button,
    });
}
fn key(ui: &AppWindow, key: Key, pressed: bool) {
    ui.window().dispatch_event(if pressed {
        WindowEvent::KeyPressed { text: key.into() }
    } else {
        WindowEvent::KeyReleased { text: key.into() }
    });
}
fn wheel(ui: &AppWindow, x: f32, y: f32, dx: f32, dy: f32) {
    ui.window().dispatch_event(WindowEvent::PointerScrolled {
        position: point(ui, x, y),
        delta_x: dx,
        delta_y: dy,
    });
}

#[test]
fn temporary_pan_wheel_navigation_and_text_focus_preserve_editing() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state) = create_app().unwrap();
    ui.window().set_size(LogicalSize::new(1120.0, 760.0));
    ui.show().unwrap();
    ui.window().take_snapshot().unwrap();
    down(&ui, 40.0, 50.0, Button::Left);
    moved(&ui, 120.0, 90.0);
    up(&ui, 120.0, 90.0, Button::Left);
    let shape = state.borrow().document().active_shape().unwrap();
    let pixels = state.borrow().composite();
    for button in [Button::Middle, Button::Right] {
        let before = state.borrow().viewport();
        down(&ui, 200.0, 220.0, button);
        moved(&ui, 225.0, 235.0);
        down(&ui, 225.0, 235.0, Button::Left);
        up(&ui, 225.0, 235.0, Button::Left);
        assert!(
            state.borrow().is_dragging(),
            "unrelated button release must not end pan"
        );
        moved(&ui, 240.0, 250.0);
        up(&ui, 240.0, 250.0, button);
        assert!(!state.borrow().is_dragging());
        let after = state.borrow().viewport();
        assert_eq!(
            (
                after.offset_x - before.offset_x,
                after.offset_y - before.offset_y
            ),
            (40.0, 30.0)
        );
        moved(&ui, 250.0, 260.0);
        assert_eq!(state.borrow().viewport(), after);
    }
    key(&ui, Key::Space, true);
    down(&ui, 200.0, 220.0, Button::Left);
    moved(&ui, 180.0, 230.0);
    key(&ui, Key::Space, false);
    assert!(
        !state.borrow().is_dragging(),
        "Space release must end temporary pan"
    );
    let after_space = state.borrow().viewport();
    moved(&ui, 220.0, 250.0);
    up(&ui, 220.0, 250.0, Button::Left);
    assert_eq!(state.borrow().viewport(), after_space);
    assert_eq!(state.borrow().document().active_shape(), Some(shape));

    down(&ui, 200.0, 220.0, Button::Middle);
    moved(&ui, 210.0, 230.0);
    key(&ui, Key::Escape, true);
    key(&ui, Key::Escape, false);
    assert!(!state.borrow().is_dragging());
    assert_eq!(state.borrow().document().active_shape(), Some(shape));
    up(&ui, 210.0, 230.0, Button::Middle);
    down(&ui, 200.0, 220.0, Button::Right);
    moved(&ui, 210.0, 230.0);
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(false));
    assert!(!state.borrow().is_dragging());
    assert_eq!(state.borrow().document().active_shape(), Some(shape));
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    up(&ui, 210.0, 230.0, Button::Right);
    down(&ui, 200.0, 220.0, Button::Middle);
    moved(&ui, 210.0, 230.0);
    ui.window().dispatch_event(WindowEvent::PointerExited);
    assert!(!state.borrow().is_dragging());
    assert_eq!(state.borrow().document().active_shape(), Some(shape));

    let before_wheel = state.borrow().viewport();
    wheel(&ui, 250.0, 200.0, 0.0, 120.0);
    let after_wheel = state.borrow().viewport();
    assert_eq!(after_wheel.zoom, before_wheel.zoom);
    assert_eq!(after_wheel.offset_y - before_wheel.offset_y, 120.0);
    key(&ui, Key::Shift, true);
    wheel(&ui, 250.0, 200.0, 0.0, 120.0);
    key(&ui, Key::Shift, false);
    let horizontal = state.borrow().viewport();
    assert_eq!(horizontal.offset_x - after_wheel.offset_x, 120.0);
    assert_eq!(horizontal.offset_y, after_wheel.offset_y);
    let anchor = horizontal.to_document(250.0, 200.0);
    key(&ui, Key::Control, true);
    wheel(&ui, 250.0, 200.0, 0.0, 120.0);
    key(&ui, Key::Control, false);
    let zoomed = state.borrow().viewport();
    assert!(zoomed.zoom > horizontal.zoom);
    let new_anchor = zoomed.to_document(250.0, 200.0);
    assert!((anchor.0 - new_anchor.0).abs() < 1e-10 && (anchor.1 - new_anchor.1).abs() < 1e-10);
    assert_eq!(state.borrow().document().active_shape(), Some(shape));
    assert_eq!(state.borrow().composite(), pixels);
    assert_eq!(state.borrow().document().undo_len(), 0);
    assert_eq!(state.borrow().tool(), Tool::Rectangle);

    // The explicit Pan tool uses the same safe navigation path, but keeps its
    // selected-tool state when the gesture ends.
    ui.invoke_choose_tool(2);
    down(&ui, 200.0, 220.0, Button::Left);
    moved(&ui, 210.0, 230.0);
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(false));
    assert_eq!(state.borrow().tool(), Tool::Pan);
    assert_eq!(state.borrow().document().active_shape(), Some(shape));
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    up(&ui, 210.0, 230.0, Button::Left);
    ui.invoke_choose_tool(0);
    ui.invoke_action(2);
    let position = LogicalPosition::new(
        ui.get_layer_input_left() + 20.0,
        ui.get_layer_input_top() + 10.0,
    );
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: Button::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: Button::Left,
    });
    key(&ui, Key::Space, true);
    assert!(
        ui.get_layer_edit_text().contains(' '),
        "Space in a text field must remain text input"
    );
    ui.invoke_action(4);
    let before_draw = state.borrow().viewport();
    down(&ui, 200.0, 220.0, Button::Left);
    moved(&ui, 220.0, 240.0);
    up(&ui, 220.0, 240.0, Button::Left);
    key(&ui, Key::Space, false);
    assert_eq!(
        state.borrow().viewport(),
        before_draw,
        "text-field Space must not leak into canvas pan"
    );
    assert!(state.borrow().document().active_shape().is_some());
    ui.hide().unwrap();
}
