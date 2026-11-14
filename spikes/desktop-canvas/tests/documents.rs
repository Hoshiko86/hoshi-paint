use hoshi_canvas_spike::{
    create_app_with_dialogs,
    documents::FileDialogs,
    files::{self, FileFormat},
    refresh_workspace,
    workspace::Workspace,
};
use hoshi_core::{Color, Document};
use slint::{ComponentHandle, LogicalSize};
use std::{
    cell::RefCell,
    collections::VecDeque,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Default)]
struct Dialogs {
    open: RefCell<VecDeque<Option<Vec<PathBuf>>>>,
    save: RefCell<VecDeque<Option<PathBuf>>>,
}
impl FileDialogs for Dialogs {
    fn open(&self, _: &slint::Window) -> Option<Vec<PathBuf>> {
        self.open.borrow_mut().pop_front().flatten()
    }
    fn save(&self, _: &slint::Window, _: &Path) -> Option<PathBuf> {
        self.save.borrow_mut().pop_front().flatten()
    }
}
fn shortcut(ui: &hoshi_canvas_spike::AppWindow, key: &str, shift: bool, alt: bool) {
    use slint::platform::{Key, WindowEvent};
    for modifier in [
        Some(Key::Control),
        shift.then_some(Key::Shift),
        alt.then_some(Key::Alt),
    ]
    .into_iter()
    .flatten()
    {
        ui.window().dispatch_event(WindowEvent::KeyPressed {
            text: modifier.into(),
        });
    }
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: key.into() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: key.into() });
    for modifier in [
        alt.then_some(Key::Alt),
        shift.then_some(Key::Shift),
        Some(Key::Control),
    ]
    .into_iter()
    .flatten()
    {
        ui.window().dispatch_event(WindowEvent::KeyReleased {
            text: modifier.into(),
        });
    }
}
fn edit(w: &mut Workspace, name: &str) {
    w.rename_layer(name).unwrap();
}
#[test]
fn codecs_tabs_savepoints_and_failed_writes_preserve_documents() {
    let dir = tempfile::tempdir().unwrap();
    let pixels = vec![Color::rgba(27, 83, 141, 0), Color::rgba(31, 85, 203, 127)];
    let d = Document::from_rgba(2, 1, 144.125, pixels.clone()).unwrap();
    for (ext, format) in [("hoshi", FileFormat::Project), ("png", FileFormat::Png)] {
        let path = dir.path().join(format!("星 image.{ext}"));
        files::save_atomic(&path, &d, format, 95).unwrap();
        let loaded = files::load(&path).unwrap();
        assert_eq!(loaded.layers()[0].pixels(), pixels);
        assert_eq!(loaded.resolution(), 144.125);
        assert_eq!(loaded.undo_len(), 0);
    }
    let mut w = Workspace::default();
    let first = w.active_id().unwrap();
    edit(&mut w, "First drawing");
    w.set_primary_color(Color::rgba(82, 63, 200, 255)).unwrap();
    let project = dir.path().join("first.hoshi");
    w.save_to(&project, 95, false).unwrap();
    assert!(!w.any_dirty());
    edit(&mut w, "Changed");
    assert!(w.any_dirty());
    w.undo().unwrap();
    assert!(!w.any_dirty());
    w.redo().unwrap();
    w.new_image(12, 8, 300., Color::TRANSPARENT).unwrap();
    let second = w.active_id().unwrap();
    assert_eq!(w.primary_color(), Color::rgba(82, 63, 200, 255));
    assert_eq!(w.document().undo_len(), 0);
    edit(&mut w, "Second drawing");
    assert!(w.save_to(&project, 95, false).is_err());
    assert_eq!(w.active_id(), Some(second));
    assert_eq!(w.open_path(&project).unwrap(), first);
    assert_eq!(w.document().layers()[0].name(), "Changed");
    assert_eq!(w.tabs().len(), 2);
    w.activate(second).unwrap();
    assert_eq!(w.document().layers()[0].name(), "Second drawing");
    let bad = dir.path().join("corrupt.png");
    std::fs::write(&bad, b"not an image").unwrap();
    assert!(w.open_path(&bad).is_err());
    assert_eq!(w.active_id(), Some(second));
    let destination = dir.path().join("existing.jpg");
    std::fs::write(&destination, b"original bytes").unwrap();
    assert!(files::save_atomic(&destination, &d, FileFormat::Jpeg, 90).is_err());
    assert_eq!(std::fs::read(&destination).unwrap(), b"original bytes");
    assert!(
        w.save_to(&dir.path().join("missing/image.hoshi"), 95, false)
            .is_err()
    );
    assert!(w.active_tab().unwrap().dirty());
    w.close_discard(first).unwrap();
    w.close_discard(second).unwrap();
    assert!(!w.has_document());
    w.new_image(4, 3, 96., Color::TRANSPARENT).unwrap();
    assert!(w.has_document());
}
#[test]
fn recents_persist_deduplicate_and_clear() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("config/recent.json");
    let mut w = Workspace::with_recent_store(Some(store.clone()));
    for n in 0..12 {
        w.save_to(&dir.path().join(format!("{n}.hoshi")), 95, false)
            .unwrap();
    }
    assert_eq!(w.recent().len(), 10);
    let mut reloaded = Workspace::with_recent_store(Some(store.clone()));
    assert_eq!(reloaded.recent(), w.recent());
    reloaded.clear_recent();
    assert!(
        Workspace::with_recent_store(Some(store.clone()))
            .recent()
            .is_empty()
    );
    std::fs::write(&store, b"invalid json").unwrap();
    assert!(
        Workspace::with_recent_store(Some(store))
            .recent()
            .is_empty()
    );
}
#[test]
fn ui_file_flow_cancel_save_all_format_confirmation_and_close() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let dialogs = Rc::new(Dialogs::default());
    let (ui, state) = create_app_with_dialogs(dialogs.clone()).unwrap();
    ui.window().set_size(LogicalSize::new(1120., 850.));
    ui.show().unwrap();
    edit(&mut state.borrow_mut(), "Unsaved");
    refresh_workspace(&ui, &state);
    shortcut(&ui, "w", false, false);
    assert!(ui.get_file_prompt_visible());
    ui.invoke_file_prompt_response(2);
    assert!(state.borrow().has_document());
    ui.invoke_file_action(4);
    ui.invoke_file_prompt_response(0); // cancelled native chooser
    assert!(!ui.get_file_prompt_visible());
    assert!(state.borrow().any_dirty());
    let path = dir.path().join("saved.hoshi");
    dialogs.save.borrow_mut().push_back(Some(path.clone()));
    shortcut(&ui, "s", false, false);
    assert!(path.exists());
    assert!(!state.borrow().any_dirty());
    state
        .borrow_mut()
        .new_image(8, 8, 96., Color::TRANSPARENT)
        .unwrap();
    edit(&mut state.borrow_mut(), "Other");
    refresh_workspace(&ui, &state);
    let second_id = state.borrow().active_id();
    let tab_key: slint::SharedString = slint::platform::Key::Tab.into();
    shortcut(&ui, tab_key.as_str(), false, false);
    assert_ne!(state.borrow().active_id(), second_id);
    shortcut(&ui, tab_key.as_str(), true, false);
    assert_eq!(state.borrow().active_id(), second_id);
    let jpg = dir.path().join("export.jpg");
    ui.invoke_save_document_as(jpg.to_string_lossy().as_ref().into());
    assert!(ui.get_save_config_visible());
    ui.invoke_confirm_save_configuration(false);
    assert!(!jpg.exists());
    assert!(state.borrow().document().has_transparency());
    ui.invoke_save_document_as(jpg.to_string_lossy().as_ref().into());
    ui.set_save_quality(90);
    ui.invoke_confirm_save_configuration(true);
    assert!(jpg.exists());
    assert!(!state.borrow().document().has_transparency());
    assert!(!state.borrow().active_tab().unwrap().dirty());
    state.borrow_mut().undo().unwrap();
    assert!(state.borrow().active_tab().unwrap().dirty());
    // Save As failure must leave every tab and its undo history intact.
    ui.invoke_save_document_as(
        dir.path()
            .join("missing/image.hoshi")
            .to_string_lossy()
            .as_ref()
            .into(),
    );
    assert!(!ui.get_file_error().is_empty());
    assert_eq!(state.borrow().tabs().len(), 2);
    ui.invoke_dismiss_file_error();
    let second = dir.path().join("second.hoshi");
    dialogs.save.borrow_mut().push_back(Some(second.clone()));
    shortcut(&ui, "s", true, false);
    assert!(second.exists());
    let ids: Vec<_> = state.borrow().tabs().iter().map(|t| t.id).collect();
    for id in ids {
        ui.invoke_activate_document(id as i32);
        edit(&mut state.borrow_mut(), "Save all");
    }
    refresh_workspace(&ui, &state);
    shortcut(&ui, "s", false, true);
    assert!(!state.borrow().any_dirty());
    dialogs.open.borrow_mut().push_back(Some(vec![path]));
    shortcut(&ui, "o", false, false);
    assert_eq!(state.borrow().tabs().len(), 2); // duplicate open focuses existing tab
    edit(&mut state.borrow_mut(), "Close cancelled");
    refresh_workspace(&ui, &state);
    ui.invoke_file_action(5);
    assert!(ui.get_file_prompt_visible());
    ui.invoke_file_prompt_response(2);
    assert_eq!(state.borrow().tabs().len(), 2);
    ui.invoke_file_action(5);
    ui.invoke_file_prompt_response(1);
    assert!(!state.borrow().has_document());
    assert!(!ui.get_has_document());
    shortcut(&ui, "n", false, false);
    assert_eq!(ui.get_image_dialog_kind(), 0);
    ui.invoke_close_image_settings();
    // Saving from the final close prompt writes the file before removing the tab.
    state
        .borrow_mut()
        .new_image(4, 4, 96., Color::TRANSPARENT)
        .unwrap();
    edit(&mut state.borrow_mut(), "Final save");
    refresh_workspace(&ui, &state);
    let final_path = dir.path().join("final.hoshi");
    dialogs
        .save
        .borrow_mut()
        .push_back(Some(final_path.clone()));
    ui.invoke_file_action(4);
    assert!(ui.get_file_prompt_visible());
    ui.invoke_file_prompt_response(0);
    assert!(final_path.exists());
    assert!(!state.borrow().has_document());
    ui.hide().unwrap();
}

