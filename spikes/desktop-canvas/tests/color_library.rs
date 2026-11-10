use hoshi_canvas_spike::create_app_with_palette_root;
use hoshi_controller::{Color, ColorSlot};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize, Model};

#[test]
fn name_search_requires_selection_and_palette_slots_preserve_alpha() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    // Keep this interaction test independent of the user's saved palettes.
    let app = create_app_with_palette_root(None).unwrap();
    let picker = &app.colors;
    app.ui.show().unwrap();
    app.ui.invoke_show_colors();
    picker.window().set_size(LogicalSize::new(640., 1100.));
    picker
        .window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    picker.window().take_snapshot().unwrap();
    let click = |position, button| {
        picker
            .window()
            .dispatch_event(WindowEvent::PointerPressed { position, button });
        picker
            .window()
            .dispatch_event(WindowEvent::PointerReleased { position, button });
    };
    let key = |text: slint::SharedString| {
        picker
            .window()
            .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        picker
            .window()
            .dispatch_event(WindowEvent::KeyReleased { text });
    };
    let original = app.controller.borrow().primary_color();
    let search_top = picker.get_name_input_top();
    click(
        LogicalPosition::new(
            picker.get_name_input_left() + 12.,
            picker.get_name_input_top() + 14.,
        ),
        PointerEventButton::Left,
    );
    key("Rebecca".into());
    picker.window().take_snapshot().unwrap();
    assert_eq!(
        picker.get_name_input_top(),
        search_top,
        "suggestions must not move the search field into their rows"
    );
    assert_eq!(
        app.controller.borrow().primary_color(),
        original,
        "typing only filters suggestions"
    );
    assert_eq!(
        picker
            .get_color_name_suggestions()
            .row_data(0)
            .unwrap()
            .name,
        "RebeccaPurple"
    );
    key(Key::Return.into());
    assert_eq!(
        app.controller.borrow().primary_color(),
        Color::rgba(102, 51, 153, 255)
    );
    assert!(picker.get_color_name_query().is_empty());

    key("blue".into());
    let next = picker.get_color_name_suggestions().row_data(1).unwrap();
    key(Key::Escape.into());
    assert_eq!(
        app.controller.borrow().primary_color(),
        Color::rgba(102, 51, 153, 255)
    );
    key(Key::DownArrow.into());
    key(Key::Return.into());
    let expected = hoshi_controller::named_colors::lookup(next.name.as_str()).unwrap();
    assert_eq!(app.controller.borrow().primary_color(), expected);

    picker.invoke_set_rgba(12, 34, 56, 78);
    picker.invoke_toggle_palette_insert();
    picker.window().take_snapshot().unwrap();
    let slot = LogicalPosition::new(
        picker.get_swatches_left() + 13.,
        picker.get_swatches_top() + 13.,
    );
    click(slot, PointerEventButton::Left);
    assert!(!picker.get_palette_inserting());
    let stored = picker.get_swatches().row_data(0).unwrap();
    assert_eq!(
        (stored.red, stored.green, stored.blue, stored.alpha),
        (12, 34, 56, 78)
    );
    picker.invoke_set_rgba(200, 100, 50, 255);
    click(slot, PointerEventButton::Right);
    assert_eq!(app.controller.borrow().color_slot(), ColorSlot::Primary);
    assert_eq!(
        app.controller.borrow().primary_color(),
        Color::rgba(200, 100, 50, 255)
    );
    assert_eq!(
        app.controller.borrow().secondary_color(),
        Color::rgba(12, 34, 56, 78)
    );
    click(slot, PointerEventButton::Left);
    assert_eq!(
        app.controller.borrow().primary_color(),
        Color::rgba(12, 34, 56, 78)
    );

    // Exercise the application bridge as well as the standalone storage layer.
    assert!(picker.get_palette_dirty());
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("Favorite colors.txt");
    app.ui
        .invoke_save_palette_path(path.to_string_lossy().as_ref().into());
    assert!(picker.get_palette_error().is_empty());
    assert!(path.is_file());
    assert!(!picker.get_palette_dirty());
    assert_eq!(picker.get_palette_selected(), app.ui.get_palette_selected());
    assert_eq!(
        picker.get_palette_names().row_count(),
        app.ui.get_palette_names().row_count()
    );
    picker.invoke_reset_palette();
    assert!(!picker.get_palette_dirty());
    assert_eq!(picker.get_swatches().row_data(0).unwrap().alpha, 255);
    app.ui
        .invoke_import_palette_path(path.to_string_lossy().as_ref().into());
    assert!(picker.get_palette_error().is_empty());
    assert!(
        picker.get_palette_dirty(),
        "import remains unsaved against the default baseline"
    );
    assert_eq!(picker.get_palette_selected(), -1);
    for model in [picker.get_swatches(), app.ui.get_color_swatches()] {
        let restored = model.row_data(0).unwrap();
        assert_eq!(
            (restored.red, restored.green, restored.blue, restored.alpha),
            (12, 34, 56, 78)
        );
    }
    assert_eq!(app.controller.borrow().document().undo_len(), 0);
}
