//! A single shape-properties view can float or share the application's dock.
//!
//! This adapter forwards edits to the main window's existing command bindings;
//! hosting changes never commit, cancel, or duplicate the shape preview.
use crate::{AppWindow, ShapePropertiesWindow, SharedController, update};
use hoshi_controller::InputModifiers;
use slint::ComponentHandle;
use std::rc::Rc;

fn sync(main: &AppWindow, window: &ShapePropertiesWindow) {
    window.set_has_document(main.get_has_document());
    window.set_has_shape(main.get_has_shape());
    window.set_shape_mode(main.get_shape_mode());
    window.set_stroke_width_pixels(main.get_stroke_width_pixels());
    window.set_shape_antialiasing(main.get_shape_antialiasing());
    window.set_line_style(main.get_line_style());
    window.set_fill_pattern(main.get_fill_pattern());
    window.set_shape_x_text(main.get_shape_x_text());
    window.set_shape_y_text(main.get_shape_y_text());
    window.set_shape_width_text(main.get_shape_width_text());
    window.set_shape_height_text(main.get_shape_height_text());
    window.set_shape_bounds_locked(main.get_shape_bounds_locked());
    window.set_placement(main.get_shape_properties_placement());
}

pub(super) fn create(
    main: &AppWindow,
    state: SharedController,
) -> Result<Rc<ShapePropertiesWindow>, slint::PlatformError> {
    let window = Rc::new(ShapePropertiesWindow::new()?);
    sync(main, &window);
    let (weak_main, weak_window) = (main.as_weak(), window.as_weak());
    main.on_sync_shape_properties(move || {
        if let (Some(main), Some(window)) = (weak_main.upgrade(), weak_window.upgrade()) {
            sync(&main, &window);
        }
    });
    let (weak_main, weak_window) = (main.as_weak(), window.as_weak());
    main.on_finish_shape_properties_inputs(move || {
        if let (Some(main), Some(window)) = (weak_main.upgrade(), weak_window.upgrade()) {
            window.invoke_finish_edits();
            main.invoke_finish_shape_properties_edits();
        }
    });
    let (weak_main, properties) = (main.as_weak(), window.clone());
    main.on_show_shape_properties(move || {
        if let Some(main) = weak_main.upgrade() {
            if main.get_modal_visible() {
                return;
            }
            properties.invoke_finish_edits();
            main.invoke_finish_shape_properties_edits();
            // End physical capture before focus moves to a second window. The
            // editable preview itself remains active until Enter or Escape.
            update(&weak_main, &state, |controller| {
                controller.pointer_up()?;
                controller.set_input_modifiers(InputModifiers::default())
            });
            main.invoke_clear_navigation_state();
            main.set_shape_properties_visible(true);
            sync(&main, &properties);
            if main.get_shape_properties_placement() == 0 {
                let _ = properties.show();
            }
        }
    });
    let (weak_main, weak_window) = (main.as_weak(), window.as_weak());
    main.on_close_shape_properties(move || {
        if let (Some(main), Some(window)) = (weak_main.upgrade(), weak_window.upgrade()) {
            window.invoke_finish_edits();
            main.invoke_finish_shape_properties_edits();
            main.set_shape_properties_visible(false);
            let _ = window.hide();
        }
    });
    let (weak_main, weak_window) = (main.as_weak(), window.as_weak());
    main.on_set_shape_properties_placement(move |placement| {
        if let (Some(main), Some(window)) = (weak_main.upgrade(), weak_window.upgrade()) {
            if main.get_modal_visible() {
                return;
            }
            // Flush the focused field while its original host is still visible,
            // before synchronizing model values into the destination host.
            window.invoke_finish_edits();
            main.invoke_finish_shape_properties_edits();
            let placement = placement.clamp(0, 4);
            main.set_shape_properties_placement(placement);
            main.set_shape_properties_visible(true);
            sync(&main, &window);
            if placement == 0 {
                let _ = window.show();
            } else {
                let _ = window.hide();
            }
        }
    });

    // Keep the bindings explicit: readers can follow each panel action to the
    // same main-window command used by the docked host and keyboard shortcuts.
    let weak = main.as_weak();
    window.on_select_shape_mode(move |value| {
        if let Some(main) = weak.upgrade() {
            main.invoke_select_shape_mode(value);
        }
    });
    let weak = main.as_weak();
    window.on_edit_stroke_width_pixels(move |value| {
        if let Some(main) = weak.upgrade() {
            main.invoke_edit_stroke_width_pixels(value);
        }
    });
    let weak = main.as_weak();
    window.on_select_line_style(move |value| {
        if let Some(main) = weak.upgrade() {
            main.invoke_select_line_style(value);
        }
    });
    let weak = main.as_weak();
    window.on_select_fill_pattern(move |value| {
        if let Some(main) = weak.upgrade() {
            main.invoke_select_fill_pattern(value);
        }
    });
    let weak = main.as_weak();
    window.on_toggle_shape_antialiasing(move |value| {
        if let Some(main) = weak.upgrade() {
            main.invoke_toggle_shape_antialiasing(value);
        }
    });
    let weak = main.as_weak();
    window.on_edit_shape_bound(move |field, value, locked| {
        if let Some(main) = weak.upgrade() {
            main.invoke_edit_shape_bound(field, value, locked);
        }
    });
    let weak = main.as_weak();
    window.on_bounds_lock_changed(move |value| {
        if let Some(main) = weak.upgrade()
            && main.get_shape_bounds_locked() != value
        {
            main.set_shape_bounds_locked(value);
        }
    });
    let weak = main.as_weak();
    window.on_file_action(move |value| {
        if let Some(main) = weak.upgrade() {
            main.invoke_file_action(value);
        }
    });
    let weak = main.as_weak();
    window.on_cycle_document(move |backward| {
        if let Some(main) = weak.upgrade() {
            main.invoke_cycle_document(backward);
        }
    });
    let weak = main.as_weak();
    window.on_placement_selected(move |value| {
        if let Some(main) = weak.upgrade() {
            main.invoke_set_shape_properties_placement(value);
        }
    });
    let weak = main.as_weak();
    window.on_close_panel(move || {
        if let Some(main) = weak.upgrade() {
            main.invoke_close_shape_properties();
        }
    });
    let weak = main.as_weak();
    window.window().on_close_requested(move || {
        if let Some(main) = weak.upgrade() {
            main.invoke_close_shape_properties();
        }
        slint::CloseRequestResponse::HideWindow
    });
    Ok(window)
}
