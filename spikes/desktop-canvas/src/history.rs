//! One history model is shared by the floating and internally docked hosts.
use crate::{AppWindow, HistoryRow, HistoryWindow, SharedController, update};
use hoshi_controller::Controller;
use slint::{ComponentHandle, Model};
use std::rc::Rc;

pub(super) fn refresh(ui: &AppWindow, controller: &Controller) {
    let steps = controller.history_steps();
    let old = ui.get_history_rows();
    let rows_changed = old.row_count() != steps.len()
        || steps.iter().enumerate().any(|(index, step)| {
            old.row_data(index).is_none_or(|row| {
                row.label != step.label
                    || row.current != step.is_current
                    || row.redo != step.is_redo
            })
        });
    let can_jump = controller.document().active_shape().is_none();
    if !rows_changed && ui.get_history_can_jump() == can_jump {
        return;
    }
    if rows_changed {
        let rows = steps
            .into_iter()
            .enumerate()
            .map(|(index, step)| HistoryRow {
                label: step.label.into(),
                index: index as i32,
                current: step.is_current,
                redo: step.is_redo,
            })
            .collect::<Vec<_>>();
        ui.set_history_rows(Rc::new(slint::VecModel::from(rows)).into());
    }
    let document = controller.document();
    ui.set_history_can_jump(document.active_shape().is_none());
    ui.set_history_position(controller.history_position() as i32);
    let pending = if document.active_shape().is_some() {
        "Commit or cancel the active shape before selecting a history step."
    } else {
        "Select a step to return to it."
    };
    ui.set_history_status(
        format!(
            "Undo: {} · Redo: {}\n{pending}",
            document.undo_len(),
            document.redo_len()
        )
        .into(),
    );
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
) -> Result<Rc<HistoryWindow>, slint::PlatformError> {
    let window = Rc::new(HistoryWindow::new()?);
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
