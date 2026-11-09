use hoshi_canvas_spike::{create_app, refresh_workspace};
use hoshi_controller::Color;
use slint::{ComponentHandle, LogicalSize};

#[test]
fn huge_document_uses_a_viewport_texture_and_updates_when_window_resizes() {
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
    state
        .borrow_mut()
        .new_image(262_144, 262_144, 96., Color::rgba(255, 255, 255, 255))
        .unwrap();
    refresh_workspace(&ui, &state);
    ui.window().take_snapshot().unwrap();
    assert!(ui.get_viewport_preview());
    let size = ui.get_canvas_image().size();
    assert!(u64::from(size.width) * u64::from(size.height) <= 4_194_304);
    assert!(size.width < 2000 && size.height < 2000);
    assert_eq!(
        (ui.get_image_width(), ui.get_image_height()),
        (262_144, 262_144)
    );
    ui.window().set_size(LogicalSize::new(1400., 1000.));
    ui.window().take_snapshot().unwrap();
    let larger = ui.get_canvas_image().size();
    assert!(larger.width > size.width);
    assert!(larger.height > size.height);
    ui.invoke_fit_canvas();
    assert!(state.borrow().viewport().zoom < 0.01);
    assert!(!ui.get_status().contains(" · 0%"));
    ui.window().take_snapshot().unwrap();
    state.borrow_mut().reset_view();
    // Pan exposes ordinary workspace outside the image, with no stretched stale texture.
    state.borrow_mut().pan(60., 50.).unwrap();
    refresh_workspace(&ui, &state);
    let snapshot = ui.window().take_snapshot().unwrap();
    let x = ui.get_canvas_left() as usize + 10;
    let y = ui.get_canvas_top() as usize + 10;
    let pixel = snapshot.as_slice()[y * snapshot.width() as usize + x];
    assert_eq!((pixel.r, pixel.g, pixel.b), (220, 226, 235));
    ui.hide().unwrap();
}
