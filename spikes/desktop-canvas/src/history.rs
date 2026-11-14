//! One history model is shared by the floating and internally docked hosts.
use crate::{
    AppWindow, HistoryRow, HistoryWindow, SharedController, localization, update,
    workspace::Workspace,
};
use hoshi_core::HistoryLabel;
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
    labels: Vec<HistoryLabel>,
    generation: u64,
    structure_generation: u64,
    position: usize,
}
struct HistoryModel {
    timelines: RefCell<HashMap<u32, TimelineRows>>,
    active: Cell<Option<u32>>,
    notify: ModelNotify,
    text: localization::Localizer,
}

/// Catalog key for each core history label (#111). The core holds no text.
fn label_key(label: HistoryLabel) -> &'static str {
    match label {
        HistoryLabel::OldestAvailableState => "history_oldest",
        HistoryLabel::AddLayer => "history_add_layer",
        HistoryLabel::DeleteLayer => "history_delete_layer",
        HistoryLabel::RenameLayer => "history_rename_layer",
        HistoryLabel::LayerProperties => "history_layer_properties",
        HistoryLabel::ReorderLayer => "history_reorder_layer",
        HistoryLabel::Rectangle => "history_rectangle",
        HistoryLabel::Ellipse => "history_ellipse",
        HistoryLabel::Brush => "history_brush",
        HistoryLabel::Eraser => "history_eraser",
        HistoryLabel::Resolution => "history_resolution",
        HistoryLabel::CanvasSize => "history_canvas_size",
        HistoryLabel::Resize => "history_resize",
        HistoryLabel::FlipHorizontal => "history_flip_horizontal",
        HistoryLabel::FlipVertical => "history_flip_vertical",
        HistoryLabel::RotateClockwise => "history_rotate_clockwise",
        HistoryLabel::RotateCounterClockwise => "history_rotate_counter_clockwise",
        HistoryLabel::Rotate180 => "history_rotate_180",
        HistoryLabel::Flatten => "history_flatten",
        HistoryLabel::FlattenForSave => "history_flatten_for_save",
    }
}

impl HistoryModel {
    fn new(text: localization::Localizer) -> Self {
        Self {
            timelines: RefCell::default(),
            active: Cell::default(),
            notify: ModelNotify::default(),
            text,
        }
    }

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
            label: self.text.text(label_key(*rows.labels.get(index)?)).into(),
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

/// Installs the shared history model with the window's locale. Call once,
/// before the first `refresh`.
pub(super) fn install(ui: &AppWindow, text: &localization::Localizer) {
    ui.set_history_rows(Rc::new(HistoryModel::new(text.clone())).into());
}

pub(super) fn refresh(ui: &AppWindow, workspace: &Workspace) {
    let old = ui.get_history_rows();
    let model;
    let rows = if let Some(rows) = old.as_any().downcast_ref::<HistoryModel>() {
        rows
    } else {
        // Only reached if `install` was skipped; use the system locale.
        model = Rc::new(HistoryModel::new(localization::Localizer::new(
            &localization::system_selection(),
        )));
        ui.set_history_rows(model.clone().into());
        model.as_ref()
    };
    let text = &rows.text;
    if rows.sync(workspace).is_err() {
        rows.active.set(None);
        rows.notify.reset();
        ui.set_history_can_jump(false);
        ui.set_history_position(0);
        ui.set_history_status(text.text("history_status_no_memory").into());
        ui.invoke_sync_history();
        return;
    }
    let document = workspace.document();
    ui.set_history_can_jump(workspace.has_document() && document.active_shape().is_none());
    ui.set_history_position(document.history_position() as i32);
    let hint = text.text(if document.active_shape().is_some() {
        "history_status_commit_first"
    } else {
        "history_status_select_step"
    });
    ui.set_history_status(if workspace.has_document() {
        let (undo, redo) = (
            document.undo_len().to_string(),
            document.redo_len().to_string(),
        );
        let size = disk_size(document.history_storage_bytes());
        text.format(
            if document.history_cleanup_pending() {
                "history_status_cleanup"
            } else {
                "history_status"
            },
            &[
                ("undo", &undo),
                ("redo", &redo),
                ("size", &size),
                ("hint", hint),
            ],
        )
        .into()
    } else {
        text.text("history_status_no_image").into()
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
    localization::apply_history(window.as_ref(), text);
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

#[cfg(test)]
mod tests {
    use super::label_key;
    use crate::localization::{Localizer, REQUIRED_KEYS, select_preferred};
    use hoshi_core::HistoryLabel;

    #[test]
    fn every_history_label_has_a_catalog_key() {
        for label in HistoryLabel::ALL {
            assert!(
                REQUIRED_KEYS.contains(&label_key(label)),
                "{label:?} has no catalog key"
            );
        }
    }

    #[test]
    fn history_rows_and_status_follow_the_selected_locale() {
        let english = Localizer::new(&select_preferred(["en-US"]));
        let spanish = Localizer::new(&select_preferred(["es-ES"]));
        let row = |text: &Localizer, label| text.text(label_key(label)).to_owned();
        assert_eq!(
            row(&english, HistoryLabel::OldestAvailableState),
            "Oldest available state"
        );
        assert_eq!(
            row(&spanish, HistoryLabel::OldestAvailableState),
            "Estado más antiguo disponible"
        );
        assert_eq!(row(&english, HistoryLabel::Brush), "Brush");
        assert_eq!(row(&spanish, HistoryLabel::Brush), "Pincel");
        let status = spanish.format(
            "history_status_cleanup",
            &[
                ("undo", "2"),
                ("redo", "1"),
                ("size", "4.0 KiB"),
                ("hint", "…"),
            ],
        );
        assert_eq!(
            status,
            "Deshacer: 2 · Rehacer: 1\nHistorial de la sesión: 4.0 KiB en disco · limpieza pendiente\n…"
        );
    }
}