#[test]
fn supported_raster_imports_jpeg_density_and_high_depth_guard() {
    let dir = tempfile::tempdir().unwrap();
    let pixels = vec![Color::rgba(75, 120, 180, 255); 64];
    let d = Document::from_rgba(8, 8, 300.4, pixels).unwrap();
    let jpg = dir.path().join("photo.jpg");
    files::save_atomic(&jpg, &d, FileFormat::Jpeg, 95).unwrap();
    let loaded = files::load(&jpg).unwrap();
    assert_eq!(loaded.resolution(), 300.);
    assert_eq!((loaded.width(), loaded.height()), (8, 8));
    assert!(
        loaded.layers()[0]
            .pixels()
            .iter()
            .all(|p| p.r.abs_diff(75) < 5
                && p.g.abs_diff(120) < 5
                && p.b.abs_diff(180) < 5
                && p.a == 255)
    );
    let raw: Vec<_> = d.layers()[0]
        .pixels()
        .iter()
        .flat_map(|c| [c.r, c.g, c.b, c.a])
        .collect();
    for (ext, format) in [
        ("bmp", image::ImageFormat::Bmp),
        ("gif", image::ImageFormat::Gif),
        ("tiff", image::ImageFormat::Tiff),
        ("tga", image::ImageFormat::Tga),
        ("webp", image::ImageFormat::WebP),
        ("ico", image::ImageFormat::Ico),
    ] {
        let path = dir.path().join(format!("image.{ext}"));
        image::save_buffer_with_format(&path, &raw, 8, 8, image::ColorType::Rgba8, format).unwrap();
        let loaded = files::load(&path).unwrap_or_else(|e| panic!("{ext}: {e}"));
        assert_eq!((loaded.width(), loaded.height()), (8, 8), "{ext}");
    }
    let high = dir.path().join("high-depth.png");
    let mut encoder = png::Encoder::new(std::fs::File::create(&high).unwrap(), 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Sixteen);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[0; 8]).unwrap();
    writer.finish().unwrap();
    assert!(matches!(
        files::load(&high),
        Err(files::FileError::Unsupported16Bit)
    ));
}

