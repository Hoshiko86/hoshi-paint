//! One history model is shared by the floating and internally docked hosts.
use crate::{
    AppWindow, HistoryRow, HistoryWindow, SharedController, localization, update,
    workspace::Workspace,
};
use slint::{ComponentHandle, Model, ModelNotify, ModelTracker};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

/// Only row labels are cached. Current/redo flags are computed for visible rows;
/// pointer motion does not walk the retained timeline or recreate its model.
#[derive(Default)]
struct TimelineRows {
    labels: Vec<&'static str>,
    generation: u64,
    structure_generation: u64,
    position: usize,
}
#[derive(Default)]
struct HistoryModel {
    timelines: RefCell<HashMap<u32, TimelineRows>>,
    active: Cell<Option<u32>>,
    notify: ModelNotify,
}
impl HistoryModel {
    fn sync(&self, workspace: &Workspace) -> Result<(), ()> {
        let active = workspace.active_id();
        let mut changed = self.active.replace(active) != active;
        {
            let mut timelines = self.timelines.borrow_mut();
            timelines.retain(|id, _| workspace.tabs().iter().any(|tab| tab.id == *id));
            if let Some(id) = active {
                let document = workspace.document();
                let length = document.history_len();
                let rows = timelines.entry(id).or_default();
                let generation = document.history_generation();
                let structure = document.history_structure_generation();
                let position = document.history_position();
                if rows.generation != generation
                    || rows.labels.len() != length
                    || rows.position != position
                    || rows.structure_generation != structure
                {
                    let rebuild =
                        rows.structure_generation != structure || rows.labels.len() > length;
                    let first = if rebuild { 0 } else { rows.labels.len() };
                    rows.labels
                        .try_reserve(length.saturating_sub(rows.labels.len()))
                        .map_err(|_| ())?;
                    if rebuild {
                        rows.labels.clear();
                    }
                    for index in first..length {
                        rows.labels
                            .push(document.history_step(index).ok_or(())?.label);
                    }
                    rows.generation = generation;
                    rows.structure_generation = structure;
                    rows.position = position;
                    changed = true;
                }
            }
        }
        if changed {
            self.notify.reset();
        }
        Ok(())
    }
}
impl Model for HistoryModel {
    type Data = HistoryRow;
    fn row_count(&self) -> usize {
        self.active
            .get()
            .and_then(|id| {
                self.timelines
                    .borrow()
                    .get(&id)
                    .map(|rows| rows.labels.len())
            })
            .unwrap_or(0)
    }
    fn row_data(&self, index: usize) -> Option<HistoryRow> {
        let active = self.active.get()?;
        let timelines = self.timelines.borrow();
        let rows = timelines.get(&active)?;
        Some(HistoryRow {
            label: (*rows.labels.get(index)?).into(),
            index: i32::try_from(index).ok()?,
            current: index == rows.position,
            redo: index > rows.position,
        })
    }
    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
fn disk_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024. * 1024.))
    } else {
        format!("{:.2} GiB", bytes as f64 / (1024. * 1024. * 1024.))
    }
}

pub(super) fn refresh(ui: &AppWindow, workspace: &Workspace) {
    let old = ui.get_history_rows();
    let model;
    let rows = if let Some(rows) = old.as_any().downcast_ref::<HistoryModel>() {
        rows
    } else {
        model = Rc::new(HistoryModel::default());
        ui.set_history_rows(model.clone().into());
        model.as_ref()
    };
    if rows.sync(workspace).is_err() {
        rows.active.set(None);
        rows.notify.reset();
        ui.set_history_can_jump(false);
        ui.set_history_position(0);
        ui.set_history_status("The history list could not allocate memory. Undo and Redo still work; the history on disk is unchanged.".into());
        ui.invoke_sync_history();
        return;
    }
    let document = workspace.document();
    ui.set_history_can_jump(workspace.has_document() && document.active_shape().is_none());
    ui.set_history_position(document.history_position() as i32);
    let pending = if document.active_shape().is_some() {
        "Commit or cancel the active shape before selecting a history step."
    } else {
        "Select a step to return to it."
    };
    ui.set_history_status(if workspace.has_document() {
        format!(
            "Undo: {} · Redo: {}\nSession history: {} on disk{}\n{pending}",
            document.undo_len(),
            document.redo_len(),
            disk_size(document.history_storage_bytes()),
            if document.history_cleanup_pending() {
                " · cleanup pending"
            } else {
                ""
            }
        )
        .into()
    } else {
        "No image open.".into()
    });
    ui.invoke_sync_history();
}

