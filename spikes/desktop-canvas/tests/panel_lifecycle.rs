use hoshi_canvas_spike::{create_app_with_all_panels, files};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalSize};

fn key(window: &slint::Window, text: impl Into<slint::SharedString>) {
    let text = text.into();
    window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text });
}
#[test]
fn pending_properties_are_saved_and_auxiliary_windows_close_for_modals_and_exit() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let app = create_app_with_all_panels().unwrap();
    let (ui, state, properties) = (app.ui, app.controller, app.shapes);
    ui.window().set_size(LogicalSize::new(1120., 760.));
    ui.show().unwrap();
    ui.window().take_snapshot().unwrap();
    ui.invoke_show_shape_properties();
    assert!(properties.window().is_visible());
    ui.invoke_show_image_settings(2);
    assert!(!properties.window().is_visible());
    assert!(ui.get_modal_visible());
    ui.invoke_close_image_settings();
    ui.invoke_show_shape_properties();
    ui.invoke_show_about();
    assert!(!properties.window().is_visible());
    assert!(ui.get_modal_visible());
    key(ui.window(), Key::Escape);
    ui.invoke_pointer_down(40., 50.);
    ui.invoke_pointer_move(120., 90.);
    ui.invoke_pointer_up(120., 90.);
    ui.invoke_show_shape_properties();
    properties.window().take_snapshot().unwrap();
    i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        properties.as_ref(),
        "Shape width",
    )
    .next()
    .unwrap()
    .mock_single_click(PointerEventButton::Left);
    properties.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    key(properties.window(), "a");
    properties
        .window()
        .dispatch_event(WindowEvent::KeyReleased {
            text: Key::Control.into(),
        });
    for character in ["3", "0", "0"] {
        key(properties.window(), character);
    }
    assert_eq!(
        state.borrow().document().active_shape().unwrap().width,
        80,
        "typing alone is not yet applied"
    );
    let path = std::env::temp_dir().join(format!(
        "hoshi-panel-save-{}-{}.hoshi",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    ui.invoke_save_document_as(path.to_string_lossy().into_owned().into());
    assert!(
        !ui.get_modal_visible(),
        "save should complete without a prompt"
    );
    let saved = files::load(&path).unwrap();
    assert_eq!(
        saved.composite_pixel(339, 70, false).a,
        255,
        "Save must include the pending width before committing pixels"
    );
    assert!(
        properties.window().is_visible(),
        "saving an existing-format path does not dismiss tool windows"
    );
    assert!(!state.borrow().any_dirty());
    ui.invoke_file_action(6);
    assert!(!ui.window().is_visible());
    assert!(
        !properties.window().is_visible(),
        "Exit must not leave an orphan floating tool window"
    );
    std::fs::remove_file(path).unwrap();
}
