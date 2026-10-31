use hoshi_canvas_spike::create_app;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

#[test]
fn software_rendered_canvas_accepts_real_input_events() {
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
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    let initial = ui.window().take_snapshot().unwrap();
    assert_eq!((initial.width(), initial.height()), (1120, 760));
    let x = ui.get_canvas_left();
    let y = ui.get_canvas_top();
    let pointer = |dx, dy| LogicalPosition::new(x + dx, y + dy);
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: pointer(20.0, 20.0),
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: pointer(100.0, 80.0),
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position: pointer(100.0, 80.0),
        button: PointerEventButton::Left,
    });
    assert!(ui.get_has_shape());
    assert_eq!(state.borrow().document().undo_len(), 0);
    assert!(
        state.borrow().document().layers()[0]
            .pixels()
            .iter()
            .all(|p| p.a == 0)
    );
    let preview = ui.window().take_snapshot().unwrap();
    let sample =
        preview.as_slice()[((y as usize + 40) * preview.width() as usize) + x as usize + 40];
    assert_eq!(
        (sample.r, sample.g, sample.b, sample.a),
        (72, 136, 240, 255)
    );
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(false));
    assert!(
        ui.get_has_shape(),
        "a settled editable shape survives window deactivation"
    );
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Return.into(),
    });
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Return.into(),
    });
    assert!(!ui.get_has_shape());
    assert_eq!(state.borrow().document().undo_len(), 1);
    let committed = state.borrow().composite();
    ui.invoke_action(0);
    assert!(state.borrow().composite().iter().all(|p| p.a == 0));
    ui.invoke_action(1);
    assert_eq!(state.borrow().composite(), committed);

    let text_point = LogicalPosition::new(
        ui.get_layer_input_left() + 20.0,
        ui.get_layer_input_top() + 15.0,
    );
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: text_point,
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position: text_point,
        button: PointerEventButton::Left,
    });
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: "z".into() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: "z".into() });
    assert!(state.borrow().document().layers()[0].name().contains('z'));
    assert_eq!(
        state.borrow().composite(),
        committed,
        "text editing must not modify canvas pixels"
    );

    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: pointer(150.0, 150.0),
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: pointer(200.0, 200.0),
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position: pointer(200.0, 200.0),
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Escape.into(),
    });
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Escape.into(),
    });
    assert!(!ui.get_has_shape());
    assert_eq!(state.borrow().composite(), committed);
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: pointer(210.0, 210.0),
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: pointer(250.0, 250.0),
    });
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(false));
    assert!(!state.borrow().is_dragging());
    assert!(!ui.get_has_shape());
    assert_eq!(state.borrow().composite(), committed);

    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    ui.invoke_show_about();
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: pointer(30.0, 30.0),
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: pointer(90.0, 90.0),
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position: pointer(90.0, 90.0),
        button: PointerEventButton::Left,
    });
    assert!(
        !ui.get_has_shape(),
        "modal backdrop must not leak drawing input"
    );
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Escape.into(),
    });
    ui.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Escape.into(),
    });
    assert_eq!(state.borrow().composite(), committed);

    if let Some(path) = std::env::var_os("HOSHI_SPIKE_SNAPSHOT") {
        let snapshot = ui.window().take_snapshot().unwrap();
        image::save_buffer(
            path,
            snapshot.as_bytes(),
            snapshot.width(),
            snapshot.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    ui.hide().unwrap();
}
