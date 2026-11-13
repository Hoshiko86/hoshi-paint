use hoshi_canvas_spike::{AppWindow, create_app, refresh_workspace};
use hoshi_controller::{Color, Tool};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn press(ui: &AppWindow) {
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(ui.get_canvas_left() + 30.0, ui.get_canvas_top() + 30.0),
        button: PointerEventButton::Left,
    });
}

fn release(ui: &AppWindow, position: LogicalPosition) {
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
}

#[test]
fn drawing_tools_preserve_strokes_and_document_color_alpha() {
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
    ui.invoke_choose_tool(4);
    assert_eq!(state.borrow().tool(), Tool::Brush);
    ui.invoke_edit_brush_width(20);
    assert_eq!(state.borrow().brush_width(), 20.0);
    assert_eq!(ui.get_brush_width(), 20);
    let before = state.borrow().composite();
    press(&ui);
    let outside = LogicalPosition::new(-30.0, -30.0);
    ui.window()
        .dispatch_event(WindowEvent::PointerMoved { position: outside });
    release(&ui, outside);
    assert_eq!(state.borrow().document().undo_len(), 1);
    assert_ne!(state.borrow().composite(), before);
    let painted = state.borrow().composite();
    ui.invoke_action(0);
    assert_eq!(state.borrow().composite(), before);
    ui.invoke_action(1);
    assert_eq!(state.borrow().composite(), painted);

    // PointerExited delivers TouchArea's cancel event during an active stroke.
    press(&ui);
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(ui.get_canvas_left() + 70.0, ui.get_canvas_top() + 70.0),
    });
    ui.window().dispatch_event(WindowEvent::PointerExited);
    assert_eq!(state.borrow().document().undo_len(), 2);
    assert!(!state.borrow().is_dragging());
    assert_ne!(state.borrow().composite(), painted);

    state
        .borrow_mut()
        .new_image(256, 256, 96.0, Color::rgba(80, 120, 160, 255))
        .unwrap();
    refresh_workspace(&ui, &state);
    ui.window().take_snapshot().unwrap();
    ui.invoke_choose_tool(5);
    assert_eq!(state.borrow().tool(), Tool::Eraser);
    press(&ui);
    release(
        &ui,
        LogicalPosition::new(ui.get_canvas_left() + 30.0, ui.get_canvas_top() + 30.0),
    );
    assert_eq!(state.borrow().document().undo_len(), 1);
    assert!(state.borrow().composite().iter().any(|pixel| pixel.a < 255));

    let sampled = Color::rgba(43, 91, 157, 103);
    state.borrow_mut().new_image(256, 256, 96.0, sampled).unwrap();
    refresh_workspace(&ui, &state);
    ui.window().take_snapshot().unwrap();
    ui.invoke_choose_tool(3);
    assert_eq!(state.borrow().tool(), Tool::Eyedropper);
    press(&ui);
    release(
        &ui,
        LogicalPosition::new(ui.get_canvas_left() + 30.0, ui.get_canvas_top() + 30.0),
    );
    assert_eq!(state.borrow().primary_color(), sampled);
    assert_eq!(state.borrow().document().undo_len(), 0);

    // Replace the OS boundary: this test checks the action's dispatch and state.
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = calls.clone();
    ui.on_pick_screen(move || observed.set(observed.get() + 1));
    ui.invoke_pick_screen();
    assert_eq!(calls.get(), 1);
    assert_eq!(state.borrow().tool(), Tool::Eyedropper);
    assert_eq!(state.borrow().document().undo_len(), 0);
    ui.hide().unwrap();
}
