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
    assert_eq!(state.borrow().tool(), Tool::Eyedropper);
    assert_eq!(state.borrow().document().undo_len(), 1);
    picker.invoke_set_rgba(0, 0, 0, 255);
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
