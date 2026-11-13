//! Native file workflow and queued Save / Don't Save / Cancel decisions.
use crate::{
    AppWindow, DocumentTabRow, SharedController, files::FileFormat, localization::Localizer,
    refresh as refresh_app, workspace::Workspace,
};
use slint::{ComponentHandle, Model};
use std::{
    cell::RefCell,
    collections::VecDeque,
    path::{Path, PathBuf},
    rc::Rc,
};

pub(super) fn recent_store_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("HOSHI_CONFIG_DIR") {
        return Some(PathBuf::from(path).join("recent-files.json"));
    }
    #[cfg(target_os = "windows")]
    let root = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let root =
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")));
    root.map(|p| p.join("hoshi-paint/recent-files.json"))
}
pub trait FileDialogs {
    fn open(&self, owner: &slint::Window) -> Option<Vec<PathBuf>>;
    fn save(&self, owner: &slint::Window, suggested: &Path) -> Option<PathBuf>;
}
struct NativeDialogs {
    text: Localizer,
}
impl FileDialogs for NativeDialogs {
    fn open(&self, owner: &slint::Window) -> Option<Vec<PathBuf>> {
        rfd::FileDialog::new()
            .set_parent(&owner.window_handle())
            .set_title(self.text.text("open_image"))
            .add_filter(
                self.text.text("images_hoshi_projects"),
                &[
                    "hoshi", "png", "jpg", "jpeg", "jpe", "jfif", "bmp", "gif", "tif", "tiff",
                    "tga", "webp", "ico",
                ],
            )
            .add_filter(self.text.text("all_files"), &["*"])
            .pick_files()
    }
    fn save(&self, owner: &slint::Window, suggested: &Path) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new()
            .set_parent(&owner.window_handle())
            .set_title(self.text.text("save_as"))
            .set_file_name(suggested.file_name().unwrap_or_default().to_string_lossy());
        if let Some(parent) = suggested.parent().filter(|p| p.is_dir()) {
            dialog = dialog.set_directory(parent)
        }
        let preferred = FileFormat::for_path(suggested).unwrap_or(FileFormat::Project);
        let filters = match preferred {
            FileFormat::Project => vec![
                (self.text.text("hoshi_layered_project"), vec!["hoshi"]),
                (self.text.text("png_image"), vec!["png"]),
                (self.text.text("jpeg_image"), vec!["jpg", "jpeg"]),
            ],
            FileFormat::Png => vec![
                (self.text.text("png_image"), vec!["png"]),
                (self.text.text("hoshi_layered_project"), vec!["hoshi"]),
                (self.text.text("jpeg_image"), vec!["jpg", "jpeg"]),
            ],
            FileFormat::Jpeg => vec![
                (self.text.text("jpeg_image"), vec!["jpg", "jpeg"]),
                (self.text.text("hoshi_layered_project"), vec!["hoshi"]),
                (self.text.text("png_image"), vec!["png"]),
            ],
        };
        for (name, extensions) in filters {
            dialog = dialog.add_filter(name, &extensions)
        }
        dialog.save_file().map(|p| {
            if p.extension().is_none() {
                p.with_extension(match preferred {
                    FileFormat::Project => "hoshi",
                    FileFormat::Png => "png",
                    FileFormat::Jpeg => "jpg",
                })
            } else {
                p
            }
        })
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Goal {
    Close,
    Exit,
    SaveAll,
}
struct SaveRequest {
    id: u32,
    path: PathBuf,
}
#[derive(Default)]
struct Flow {
    goal: Option<Goal>,
    queue: VecDeque<u32>,
    save: Option<SaveRequest>,
}
struct Manager {
    state: SharedController,
    flow: RefCell<Flow>,
    dialogs: Rc<dyn FileDialogs>,
    text: Localizer,
}
impl Manager {
    fn cancel(&self, ui: &AppWindow) {
        *self.flow.borrow_mut() = Flow::default();
        ui.set_file_prompt_visible(false);
        ui.set_save_config_visible(false);
        ui.set_file_busy(false);
        ui.invoke_focus_canvas();
    }
    fn error(&self, ui: &AppWindow, error: String) {
        self.cancel(ui);
        refresh_app(ui, &self.state.borrow());
        ui.set_file_error(error.into());
    }
    fn prepare(&self, ui: &AppWindow) {
        crate::panel_hosts::close_adjustment_panels(ui);
        ui.invoke_clear_navigation_state();
        ui.invoke_focus_canvas();
        let mut c = self.state.borrow_mut();
        let _ = c.pointer_up();
        let _ = c.set_aspect_lock(false);
    }
    fn open(&self, ui: &AppWindow, paths: Vec<PathBuf>) {
        self.prepare(ui);
        let mut failures = Vec::new();
        for path in paths {
            if let Err(e) = self.state.borrow_mut().open_path(&path) {
                failures.push(format!("{}: {e}", path.display()))
            }
        }
        refresh_app(ui, &self.state.borrow());
        if !failures.is_empty() {
            self.error(ui, failures.join("\n"))
        }
    }
    fn begin(&self, ui: &AppWindow, goal: Goal, ids: Vec<u32>) {
        self.prepare(ui);
        *self.flow.borrow_mut() = Flow {
            goal: Some(goal),
            queue: ids.into(),
            save: None,
        };
        self.advance(ui);
    }
    fn advance(&self, ui: &AppWindow) {
        loop {
            let (goal, id) = {
                let f = self.flow.borrow();
                (f.goal, f.queue.front().copied())
            };
            let Some(goal) = goal else { return };
            let Some(id) = id else {
                self.cancel(ui);
                refresh_app(ui, &self.state.borrow());
                if goal == Goal::Exit {
                    let _ = ui.hide();
                }
                return;
            };
            if !self.state.borrow().tabs().iter().any(|t| t.id == id) {
                self.flow.borrow_mut().queue.pop_front();
                continue;
            }
            let result = self.state.borrow_mut().activate(id);
            if let Err(e) = result {
                self.error(ui, e);
                return;
            }
            refresh_app(ui, &self.state.borrow());
            let dirty = self.state.borrow().active_tab().is_some_and(|t| t.dirty());
            if dirty {
                if goal == Goal::SaveAll {
                    self.save(ui, false);
                    return;
                }
                ui.set_file_prompt_name(
                    self.state
                        .borrow()
                        .active_tab()
                        .unwrap()
                        .name
                        .clone()
                        .into(),
                );
                ui.set_file_prompt_visible(true);
                return;
            }
            self.flow.borrow_mut().queue.pop_front();
            if goal != Goal::SaveAll {
                let result = self.state.borrow_mut().close_discard(id);
                if let Err(e) = result {
                    self.error(ui, e);
                    return;
                }
            }
        }
    }
    fn saved(&self, ui: &AppWindow, id: u32) {
        let goal = self.flow.borrow().goal;
        if let Some(goal) = goal {
            self.flow.borrow_mut().queue.pop_front();
            if goal != Goal::SaveAll {
                let result = self.state.borrow_mut().close_discard(id);
                if let Err(e) = result {
                    self.error(ui, e);
                    return;
                }
            }
            self.advance(ui);
        } else {
            ui.invoke_focus_canvas();
            refresh_app(ui, &self.state.borrow());
        }
    }
    fn save(&self, ui: &AppWindow, force_as: bool) {
        let (id, existing, suggested) = {
            let c = self.state.borrow();
            let Some(tab) = c.active_tab() else { return };
            let supported = tab
                .path
                .as_ref()
                .filter(|p| FileFormat::for_path(p).is_ok())
                .cloned();
            let suggested = supported.clone().unwrap_or_else(|| {
                tab.path
                    .as_ref()
                    .map(|p| p.with_extension("hoshi"))
                    .unwrap_or_else(|| PathBuf::from(format!("{}.hoshi", tab.name)))
            });
            (tab.id, supported, suggested)
        };
        let path = if !force_as && existing.is_some() {
            existing
        } else {
            self.prepare(ui);
            ui.set_file_busy(true);
            let picked = self.dialogs.save(ui.window(), &suggested);
            ui.set_file_busy(false);
            picked
        };
        let Some(path) = path else {
            self.cancel(ui);
            return;
        };
        self.save_path(ui, id, path);
    }
    fn save_path(&self, ui: &AppWindow, id: u32, path: PathBuf) {
        ui.invoke_finish_shape_properties_inputs();
        let format = match FileFormat::for_path(&path) {
            Ok(f) => f,
            Err(e) => {
                self.error(ui, e.to_string());
                return;
            }
        };
        if self.state.borrow().active_id() != Some(id) {
            self.error(
                ui,
                self.text.text("save_target_changed_before_write").into(),
            );
            return;
        }
        let flatten = self.state.borrow().requires_flatten(format);
        if format == FileFormat::Jpeg || flatten {
            self.prepare(ui);
            ui.set_save_quality(
                self.state
                    .borrow()
                    .active_tab()
                    .unwrap()
                    .jpeg_quality
                    .into(),
            );
            ui.set_save_config_jpeg(format == FileFormat::Jpeg);
            ui.set_save_config_warning(
                self.text
                    .text(if format == FileFormat::Jpeg {
                        "jpeg_lossy_warning"
                    } else {
                        "save_flatten_warning"
                    })
                    .into(),
            );
            self.flow.borrow_mut().save = Some(SaveRequest { id, path });
            ui.set_save_config_visible(true);
            return;
        }
        self.write(ui, id, path, false);
    }
    fn write(&self, ui: &AppWindow, id: u32, path: PathBuf, flatten: bool) {
        if self.state.borrow().active_id() != Some(id) {
            self.error(ui, self.text.text("save_target_changed").into());
            return;
        }
        ui.set_file_busy(true);
        let result = self.state.borrow_mut().save_to(
            &path,
            ui.get_save_quality().clamp(1, 100) as u8,
            flatten,
        );
        ui.set_file_busy(false);
        match result {
            Ok(()) => self.saved(ui, id),
            Err(error) => self.error(
                ui,
                self.text.format(
                    "save_failed_preserved",
                    &[("path", &path.display().to_string()), ("error", &error)],
                ),
            ),
        }
    }
    fn action(&self, ui: &AppWindow, action: i32) {
        if ui.get_modal_visible() {
            return;
        }
        match action {
            0 => {
                self.prepare(ui);
                ui.set_file_busy(true);
                let paths = self.dialogs.open(ui.window());
                ui.set_file_busy(false);
                if let Some(paths) = paths {
                    self.open(ui, paths)
                }
            }
            1 | 2 => self.save(ui, action == 2),
            3 => {
                let ids = self
                    .state
                    .borrow()
                    .tabs()
                    .iter()
                    .filter(|t| t.dirty())
                    .map(|t| t.id)
                    .collect();
                self.begin(ui, Goal::SaveAll, ids)
            }
            4 => {
                let id = self.state.borrow().active_id();
                if let Some(id) = id {
                    self.begin(ui, Goal::Close, vec![id])
                }
            }
            5 | 6 => {
                let ids = self.state.borrow().tabs().iter().map(|t| t.id).collect();
                self.begin(ui, if action == 6 { Goal::Exit } else { Goal::Close }, ids)
            }
            7 => ui.invoke_show_image_settings(0),
            _ => {}
        }
    }
}
pub(super) fn bind(
    ui: &AppWindow,
    state: SharedController,
    dialogs: Option<Rc<dyn FileDialogs>>,
    text: Localizer,
) {
    let manager = Rc::new(Manager {
        state: state.clone(),
        flow: RefCell::new(Flow::default()),
        dialogs: dialogs.unwrap_or_else(|| Rc::new(NativeDialogs { text: text.clone() })),
        text,
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_file_action(move |action| {
        if let Some(ui) = main.upgrade() {
            m.action(&ui, action)
        }
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_open_document(move |path| {
        if let Some(ui) = main.upgrade()
            && !ui.get_modal_visible()
        {
            m.open(&ui, vec![PathBuf::from(path.as_str())])
        }
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_save_document_as(move |path| {
        if let Some(ui) = main.upgrade()
            && !ui.get_modal_visible()
        {
            let id = m.state.borrow().active_id();
            if let Some(id) = id {
                m.save_path(&ui, id, PathBuf::from(path.as_str()))
            }
        }
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_activate_document(move |id| {
        if let Some(ui) = main.upgrade()
            && !ui.get_modal_visible()
        {
            m.prepare(&ui);
            let result = m.state.borrow_mut().activate(id as u32);
            match result {
                Ok(()) => refresh_app(&ui, &m.state.borrow()),
                Err(e) => m.error(&ui, e),
            }
        }
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_cycle_document(move |backward| {
        if let Some(ui) = main.upgrade()
            && !ui.get_modal_visible()
        {
            m.prepare(&ui);
            let result = m.state.borrow_mut().cycle(backward);
            match result {
                Ok(()) => refresh_app(&ui, &m.state.borrow()),
                Err(e) => m.error(&ui, e),
            }
        }
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_close_document(move |id| {
        if let Some(ui) = main.upgrade()
            && !ui.get_modal_visible()
        {
            m.begin(&ui, Goal::Close, vec![id as u32])
        }
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_open_recent(move |index| {
        if let Some(ui) = main.upgrade()
            && !ui.get_modal_visible()
        {
            let path = m
                .state
                .borrow()
                .recent()
                .get(index.max(0) as usize)
                .cloned();
            if let Some(path) = path {
                m.open(&ui, vec![path])
            }
        }
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_clear_recent(move || {
        if let Some(ui) = main.upgrade()
            && !ui.get_modal_visible()
        {
            m.state.borrow_mut().clear_recent();
            refresh_app(&ui, &m.state.borrow())
        }
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_file_prompt_response(move |response| {
        if let Some(ui) = main.upgrade() {
            if !ui.get_file_prompt_visible() {
                return;
            }
            ui.set_file_prompt_visible(false);
            match response {
                0 => m.save(&ui, false),
                1 => {
                    let id = m.flow.borrow_mut().queue.pop_front();
                    if let Some(id) = id {
                        let result = m.state.borrow_mut().close_discard(id);
                        if let Err(e) = result {
                            m.error(&ui, e);
                            return;
                        }
                    }
                    m.advance(&ui)
                }
                _ => {
                    m.cancel(&ui);
                    refresh_app(&ui, &m.state.borrow())
                }
            }
        }
    });
    let (main, m) = (ui.as_weak(), manager.clone());
    ui.on_confirm_save_configuration(move |accept| {
        if let Some(ui) = main.upgrade() {
            if !ui.get_save_config_visible() {
                return;
            }
            ui.set_save_config_visible(false);
            let request = m.flow.borrow_mut().save.take();
            if accept {
                if let Some(request) = request {
                    m.write(&ui, request.id, request.path, true)
                }
            } else {
                m.cancel(&ui);
                refresh_app(&ui, &m.state.borrow())
            }
        }
    });
    let (main, m) = (ui.as_weak(), manager);
    ui.window().on_close_requested(move || {
        if let Some(ui) = main.upgrade()
            && !ui.get_modal_visible()
        {
            m.action(&ui, 6)
        }
        slint::CloseRequestResponse::KeepWindowShown
    });
}
fn thumbnail(controller: &hoshi_controller::Controller) -> slint::Image {
    let preview = controller.thumbnail(40, 32);
    let mut pixels =
        slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(preview.width, preview.height);
    pixels.make_mut_bytes().copy_from_slice(&preview.rgba);
    slint::Image::from_rgba8(pixels)
}
pub(super) fn refresh(ui: &AppWindow, state: &Workspace) {
    let old = ui.get_document_tabs();
    let rows: Vec<_> = state
        .tabs()
        .iter()
        .map(|tab| {
            let generation = tab.generation() as i32;
            let cached = (0..old.row_count())
                .filter_map(|i| old.row_data(i))
                .find(|row| row.id == tab.id as i32 && row.generation == generation);
            let thumbnail = if let Some(cached) = cached {
                cached.thumbnail
            } else {
                thumbnail(&tab.controller)
            };
            DocumentTabRow {
                id: tab.id as i32,
                title: tab.name.clone().into(),
                detail: tab
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Unsaved image".into())
                    .into(),
                thumbnail,
                active: state.active_id() == Some(tab.id),
                dirty: tab.dirty(),
                generation,
            }
        })
        .collect();
    let same = old.row_count() == rows.len()
        && rows.iter().enumerate().all(|(i, row)| {
            old.row_data(i).is_some_and(|old| {
                old.id == row.id
                    && old.title == row.title
                    && old.detail == row.detail
                    && old.active == row.active
                    && old.dirty == row.dirty
                    && old.generation == row.generation
            })
        });
    if !same {
        ui.set_document_tabs(Rc::new(slint::VecModel::from(rows)).into())
    }
    ui.set_any_dirty(state.any_dirty());
    let recents: Vec<slint::SharedString> = state
        .recent()
        .iter()
        .map(|p| p.display().to_string().into())
        .collect();
    let old = ui.get_recent_files();
    if old.row_count() != recents.len()
        || recents
            .iter()
            .enumerate()
            .any(|(i, p)| old.row_data(i).as_ref() != Some(p))
    {
        ui.set_recent_files(Rc::new(slint::VecModel::from(recents)).into());
    }
    ui.set_document_title(
        state
            .active_tab()
            .map(|tab| {
                format!(
                    "{}{} · Hoshi Paint",
                    tab.name,
                    if tab.dirty() { " *" } else { "" }
                )
            })
            .unwrap_or_else(|| "Hoshi Paint".into())
            .into(),
    );
    if !state.has_document() {
        ui.set_status("No image open · Ctrl/Cmd+N to create or Ctrl/Cmd+O to open".into())
    }
}