fn sync(ui: &AppWindow, window: &HistoryWindow) {
    window.set_rows(ui.get_history_rows());
    window.set_current_index(ui.get_history_position());
    window.set_can_jump(ui.get_history_can_jump());
    window.set_status_text(ui.get_history_status());
    window.set_can_undo(ui.get_can_undo() || ui.get_has_shape());
    window.set_can_redo(ui.get_can_redo() && !ui.get_has_shape());
    window.set_has_shape(ui.get_has_shape());
    window.set_placement(ui.get_history_placement());
}

pub(super) fn bind_window(
    ui: &AppWindow,
    controller: SharedController,
    text: &localization::Localizer,
) -> Result<Rc<HistoryWindow>, slint::PlatformError> {
    let window = Rc::new(HistoryWindow::new()?);
    localization::apply(window.as_ref(), text);
    sync(ui, &window);
    let (main, history) = (ui.as_weak(), window.as_weak());
    ui.on_sync_history(move || {
        if let (Some(ui), Some(window)) = (main.upgrade(), history.upgrade()) {
            sync(&ui, &window);
        }
    });
    let (main, history) = (ui.as_weak(), window.clone());
    let state = controller.clone();
    ui.on_show_history(move || {
        if let Some(ui) = main.upgrade() {
            if ui.get_modal_visible() {
                return;
            }
            update(&main, &state, |c| {
                c.pointer_up()?;
                c.set_aspect_lock(false)
            });
            ui.set_history_visible(true);
            if ui.get_history_placement() == 0 {
                let _ = history.show();
            }
        }
    });
    let (main, history) = (ui.as_weak(), window.as_weak());
    ui.on_close_history(move || {
        if let Some(ui) = main.upgrade() {
            ui.set_history_visible(false);
        }
        if let Some(window) = history.upgrade() {
            let _ = window.hide();
        }
    });
    let (main, history) = (ui.as_weak(), window.as_weak());
    ui.on_set_history_placement(move |placement| {
        if let (Some(ui), Some(window)) = (main.upgrade(), history.upgrade()) {
            if ui.get_modal_visible() {
                return;
            }
            let placement = placement.clamp(0, 4);
            ui.set_history_placement(placement);
            ui.set_history_visible(true);
            window.set_placement(placement);
            if placement == 0 {
                let _ = window.show();
            } else {
                let _ = window.hide();
            }
        }
    });
    let main = ui.as_weak();
    window.on_placement_selected(move |value| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_set_history_placement(value);
        }
    });
    let main = ui.as_weak();
    window.on_close_panel(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_close_history();
        }
    });
    let main = ui.as_weak();
    window.window().on_close_requested(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_close_history();
        }
        slint::CloseRequestResponse::HideWindow
    });
    let (main, state) = (ui.as_weak(), controller);
    ui.on_history_jump(move |index| {
        if index >= 0 {
            update(&main, &state, |c| {
                c.go_to_history_step(index as usize).map(|_| ())
            });
        }
    });
    let main = ui.as_weak();
    window.on_jump(move |index| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_history_jump(index);
        }
    });
    let main = ui.as_weak();
    ui.on_history_undo(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_action(0);
        }
    });
    let main = ui.as_weak();
    ui.on_history_redo(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_action(1);
        }
    });
    let main = ui.as_weak();
    window.on_undo(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_action(0);
        }
    });
    let main = ui.as_weak();
    window.on_redo(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_action(1);
        }
    });
    Ok(window)
}
