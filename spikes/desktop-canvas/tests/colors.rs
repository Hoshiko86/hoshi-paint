use hoshi_canvas_spike::create_app_with_colors;
use hoshi_controller::{Color, Tool};
use slint::{ComponentHandle, LogicalSize};

#[test]
fn picker_controls_sync_live_color_alpha_and_canvas_sampling() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let (ui, state, picker) = create_app_with_colors().unwrap();
    ui.window().set_size(LogicalSize::new(1120.0, 760.0));
    ui.show().unwrap();
    ui.invoke_show_colors();
    picker.window().set_size(LogicalSize::new(640.0, 680.0));
    picker.invoke_set_rgba(12, 34, 56, 128);
    assert_eq!(state.borrow().primary_color(), Color::rgba(12, 34, 56, 128));
    assert_eq!(picker.get_hex_value(), "#0C223880");
    ui.invoke_pointer_down(40.0, 40.0);
    ui.invoke_pointer_move(100.0, 80.0);
    ui.invoke_pointer_up(100.0, 80.0);
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::WindowActiveChanged(false));
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::WindowActiveChanged(true));
    let before_click = state.borrow().primary_color();
    picker.window().take_snapshot().unwrap();
    let position = slint::LogicalPosition::new(
        picker.get_sv_left() + picker.get_sv_width() * 0.8,
        picker.get_sv_top() + picker.get_sv_height() * 0.2,
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
    assert_ne!(
        state.borrow().primary_color(),
        before_click,
        "actual SV pointer input must select a color"
    );
    assert_eq!(
        state.borrow().composite()[50 * 512 + 50],
        state.borrow().primary_color(),
        "picker input must update the released active shape while the main window is inactive"
    );
    assert!(picker.get_has_active_shape());
    let selected = state.borrow().primary_color();
    let canvas = ui.window().take_snapshot().unwrap();
    let pixel = canvas.as_slice()[(ui.get_canvas_top() as usize + 50) * canvas.width() as usize
        + ui.get_canvas_left() as usize
        + 50];
    let display = |channel: u8| {
        ((u32::from(channel) * u32::from(selected.a) + 224 * (255 - u32::from(selected.a)) + 127)
            / 255) as u8
    };
    assert_eq!(
        (pixel.r, pixel.g, pixel.b),
        (
            display(selected.r),
            display(selected.g),
            display(selected.b)
        ),
        "inactive main window must render the newly selected color"
    );
    let before_slider = state.borrow().primary_color();
    let position = slint::LogicalPosition::new(
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
    assert_ne!(
        state.borrow().primary_color().r,
        before_slider.r,
        "actual RGB slider input must change the color"
    );
    assert_eq!(
        state.borrow().composite()[50 * 512 + 50],
        state.borrow().primary_color()
    );
    picker.invoke_set_hsv(120.0, 100.0, 100.0);
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 255, 0, 128));
    assert_eq!(
        (picker.get_red(), picker.get_green(), picker.get_blue()),
        (0, 255, 0)
    );
    assert_eq!(
        state.borrow().composite()[50 * 512 + 50],
        Color::rgba(0, 255, 0, 128)
    );
    assert_eq!(state.borrow().document().undo_len(), 0);
    picker.invoke_set_hsv(240.0, 80.0, 0.0);
    assert_eq!(picker.get_hue(), 240.0);
    assert_eq!(picker.get_saturation(), 80.0);
    picker.invoke_set_hsv(picker.get_hue(), picker.get_saturation(), 100.0);
    assert_eq!(
        state.borrow().primary_color(),
        Color::rgba(51, 51, 255, 128)
    );
    picker.invoke_set_hex("#F008".into());
    assert_eq!(state.borrow().primary_color(), Color::rgba(255, 0, 0, 136));
    let before = state.borrow().primary_color();
    picker.invoke_set_hex("invalid".into());
    assert_eq!(state.borrow().primary_color(), before);
    assert!(!picker.get_error().is_empty());
    picker.invoke_set_hex("#3C87E0".into());
    picker.invoke_pick();
    assert!(!picker.get_has_active_shape());
    assert_eq!(state.borrow().tool(), Tool::Eyedropper);
    assert_eq!(state.borrow().document().undo_len(), 1);
    picker.invoke_set_rgba(0, 0, 0, 255);
    assert_eq!(
        state.borrow().composite()[50 * 512 + 50],
        Color::rgba(60, 135, 224, 255),
        "changing the next drawing color must not repaint committed pixels"
    );
    ui.invoke_pointer_down(50.0, 50.0);
    ui.invoke_pointer_up(50.0, 50.0);
    assert_eq!(
        state.borrow().primary_color(),
        Color::rgba(60, 135, 224, 255)
    );
    assert_eq!(picker.get_hex_value(), "#3C87E0FF");
    ui.invoke_show_colors();
    let snapshot = picker.window().take_snapshot().unwrap();
    assert_eq!((snapshot.width(), snapshot.height()), (640, 680));
    if let Some(path) = std::env::var_os("HOSHI_COLOR_SNAPSHOT") {
        image::save_buffer(
            path,
            snapshot.as_bytes(),
            snapshot.width(),
            snapshot.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::CloseRequested);
    assert!(!ui.window().is_visible());
    assert!(
        !picker.window().is_visible(),
        "closing the app must not leave an orphaned color window"
    );
}
