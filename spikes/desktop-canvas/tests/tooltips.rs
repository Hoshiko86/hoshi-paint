use hoshi_canvas_spike::create_app_with_colors;
use hoshi_controller::Color;
use i_slint_backend_testing::ElementHandle;
use slint::{ComponentHandle, LogicalPosition, Model};
use std::time::Duration;

#[test]
fn swatch_hover_opens_rich_tooltip_without_changing_selection_or_blocking_clicks() {
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
    ui.invoke_show_colors();
    picker.invoke_set_rgba(0, 0, 0, 128);
    picker.window().take_snapshot().unwrap();
    let swatch = picker.get_swatches().row_data(0).unwrap();
    assert_eq!(swatch.label, "#000000FF");
    assert!(swatch.description.starts_with("Black\n"));
    assert!(swatch.description.contains("RGB: 0, 0, 0"));
    assert!(swatch.description.contains("HSV:"));
    assert!(
        ElementHandle::find_by_accessible_label(picker.as_ref(), "Color Picker")
            .next()
            .is_some()
    );
    let tooltip_count =
        || ElementHandle::find_by_accessible_label(picker.as_ref(), &swatch.description).count();
    assert_eq!(tooltip_count(), 0);
    let position = LogicalPosition::new(
        picker.get_swatches_left() + 20.0,
        picker.get_swatches_top() + 20.0,
    );
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerMoved { position });
    i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1000));
    let snapshot = picker.window().take_snapshot().unwrap();
    if let Some(path) = std::env::var_os("HOSHI_TOOLTIP_SNAPSHOT") {
        image::save_buffer(
            path,
            snapshot.as_bytes(),
            snapshot.width(),
            snapshot.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    assert!(
        tooltip_count() > 0,
        "hover must create a visible tooltip popup"
    );
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 0, 0, 128));
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
    assert_eq!(state.borrow().primary_color(), Color::rgba(0, 0, 0, 255));
    picker
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerMoved {
            position: LogicalPosition::new(4.0, 4.0),
        });
    i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1000));
    assert_eq!(
        tooltip_count(),
        0,
        "leaving the swatch must dismiss its tooltip"
    );
}
