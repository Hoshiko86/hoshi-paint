use hoshi_canvas_spike::create_app;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

#[test]
fn shift_events_constrain_and_release_corner_resizing_live() {
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
    let point = |x, y| LogicalPosition::new(ui.get_canvas_left() + x, ui.get_canvas_top() + y);
    let press = |x, y| {
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: point(x, y),
            button: PointerEventButton::Left,
        })
    };
    let move_to = |x, y| {
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: point(x, y),
        })
    };
    let release = |x, y| {
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: point(x, y),
            button: PointerEventButton::Left,
        })
    };
    // Real keyboard events update the initial drag immediately, even while still.
    press(200.0, 200.0);
    move_to(280.0, 240.0);
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!((shape.width, shape.height), (80, 80));
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!((shape.width, shape.height), (80, 40));
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    release(280.0, 240.0);
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!((shape.width, shape.height), (80, 80));
    ui.invoke_action(3); // Cancel before testing existing-ratio resizing.
    press(40.0, 40.0);
    move_to(120.0, 80.0);
    release(120.0, 80.0);
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    press(120.0, 80.0);
    move_to(160.0, 80.0);
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!((shape.width, shape.height), (112, 56));
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!((shape.width, shape.height), (120, 40));
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    release(160.0, 80.0);
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!((shape.width, shape.height), (112, 56));
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.invoke_action(2);
    assert_eq!(state.borrow().document().undo_len(), 1);
    ui.hide().unwrap();
}
