use hoshi_canvas_spike::create_app_with_colors;
use hoshi_controller::Color;
use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn scroll(window: &slint::Window, delta: f32) {
    window.dispatch_event(slint::platform::WindowEvent::PointerScrolled {
        position: LogicalPosition::new(window.size().width as f32 - 30., 200.),
        delta_x: 0.,
        delta_y: delta,
    });
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(1000));
    window.take_snapshot().unwrap();
}

fn slint_color(c: Color) -> slint::Color {
    slint::Color::from_argb_u8(c.a, c.r, c.g, c.b)
}

#[test]
fn original_survives_live_edits_and_docking_and_restore_keeps_shape_editable() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, picker) = create_app_with_colors().unwrap();
    ui.show().unwrap();
    let original = Color::rgba(24, 80, 140, 96);
    picker.invoke_set_rgba(24, 80, 140, 96);
    ui.invoke_pointer_down(40.0, 40.0);
    ui.invoke_pointer_move(140.0, 100.0);
    ui.invoke_pointer_up(140.0, 100.0);
    ui.invoke_show_colors();
    let shape = state.borrow().document().active_shape().unwrap();
    assert_eq!(picker.get_original_color(), slint_color(original));
    picker.window().take_snapshot().unwrap();
    scroll(picker.window(), 440. - picker.get_red_slider_top());
    let position = LogicalPosition::new(
        picker.get_red_slider_left() + picker.get_red_slider_width() * 0.9,
        picker.get_red_slider_top() + picker.get_red_slider_height() * 0.5,
    );
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerPressed {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerReleased {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    assert_ne!(state.borrow().primary_color(), original);
    assert_eq!(picker.get_original_color(), slint_color(original));
    scroll(picker.window(), 10000.);
    let restore =
        ElementHandle::find_by_accessible_label(picker.as_ref(), "Restore original color")
            .next()
            .unwrap();
    assert_eq!(restore.accessible_enabled(), Some(true));
    restore.mock_single_click(slint::platform::PointerEventButton::Left);
    assert_eq!(state.borrow().primary_color(), original);
    assert_eq!(state.borrow().composite()[60 * 512 + 60], original);
    assert_eq!(restore.accessible_enabled(), Some(false));
    ui.invoke_set_colors_placement(1);
    picker.invoke_set_hsv(120.0, 100.0, 100.0);
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 255, 0, 96));
    assert_eq!(ui.get_color_original(), slint_color(original));
    ui.invoke_restore_original_color();
    assert_eq!(state.borrow().primary_color(), original);
    ui.invoke_set_colors_placement(0);
    assert_eq!(picker.get_original_color(), slint_color(original));
    assert_eq!(state.borrow().document().active_shape(), Some(shape));
    assert_eq!(state.borrow().document().undo_len(), 0);
    picker.invoke_close_picker();
    picker.invoke_set_rgba(1, 2, 3, 255);
    picker.set_hex_value("unapplied draft".into());
    ui.invoke_show_colors();
    assert_eq!(
        picker.get_original_color(),
        slint_color(Color::rgba(1, 2, 3, 255))
    );
    assert_eq!(picker.get_hex_value(), "#010203FF");
    picker.window().take_snapshot().unwrap();
    scroll(picker.window(), -10000.);
    let hex = ElementHandle::find_by_accessible_label(picker.as_ref(), "Hex RGB or RGBA color")
        .next()
        .unwrap();
    hex.mock_single_click(slint::platform::PointerEventButton::Left);
    picker.set_hex_value("#ABC".into());
    assert_eq!(state.borrow().primary_color(), Color::rgba(1, 2, 3, 255));
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::Return.into(),
        });
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: slint::platform::Key::Return.into(),
        });
    assert_eq!(
        state.borrow().primary_color(),
        Color::rgba(170, 187, 204, 255)
    );
    assert_eq!(
        picker.get_original_color(),
        slint_color(Color::rgba(1, 2, 3, 255))
    );
    // Narrow panels use one column, with readable aligned tracks rather than
    // collapsed handles that can be mistaken for separate radio buttons.
    picker.window().set_size(LogicalSize::new(380.0, 720.0));
    picker.window().take_snapshot().unwrap();
    assert!(picker.get_red_slider_width() > 170.0);
    assert_eq!(picker.get_red_slider_left(), picker.get_green_slider_left());
    assert_eq!(picker.get_red_slider_left(), picker.get_blue_slider_left());
    assert_eq!(
        picker.get_red_slider_width(),
        picker.get_green_slider_width()
    );
    assert_eq!(
        picker.get_red_slider_width(),
        picker.get_blue_slider_width()
    );
    picker.invoke_restore_original();
    assert_eq!(state.borrow().primary_color(), Color::rgba(1, 2, 3, 255));
    assert!(state.borrow().document().active_shape().is_some());
    assert_eq!(state.borrow().document().undo_len(), 0);
}
