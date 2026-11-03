use hoshi_canvas_spike::create_app;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize, Model};

fn drag(ui: &hoshi_canvas_spike::AppWindow, from: (f64, f64), to: (f64, f64)) {
    let point = |p: (f64, f64)| {
        LogicalPosition::new(
            ui.get_canvas_left() + p.0 as f32,
            ui.get_canvas_top() + p.1 as f32,
        )
    };
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: point(from),
        button: PointerEventButton::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: point(to),
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position: point(to),
        button: PointerEventButton::Left,
    });
}

#[test]
fn visible_handles_resize_rotate_and_commit_through_window_events() {
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
    drag(&ui, (80.0, 100.0), (240.0, 200.0));
    assert_eq!(ui.get_control_points().row_count(), 9);
    let corner = state.borrow().handle_positions()[4];
    drag(
        &ui,
        (corner.x, corner.y),
        (corner.x + 40.0, corner.y + 20.0),
    );
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (80, 100, 200, 120)
    );
    let edge = state.borrow().handle_positions()[3];
    drag(&ui, (edge.x, edge.y), (edge.x + 20.0, edge.y));
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!((shape.width, shape.height), (220, 120));
    let rotation = state.borrow().handle_positions()[8];
    let center = (190.0, 160.0);
    drag(&ui, (rotation.x, rotation.y), (center.0 + 88.0, center.1));
    assert_eq!(
        state
            .borrow()
            .document()
            .active_shape()
            .unwrap()
            .rotation_degrees,
        90
    );
    assert_eq!(state.borrow().document().undo_len(), 0);
    let preview = state.borrow().composite();
    let edge = state.borrow().handle_positions()[3];
    ui.invoke_pointer_down(edge.x as f32, edge.y as f32);
    ui.invoke_pointer_up(100_000_000.0, edge.y as f32);
    assert!(
        !state.borrow().is_dragging(),
        "rejected oversized resize must still end the gesture on release"
    );
    assert_eq!(state.borrow().composite(), preview);
    let snapshot = ui.window().take_snapshot().unwrap();
    if let Some(path) = std::env::var_os("HOSHI_HANDLES_SNAPSHOT") {
        image::save_buffer(
            path,
            snapshot.as_bytes(),
            snapshot.width(),
            snapshot.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    ui.invoke_action(2);
    assert_eq!(ui.get_control_points().row_count(), 0);
    assert_eq!(state.borrow().composite(), preview);
    assert_eq!(state.borrow().document().undo_len(), 1);
    ui.invoke_action(0);
    assert!(state.borrow().composite().iter().all(|p| p.a == 0));
    ui.invoke_action(1);
    assert_eq!(state.borrow().composite(), preview);
    ui.hide().unwrap();
}
