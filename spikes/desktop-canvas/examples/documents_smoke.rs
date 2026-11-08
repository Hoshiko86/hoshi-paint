//! Real winit document lifecycle, with deterministic file-dialog selections.
use hoshi_canvas_spike::{create_app_with_dialogs, documents::FileDialogs, refresh_workspace};
use slint::{ComponentHandle, LogicalSize};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};
struct Dialogs;
impl FileDialogs for Dialogs {
    fn open(&self, _: &slint::Window) -> Option<Vec<PathBuf>> {
        None
    }
    fn save(&self, _: &slint::Window, _: &Path) -> Option<PathBuf> {
        None
    }
}
fn snapshot(ui: &hoshi_canvas_spike::AppWindow, name: &str) {
    let dir = std::env::var_os("HOSHI_DOCUMENT_SNAPSHOT_DIR").unwrap_or_else(|| "target".into());
    let p = ui.window().take_snapshot().unwrap();
    image::save_buffer(
        PathBuf::from(dir).join(name),
        p.as_bytes(),
        p.width(),
        p.height(),
        image::ColorType::Rgba8,
    )
    .unwrap();
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (ui, state) = create_app_with_dialogs(Rc::new(Dialogs))?;
    ui.window().set_size(LogicalSize::new(1120., 850.));
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(200), move || {
        let ui = weak.upgrade().unwrap();
        let dir = tempfile::tempdir().unwrap();
        ui.invoke_pointer_down(60., 60.);
        ui.invoke_pointer_move(200., 150.);
        ui.invoke_pointer_up(200., 150.);
        let project = dir.path().join("星 Drawing.hoshi");
        ui.invoke_save_document_as(project.to_string_lossy().as_ref().into());
        assert!(project.exists());
        assert!(!state.borrow().any_dirty());
        ui.invoke_show_image_settings(0);
        ui.invoke_select_image_aspect(false);
        ui.invoke_edit_image_setting(0, "320".into());
        ui.invoke_edit_image_setting(1, "240".into());
        ui.invoke_apply_image_settings();
        ui.invoke_pointer_down(40., 40.);
        ui.invoke_pointer_move(150., 130.);
        ui.invoke_pointer_up(150., 130.);
        snapshot(&ui, "spike-documents.png");
        let jpeg = dir.path().join("Export.jpg");
        ui.invoke_save_document_as(jpeg.to_string_lossy().as_ref().into());
        assert!(ui.get_save_config_visible());
        snapshot(&ui, "spike-save-configuration.png");
        ui.invoke_confirm_save_configuration(false);
        assert!(!jpeg.exists());
        ui.invoke_file_action(4);
        assert!(ui.get_file_prompt_visible());
        snapshot(&ui, "spike-save-prompt.png");
        ui.invoke_file_prompt_response(2);
        assert_eq!(state.borrow().tabs().len(), 2);
        ui.invoke_open_document(project.to_string_lossy().as_ref().into());
        assert_eq!(state.borrow().tabs().len(), 2);
        assert!(!state.borrow().active_tab().unwrap().dirty());
        refresh_workspace(&ui, &state);
        ui.invoke_file_action(6);
        while ui.get_file_prompt_visible() {
            ui.invoke_file_prompt_response(1);
        }
        assert!(!ui.window().is_visible());
        println!(
            "PASS: native tabs, Unicode project save, duplicate open, JPEG confirmation, close cancellation and exit"
        );
    });
    ui.run()?;
    Ok(())
}
