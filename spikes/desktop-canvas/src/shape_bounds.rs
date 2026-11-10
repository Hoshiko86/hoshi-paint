//! Present exact integer bounds without passing coordinates through UI floats.
use crate::{AppWindow, SharedController, update, workspace::Workspace};
use hoshi_controller::BoundsField;
use slint::ComponentHandle;

pub(super) fn refresh(ui: &AppWindow, state: &Workspace) {
    let shape = state.document().active_shape();
    ui.set_shape_x_text(shape.map(|s| s.x.to_string()).unwrap_or_default().into());
    ui.set_shape_y_text(shape.map(|s| s.y.to_string()).unwrap_or_default().into());
    ui.set_shape_width_text(
        shape
            .map(|s| s.width.to_string())
            .unwrap_or_default()
            .into(),
    );
    ui.set_shape_height_text(
        shape
            .map(|s| s.height.to_string())
            .unwrap_or_default()
            .into(),
    );
}
pub(super) fn bind(ui: &AppWindow, controller: SharedController) {
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_bounds_lock_changed(move |enabled| {
        update(&weak, &state, |c| {
            c.set_numeric_aspect_lock(enabled);
            Ok(())
        });
    });
    let weak = ui.as_weak();
    ui.on_edit_shape_bound(move |field, value, locked| {
        if controller.borrow().document().active_shape().is_none() {
            return;
        }
        let field = match field {
            0 => BoundsField::X,
            1 => BoundsField::Y,
            2 => BoundsField::Width,
            3 => BoundsField::Height,
            _ => return,
        };
        let value = value.trim().parse::<i64>();
        let invalid = if let Ok(value) = value {
            let mut invalid = false;
            update(&weak, &controller, |c| {
                let result = c.edit_shape_bound(field, value, locked);
                invalid = result.is_err();
                result
            });
            invalid
        } else {
            true
        };
        if invalid && let Some(ui) = weak.upgrade() {
            let message = match field {
                BoundsField::X | BoundsField::Y => {
                    "Position must be a whole number between -2147483648 and 2147483647."
                }
                BoundsField::Width | BoundsField::Height => {
                    "Dimensions must be 1–262144 pixels; both must fit when the ratio is locked."
                }
            };
            ui.set_status(message.into());
        }
    });
}
