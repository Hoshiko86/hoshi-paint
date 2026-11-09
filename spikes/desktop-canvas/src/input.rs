//! Translate shared UI events into platform-independent editor commands.
//!
//! Coordinates are logical pixels. Slint owns focus/capture and reports button
//! and modifier state; the controller owns every editing decision and transform.
use crate::{AppWindow, SharedController, update};
use hoshi_controller::{EditCursor, InputModifiers, PointerButton};
use slint::ComponentHandle;

fn modifiers(shift: bool, alt: bool, control: bool) -> InputModifiers {
    InputModifiers {
        shift,
        alt,
        control,
    }
}

pub(super) fn bind(ui: &AppWindow, controller: SharedController) {
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_input_modifiers(move |shift, alt, control| {
        update(&weak, &state, |c| {
            c.set_input_modifiers(modifiers(shift, alt, control))
        });
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_shift_modifier(move |pressed| {
        if state.borrow().aspect_lock() != pressed {
            update(&weak, &state, |c| c.set_aspect_lock(pressed));
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_down(move |x, y| update(&weak, &state, |c| c.pointer_down(x.into(), y.into())));
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_pressed(move |x, y, secondary, shift, alt, control| {
        update(&weak, &state, |c| {
            c.set_input_modifiers(modifiers(shift, alt, control))?;
            c.pointer_down_with_button(
                x.into(),
                y.into(),
                if secondary {
                    PointerButton::Secondary
                } else {
                    PointerButton::Primary
                },
            )
        });
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_move(move |x, y| update(&weak, &state, |c| c.pointer_move(x.into(), y.into())));
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_motion(move |x, y, shift, alt, control| {
        update(&weak, &state, |c| {
            c.set_input_modifiers(modifiers(shift, alt, control))?;
            c.pointer_move(x.into(), y.into())
        });
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_up(move |x, y| {
        update(&weak, &state, |c| {
            let movement = c.pointer_move(x.into(), y.into());
            let release = c.pointer_up();
            movement.and(release)
        });
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_released(move |x, y, shift, alt, control| {
        update(&weak, &state, |c| {
            c.set_input_modifiers(modifiers(shift, alt, control))?;
            let movement = c.pointer_move(x.into(), y.into());
            let release = c.pointer_up();
            movement.and(release)
        });
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_keyboard_transform(move |dx, dy, secondary_held, control| {
        update(&weak, &state, |c| {
            c.keyboard_transform(dx, dy, secondary_held, control)
        });
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_hover_canvas(move |x, y| {
        if let Some(ui) = weak.upgrade()
            && let Ok(state) = state.try_borrow()
        {
            // These values are only the Slint presentation wire format. The
            // controller's public API uses named semantic cursor variants.
            let cursor = match state.hover_cursor(x.into(), y.into()) {
                EditCursor::Draw => 0,
                EditCursor::Move => 1,
                EditCursor::ResizeHorizontal => 2,
                EditCursor::ResizeVertical => 3,
                EditCursor::ResizeDiagonalForward => 4,
                EditCursor::ResizeDiagonalBackward => 5,
                EditCursor::Rotate => 6,
                EditCursor::Pivot => 7,
            };
            ui.set_edit_cursor(cursor);
        }
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_pointer_cancel(move || {
        if let Some(ui) = weak.upgrade() {
            ui.invoke_clear_navigation_state();
        }
        update(&weak, &state, |c| {
            if c.is_dragging() {
                c.cancel_pointer();
            }
            c.set_input_modifiers(InputModifiers::default())
        });
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_begin_pan(move |x, y| {
        update(&weak, &state, |c| c.begin_temporary_pan(x.into(), y.into()))
    });
    let (weak, state) = (ui.as_weak(), controller.clone());
    ui.on_end_pan(move |x, y| {
        update(&weak, &state, |c| {
            let movement = c.pointer_move(x.into(), y.into());
            let release = c.end_temporary_pan();
            movement.and(release)
        });
    });
    let weak = ui.as_weak();
    ui.on_pan_by(move |dx, dy| update(&weak, &controller, |c| c.pan(dx.into(), dy.into())));
}
