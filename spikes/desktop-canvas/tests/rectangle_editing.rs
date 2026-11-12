use hoshi_canvas_spike::{AppWindow, create_app, refresh_workspace};
use hoshi_controller::{ColorSlot, ShapeMode};
use hoshi_core::{Color, Shape, ShapeStyle};
use slint::platform::{Key, PointerEventButton as Button, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn point(ui: &AppWindow, x: f64, y: f64) -> LogicalPosition {
    LogicalPosition::new(
        ui.get_canvas_left() + x as f32,
        ui.get_canvas_top() + y as f32,
    )
}
fn down(ui: &AppWindow, x: f64, y: f64, button: Button) {
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: point(ui, x, y),
        button,
    });
}
fn moved(ui: &AppWindow, x: f64, y: f64) {
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: point(ui, x, y),
    });
}
fn up(ui: &AppWindow, x: f64, y: f64, button: Button) {
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
fn draw(ui: &AppWindow, left: f64, top: f64, right: f64, bottom: f64, button: Button) {
    down(ui, left, top, button);
    moved(ui, right, bottom);
    up(ui, right, bottom, button);
}
fn center(shape: Shape) -> (f64, f64) {
    (
        f64::from(shape.x) + f64::from(shape.width) / 2.,
        f64::from(shape.y) + f64::from(shape.height) / 2.,
    )
}
fn close(a: (f64, f64), b: (f64, f64)) {
    assert!(
        (a.0 - b.0).abs() <= 1. && (a.1 - b.1).abs() <= 1.,
        "{a:?} != {b:?}"
    );
}

#[test]
fn shared_canvas_input_supports_shape_modifiers_pivot_keyboard_and_secondary_drawing() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state) = create_app().unwrap();
    ui.window().set_size(LogicalSize::new(1120., 850.));
    ui.show().unwrap();
    ui.window().take_snapshot().unwrap();
    draw(&ui, 20., 20., 100., 60., Button::Left);
    let original = state.borrow().document().active_shape().unwrap();
    assert_eq!((original.width, original.height), (80, 40));
    assert_eq!(state.borrow().handle_positions().len(), 10);
    let edge = state.borrow().handle_positions()[1];
    let anchor = state.borrow().handle_positions()[5];
    down(&ui, edge.x, edge.y, Button::Left);
    key(&ui, Key::Shift, true);
    moved(&ui, edge.x, edge.y - 10.);
    up(&ui, edge.x, edge.y - 10., Button::Left);
    key(&ui, Key::Shift, false);
    let resized = state.borrow().document().active_shape().unwrap();
    assert!((f64::from(resized.width) / f64::from(resized.height) - 2.).abs() < 0.03);
    let opposite = state.borrow().handle_positions()[5];
    close((opposite.x, opposite.y), (anchor.x, anchor.y));
    // Pressing/releasing Alt during a drag recomputes against its original frame.
    let original_center = center(resized);
    let edge = state.borrow().handle_positions()[3];
    down(&ui, edge.x, edge.y, Button::Left);
    moved(&ui, edge.x + 20., edge.y);
    assert!(center(state.borrow().document().active_shape().unwrap()).0 > original_center.0);
    key(&ui, Key::Alt, true);
    close(
        center(state.borrow().document().active_shape().unwrap()),
        original_center,
    );
    key(&ui, Key::Shift, true);
    let constrained = state.borrow().document().active_shape().unwrap();
    close(center(constrained), original_center);
    assert!((f64::from(constrained.width) / f64::from(constrained.height) - 2.).abs() < 0.03);
    key(&ui, Key::Shift, false);
    key(&ui, Key::Alt, false);
    up(&ui, edge.x + 20., edge.y, Button::Left);
    // Arrow commands are editor commands only while the canvas owns focus.
    let before = state.borrow().document().active_shape().unwrap();
    key(&ui, Key::RightArrow, true);
    key(&ui, Key::RightArrow, false);
    assert_eq!(
        state.borrow().document().active_shape().unwrap().x,
        before.x + 1
    );
    key(&ui, Key::Control, true);
    key(&ui, Key::DownArrow, true);
    key(&ui, Key::DownArrow, false);
    key(&ui, Key::Control, false);
    assert_eq!(
        state.borrow().document().active_shape().unwrap().y,
        before.y + 10
    );
    ui.invoke_action(3);
    draw(&ui, 100., 100., 180., 140., Button::Left);
    let corner = state.borrow().handle_positions()[4];
    down(&ui, corner.x, corner.y, Button::Left);
    moved(&ui, 80., 80.);
    up(&ui, 80., 80., Button::Left);
    let flipped = state.borrow().document().active_shape().unwrap();
    assert!(flipped.flip_horizontal && flipped.flip_vertical);
    assert_eq!(
        (flipped.x, flipped.y, flipped.width, flipped.height),
        (80, 80, 20, 20)
    );
    let pivot = state.borrow().handle_positions()[9];
    down(&ui, pivot.x, pivot.y, Button::Left);
    moved(&ui, 250., 250.);
    up(&ui, 250., 250., Button::Left);
    assert_eq!(state.borrow().document().active_shape(), Some(flipped));
    let pivot = state.borrow().handle_positions()[9];
    close((pivot.x, pivot.y), (250., 250.));
    let initial = center(flipped);
    let angle = 10_f64.to_radians();
    let delta = (initial.0 - 250., initial.1 - 250.);
    let destination = (
        250. + delta.0 * angle.cos() - delta.1 * angle.sin(),
        250. + delta.0 * angle.sin() + delta.1 * angle.cos(),
    );
    down(&ui, initial.0, initial.1, Button::Right);
    moved(&ui, destination.0, destination.1);
    key(&ui, Key::Shift, true);
    assert_eq!(
        state
            .borrow()
            .document()
            .active_shape()
            .unwrap()
            .rotation_degrees,
        15
    );
    up(&ui, destination.0, destination.1, Button::Right);
    key(&ui, Key::Shift, false);
    close(
        (
            state.borrow().handle_positions()[9].x,
            state.borrow().handle_positions()[9].y,
        ),
        (250., 250.),
    );
    // Keyboard rotation while holding the right button survives its release.
    let current = center(state.borrow().document().active_shape().unwrap());
    down(&ui, current.0, current.1, Button::Right);
    key(&ui, Key::RightArrow, true);
    key(&ui, Key::RightArrow, false);
    up(&ui, current.0, current.1, Button::Right);
    assert_eq!(
        state
            .borrow()
            .document()
            .active_shape()
            .unwrap()
            .rotation_degrees,
        16
    );
    // Logical coordinates remain stable across monitor scale-factor changes.
    for scale_factor in [1.25, 2., 1.] {
        ui.window()
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor });
        let current = center(state.borrow().document().active_shape().unwrap());
        down(&ui, current.0, current.1, Button::Right);
        key(&ui, Key::RightArrow, true);
        key(&ui, Key::RightArrow, false);
        up(&ui, current.0, current.1, Button::Right);
    }
    assert_eq!(
        state
            .borrow()
            .document()
            .active_shape()
            .unwrap()
            .rotation_degrees,
        19
    );
    // Switching tabs commits the preview and clears its transient controls.
    let first = state.borrow().active_id().unwrap();
    state
        .borrow_mut()
        .new_image(512, 512, 96., Color::TRANSPARENT)
        .unwrap();
    refresh_workspace(&ui, &state);
    assert!(state.borrow().handle_positions().is_empty());
    ui.invoke_activate_document(first as i32);
    assert!(state.borrow().document().active_shape().is_none());
    assert!(state.borrow().handle_positions().is_empty());
    // Secondary drawing reverses color roles without selecting a different slot.
    ui.invoke_action(3);
    draw(&ui, 300., 100., 360., 160., Button::Right);
    assert_eq!(state.borrow().color_slot(), ColorSlot::Primary);
    assert_eq!(
        state.borrow().document().active_shape().unwrap().style,
        ShapeStyle::Fill(Color::rgba(255, 255, 255, 255))
    );
    ui.invoke_select_shape_mode(2);
    assert_eq!(state.borrow().shape_mode(), ShapeMode::Both);
    assert_eq!(
        state.borrow().document().active_shape().unwrap().style,
        ShapeStyle::Both {
            fill: state.borrow().primary_color(),
            outline: state.borrow().secondary_color(),
            width: state.borrow().stroke_width_pixels()
        }
    );
    key(&ui, Key::Return, true);
    key(&ui, Key::Return, false);
    assert!(state.borrow().document().active_shape().is_none());
    // Typing-navigation keys must not reach shape commands through LineEdit.
    draw(&ui, 40., 40., 90., 90., Button::Left);
    let before = state.borrow().document().active_shape();
    let position = LogicalPosition::new(
        ui.get_layer_input_left() + 20.,
        ui.get_layer_input_top() + 10.,
    );
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: Button::Left,
    });
    ui.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: Button::Left,
    });
    key(&ui, Key::LeftArrow, true);
    key(&ui, Key::LeftArrow, false);
    assert_eq!(state.borrow().document().active_shape(), before);
    // Regaining canvas focus restores editor shortcuts; Escape commits.
    down(&ui, 55., 55., Button::Left);
    up(&ui, 55., 55., Button::Left);
    let undo_before = state.borrow().document().undo_len();
    let pixels_before = state.borrow().document().composite(false);
    key(&ui, Key::Escape, true);
    key(&ui, Key::Escape, false);
    assert!(state.borrow().document().active_shape().is_none());
    assert_eq!(state.borrow().document().undo_len(), undo_before + 1);
    assert_ne!(state.borrow().document().composite(false), pixels_before);
    assert!(state.borrow().handle_positions().is_empty());
    ui.hide().unwrap();
}