#[test]
fn file_errors_are_described_in_the_selected_locale() {
    use hoshi_canvas_spike::{describe_error_in, workspace::WorkspaceError};
    let dir = tempfile::tempdir().unwrap();
    let mut w = Workspace::default();
    let pdn = dir.path().join("picture.pdn");
    std::fs::write(&pdn, b"pdn").unwrap();
    let error = w.open_path(&pdn).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::File(files::FileError::PdnImportUnsupported)
    ));
    let english = describe_error_in(&error, &["en-US"]);
    let spanish = describe_error_in(&error, &["es-ES"]);
    assert!(
        english.starts_with("Paint.NET .pdn files are not supported yet"),
        "{english}"
    );
    assert!(
        spanish.starts_with("Los archivos .pdn de Paint.NET"),
        "{spanish}"
    );
    // Developer text from Display never reaches the user.
    assert_ne!(english, error.to_string());
    // Placeholders are filled from the error's data.
    let limit = describe_error_in(&WorkspaceError::OpenLimit, &["es-ES"]);
    assert!(limit.contains("32"), "{limit}");
}

#[test]
fn shared_rendering_settings_keep_fractional_precision_when_switching_tabs() {
    use hoshi_controller::{FillPattern, LineStyle};
    let mut w = Workspace::default();
    let first = w.active_id().unwrap();
    w.set_stroke_width_pixels(6.5).unwrap();
    w.set_line_style(LineStyle::DashDot).unwrap();
    w.set_fill_pattern(FillPattern::DiagonalCross).unwrap();
    w.set_antialiasing(false).unwrap();
    w.new_image(32, 32, 96., Color::TRANSPARENT).unwrap();
    assert_eq!(w.stroke_width_pixels(), 6.5);
    assert_eq!(w.line_style(), LineStyle::DashDot);
    assert_eq!(w.fill_pattern(), FillPattern::DiagonalCross);
    assert!(!w.antialiasing());
    w.set_stroke_width_pixels(0.25).unwrap();
    w.set_antialiasing(true).unwrap();
    w.activate(first).unwrap();
    assert_eq!(w.stroke_width_pixels(), 0.25);
    assert!(w.antialiasing());
    w.pointer_down(2., 2.).unwrap();
    w.pointer_move(20., 20.).unwrap();
    w.pointer_up().unwrap();
    let shape = w.document().active_shape().unwrap();
    assert_eq!(shape.rendering.line_style, LineStyle::DashDot);
    assert_eq!(shape.rendering.fill_pattern, FillPattern::DiagonalCross);
}
