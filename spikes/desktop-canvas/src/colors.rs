use crate::{AppWindow, ColorPickerWindow, SharedController, refresh, update};
use hoshi_controller::{
    Color,
    color::{self, Hsv},
};
use slint::ComponentHandle;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc::TryRecvError,
    time::Duration,
};

fn synchronize(picker: &ColorPickerWindow, value: Color, override_hsv: Option<Hsv>) {
    let mut hsv = override_hsv.unwrap_or_else(|| color::color_to_hsv(value));
    if override_hsv.is_none() {
        if hsv.saturation == 0.0 {
            hsv.hue = picker.get_hue().into();
        }
        if hsv.value == 0.0 {
            hsv.saturation = picker.get_saturation().into();
        }
    }
    if (f64::from(picker.get_hue()) - hsv.hue).abs() > 0.0001
        || picker.get_sv_image().size().width == 0
    {
        let base = color::color_from_hsv(hsv.hue, 100.0, 100.0, 255).expect("validated hue");
        let mut pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(256, 180);
        for (index, pixel) in pixels.make_mut_slice().iter_mut().enumerate() {
            let s = (index % 256) as f64 / 255.0;
            let v = 1.0 - (index / 256) as f64 / 179.0;
            let channel = |c: u8| (((1.0 - s) * 255.0 + s * f64::from(c)) * v).round() as u8;
            *pixel = slint::Rgba8Pixel {
                r: channel(base.r),
                g: channel(base.g),
                b: channel(base.b),
                a: 255,
            };
        }
        picker.set_sv_image(slint::Image::from_rgba8(pixels));
    }
    picker.set_red(value.r.into());
    picker.set_green(value.g.into());
    picker.set_blue(value.b.into());
    picker.set_alpha(value.a.into());
    picker.set_hue(hsv.hue as f32);
    picker.set_saturation(hsv.saturation as f32);
    picker.set_brightness(hsv.value as f32);
    picker.set_hex_value(color::format_hex(value).into());
    picker
        .set_current_description(format!("Current color\n{}", describe_swatch(value, None)).into());
    picker.set_current_color(slint::Color::from_argb_u8(
        value.a, value.r, value.g, value.b,
    ));
}

pub(super) fn bind_picker(
    ui: &AppWindow,
    controller: SharedController,
    palette_root: Option<std::path::PathBuf>,
) -> Result<Rc<ColorPickerWindow>, slint::PlatformError> {
    let picker = Rc::new(ColorPickerWindow::new()?);
    let original = Rc::new(Cell::new(controller.borrow().selected_color()));
    let originals = Rc::new(Cell::new([
        controller.borrow().primary_color(),
        controller.borrow().secondary_color(),
    ]));
    let (weak, reference) = (picker.as_weak(), original.clone());
    picker.on_restore_original(move || {
        if let Some(picker) = weak.upgrade() {
            let color = reference.get();
            picker.invoke_set_rgba(
                color.r.into(),
                color.g.into(),
                color.b.into(),
                color.a.into(),
            );
        }
    });
    let weak = picker.as_weak();
    ui.on_restore_original_color(move || {
        if let Some(picker) = weak.upgrade() {
            picker.invoke_restore_original();
        }
    });
    let (main, colors, state, reference) = (
        ui.as_weak(),
        picker.as_weak(),
        controller.clone(),
        original.clone(),
    );
    let baselines = originals.clone();
    ui.on_select_color_slot(move |target| {
        if let (Some(ui), Some(picker)) = (main.upgrade(), colors.upgrade()) {
            let target = if target == 1 {
                hoshi_controller::ColorSlot::Secondary
            } else {
                hoshi_controller::ColorSlot::Primary
            };
            if state.borrow().color_slot() == target || ui.get_screen_picking() {
                return;
            }
            state.borrow_mut().set_color_slot(target);
            picker.set_color_slot(if target == hoshi_controller::ColorSlot::Primary {
                0
            } else {
                1
            });
            let current = state.borrow().selected_color();
            let baseline = baselines.get()[if target == hoshi_controller::ColorSlot::Primary {
                0
            } else {
                1
            }];
            begin_comparison(&ui, &picker, &reference, baseline);
            synchronize(&picker, current, None);
            sync_docked(&ui, &picker);
            refresh(&ui, &state.borrow());
        }
    });
    let main = ui.as_weak();
    picker.on_select_color_slot(move |target| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_select_color_slot(target);
        }
    });
    for swap in [true, false] {
        let (main, state) = (ui.as_weak(), controller.clone());
        let action = move || {
            if let Some(ui) = main.upgrade() {
                if ui.get_screen_picking() {
                    return;
                }
                update(&ui.as_weak(), &state, |c| {
                    c.pointer_up()?;
                    if swap {
                        c.swap_colors()
                    } else {
                        c.reset_colors()
                    }
                });
            }
        };
        if swap {
            ui.on_swap_colors(action);
        } else {
            ui.on_reset_colors(action);
        }
    }
    let main = ui.as_weak();
    picker.on_swap_colors(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_swap_colors();
        }
    });
    let main = ui.as_weak();
    picker.on_reset_colors(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_reset_colors();
        }
    });
    // Both hosts edit the same controller and use the same panel component.
    let weak = picker.as_weak();
    ui.on_edit_rgba(move |r, g, b, a| {
        if let Some(p) = weak.upgrade() {
            p.invoke_set_rgba(r, g, b, a);
        }
    });
    let weak = picker.as_weak();
    ui.on_edit_hsv(move |h, s, v| {
        if let Some(p) = weak.upgrade() {
            p.invoke_set_hsv(h, s, v);
        }
    });
    let weak = picker.as_weak();
    ui.on_edit_hex(move |text| {
        if let Some(p) = weak.upgrade() {
            p.invoke_set_hex(text);
        }
    });
    let weak = picker.as_weak();
    ui.on_pick_screen(move || {
        if let Some(p) = weak.upgrade() {
            p.invoke_pick_screen();
        }
    });
    let weak = picker.as_weak();
    ui.on_toggle_color_dock(move || {
        if let Some(p) = weak.upgrade() {
            p.invoke_toggle_dock();
        }
    });
    let weak = picker.as_weak();
    ui.on_close_colors(move || {
        if let Some(p) = weak.upgrade() {
            p.invoke_close_picker();
        }
    });
    let main = ui.as_weak();
    picker.on_toggle_dock(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_set_colors_placement(if ui.get_colors_placement() == 0 { 2 } else { 0 });
        }
    });
    let (main, weak, state) = (ui.as_weak(), picker.as_weak(), controller.clone());
    let reference = original.clone();
    let baselines = originals.clone();
    ui.on_set_colors_placement(move |placement| {
        if let (Some(ui), Some(p)) = (main.upgrade(), weak.upgrade()) {
            if ui.get_modal_visible() {
                return;
            }
            {
                let mut c = state.borrow_mut();
                c.pointer_up().expect("finish gesture before docking");
                c.set_aspect_lock(false).expect("gesture ended");
            }
            ui.invoke_refresh_palette_library();
            if !ui.get_colors_visible() {
                baselines.set([
                    state.borrow().primary_color(),
                    state.borrow().secondary_color(),
                ]);
                begin_comparison(&ui, &p, &reference, state.borrow().selected_color());
            }
            let placement = placement.clamp(0, 4);
            ui.set_colors_placement(placement);
            ui.set_colors_docked(placement != 0);
            ui.set_colors_visible(true);
            p.set_placement(placement);
            p.set_docked(placement != 0);
            if placement == 0 {
                let _ = p.show();
            } else {
                let _ = p.hide();
            }
        }
    });
    let main = ui.as_weak();
    picker.on_placement_selected(move |value| {
        if let Some(ui) = main.upgrade() {
            ui.invoke_set_colors_placement(value);
        }
    });
    let request = Rc::new(RefCell::new(None::<crate::screen_pick::ScreenPick>));
    let timer = Rc::new(slint::Timer::default());
    let colors = picker.as_weak();
    let main = ui.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(ui) = main.upgrade() {
            ui.invoke_close_history();
        }
        if let Some(p) = colors.upgrade() {
            p.invoke_close_picker();
        }
        slint::CloseRequestResponse::HideWindow
    });
    synchronize(&picker, controller.borrow().selected_color(), None);
    picker.set_has_active_shape(controller.borrow().document().active_shape().is_some());
    begin_comparison(ui, &picker, &original, original.get());
    let weak = picker.as_weak();
    ui.on_hide_colors(move || {
        if let Some(p) = weak.upgrade() {
            p.invoke_close_picker();
        }
    });
    let weak = picker.as_weak();
    let main = ui.as_weak();
    ui.on_sync_color(move |r, g, b, a, active| {
        if let Some(picker) = weak.upgrade() {
            picker.set_has_active_shape(active);
            if let Some(ui) = main.upgrade() {
                picker.set_color_slot(ui.get_color_slot());
                picker.set_workspace_primary(ui.get_workspace_primary());
                picker.set_workspace_secondary(ui.get_workspace_secondary());
                picker.set_primary_description(ui.get_workspace_primary_description());
                picker.set_secondary_description(ui.get_workspace_secondary_description());
            }
            // Geometry/viewport updates do not change the color controls. Avoid
            // HSV conversion, image/property syncing and layout invalidation on
            // every resize event when the selected color is unchanged.
            if (
                picker.get_red(),
                picker.get_green(),
                picker.get_blue(),
                picker.get_alpha(),
            ) == (r, g, b, a)
                && picker.get_sv_image().size().width != 0
            {
                return;
            }
            synchronize(
                &picker,
                Color::rgba(r as u8, g as u8, b as u8, a as u8),
                None,
            );
            if let Some(ui) = main.upgrade() {
                sync_docked(&ui, &picker);
            }
        }
    });
    let (colors, main, state) = (picker.clone(), ui.as_weak(), controller.clone());
    let reference = original.clone();
    let baselines = originals.clone();
    ui.on_show_colors(move || {
        if let Some(ui) = main.upgrade() {
            if ui.get_modal_visible() {
                return;
            }
            ui.invoke_refresh_palette_library();
            if !ui.get_colors_visible() {
                baselines.set([
                    state.borrow().primary_color(),
                    state.borrow().secondary_color(),
                ]);
                begin_comparison(&ui, &colors, &reference, state.borrow().selected_color());
            }
            // Switching to our color window must preserve the editable preview,
            // even if native pointer release/capture has not settled yet.
            {
                let mut controller = state.borrow_mut();
                controller
                    .pointer_up()
                    .expect("end gesture before color editing");
                controller
                    .set_aspect_lock(false)
                    .expect("gesture has ended");
            }
            refresh(&ui, &state.borrow());
            ui.set_colors_visible(true);
            if !ui.get_colors_docked() {
                colors.show().expect("show color picker");
            }
        }
    });
    let (main, colors, state) = (ui.as_weak(), picker.as_weak(), controller.clone());
    let (pending, polling) = (request.clone(), timer.clone());
    picker.on_set_rgba(move |r, g, b, a| {
        pending.borrow_mut().take();
        polling.stop();
        if let Some(p) = colors.upgrade() {
            p.set_screen_picking(false);
        }
        let value = Color::rgba(
            r.clamp(0, 255) as u8,
            g.clamp(0, 255) as u8,
            b.clamp(0, 255) as u8,
            a.clamp(0, 255) as u8,
        );
        update(&main, &state, |c| {
            c.pointer_up()?;
            c.set_selected_color(value)
        });
        if let Some(picker) = colors.upgrade() {
            picker.set_error("".into());
            if let Some(ui) = main.upgrade() {
                sync_docked(&ui, &picker);
            }
        }
    });
    let (main, colors, state) = (ui.as_weak(), picker.as_weak(), controller.clone());
    let (pending, polling) = (request.clone(), timer.clone());
    picker.on_set_hsv(move |h, s, v| {
        pending.borrow_mut().take();
        polling.stop();
        if let Some(p) = colors.upgrade() {
            p.set_screen_picking(false);
        }
        if !h.is_finite() || !s.is_finite() || !v.is_finite() {
            return;
        }
        let hsv = Hsv {
            hue: f64::from(h.clamp(0.0, 360.0)),
            saturation: f64::from(s.clamp(0.0, 100.0)),
            value: f64::from(v.clamp(0.0, 100.0)),
        };
        let alpha = state.borrow().selected_color().a;
        if let Ok(value) = color::color_from_hsv(hsv.hue, hsv.saturation, hsv.value, alpha) {
            update(&main, &state, |c| {
                c.pointer_up()?;
                c.set_selected_color(value)
            });
            if let Some(picker) = colors.upgrade() {
                synchronize(&picker, value, Some(hsv));
                picker.set_error("".into());
                if let Some(ui) = main.upgrade() {
                    sync_docked(&ui, &picker);
                }
            }
        }
    });
    let (main, colors, state) = (ui.as_weak(), picker.as_weak(), controller.clone());
    let (pending, polling) = (request.clone(), timer.clone());
    picker.on_set_hex(move |text| {
        pending.borrow_mut().take();
        polling.stop();
        if let Some(p) = colors.upgrade() {
            p.set_screen_picking(false);
        }
        match color::parse_hex(&text) {
            Ok(value) => {
                update(&main, &state, |c| {
                    c.pointer_up()?;
                    c.set_selected_color(value)
                });
                if let Some(p) = colors.upgrade() {
                    p.set_error("".into());
                    if let Some(ui) = main.upgrade() {
                        sync_docked(&ui, &p);
                    }
                }
            }
            Err(_) => {
                if let Some(p) = colors.upgrade() {
                    p.set_error("Use #RGB, #RGBA, #RRGGBB or #RRGGBBAA.".into());
                    if let Some(ui) = main.upgrade() {
                        sync_docked(&ui, &p);
                    }
                }
            }
        }
    });
    // Retain the request and polling timer with the picker callbacks.
    let (main, colors, state) = (ui.as_weak(), picker.as_weak(), controller.clone());
    let (pending, polling) = (request.clone(), timer.clone());
    picker.on_pick_screen(move || {
        if pending.borrow().is_some() {
            return;
        }
        let Some(ui) = main.upgrade() else { return };
        if ui.get_modal_visible() {
            return;
        }
        if !ui.get_colors_visible() {
            ui.invoke_show_colors();
        }
        // Finish the pointer gesture before the system picker takes activation.
        update(&main, &state, |c| c.pointer_up());
        if let Some(picker) = colors.upgrade() {
            picker.set_error("Choose a screen pixel. Escape cancels.".into());
            picker.set_screen_picking(true);
            sync_docked(&ui, &picker);
        }
        *pending.borrow_mut() = Some(crate::screen_pick::start());
        let (pending, main, colors, state) =
            (pending.clone(), main.clone(), colors.clone(), state.clone());
        let timer = Rc::downgrade(&polling);
        polling.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(30),
            move || {
                let result = {
                    let borrowed = pending.borrow();
                    let Some(request) = borrowed.as_ref() else {
                        return;
                    };
                    match request.receiver.try_recv() {
                        Ok(value) => value,
                        Err(TryRecvError::Empty) => return,
                        Err(TryRecvError::Disconnected) => {
                            Err("Color Picker closed unexpectedly.".into())
                        }
                    }
                };
                pending.borrow_mut().take();
                if let Some(timer) = timer.upgrade() {
                    timer.stop();
                }
                finish_screen_pick(&main, &state, &colors, result);
            },
        );
    });
    let colors = picker.as_weak();
    let weak = picker.as_weak();
    picker.window().on_close_requested(move || {
        if let Some(p) = weak.upgrade() {
            p.invoke_close_picker();
        }
        slint::CloseRequestResponse::HideWindow
    });
    let main = ui.as_weak();
    picker.on_close_picker(move || {
        if let Some(ui) = main.upgrade() {
            ui.set_colors_visible(false);
            ui.set_screen_picking(false);
        }
        request.borrow_mut().take();
        timer.stop();
        if let Some(p) = colors.upgrade() {
            p.set_screen_picking(false);
            p.set_error("".into());
            let _ = p.hide();
        }
    });
    crate::color_library::bind(ui, &picker, controller.clone(), palette_root);
    Ok(picker)
}

fn finish_screen_pick(
    main: &slint::Weak<AppWindow>,
    state: &SharedController,
    colors: &slint::Weak<ColorPickerWindow>,
    result: crate::screen_pick::PickResult,
) {
    if let Some(picker) = colors.upgrade() {
        picker.set_screen_picking(false);
        match result {
            Ok(Some(value)) => {
                update(main, state, |c| c.set_selected_color(value));
                picker.set_error("".into());
            }
            Ok(None) => picker.set_error("".into()),
            Err(error) => picker.set_error(error.into()),
        }
        if let Some(ui) = main.upgrade() {
            sync_docked(&ui, &picker);
        }
    }
}

fn swatch_hex(value: Color) -> String {
    if value.a == 255 {
        format!("#{:02X}{:02X}{:02X}", value.r, value.g, value.b)
    } else {
        color::format_hex(value)
    }
}

pub(super) fn describe_swatch(value: Color, name: Option<&str>) -> String {
    let hsv = color::color_to_hsv(value);
    let heading = name
        .filter(|name| !name.is_empty())
        .map(|name| format!("{name}\n"))
        .unwrap_or_default();
    format!(
        "{heading}Hex: {}\nRGB: {}, {}, {}\nHSV: {:.0}°, {:.0}%, {:.0}%",
        swatch_hex(value),
        value.r,
        value.g,
        value.b,
        hsv.hue,
        hsv.saturation,
        hsv.value
    )
}

fn begin_comparison(
    ui: &AppWindow,
    picker: &ColorPickerWindow,
    original: &Cell<Color>,
    value: Color,
) {
    original.set(value);
    // A new comparison also discards any hex draft that was never accepted.
    synchronize(picker, value, None);
    picker.set_error("".into());
    picker.set_original_color(slint::Color::from_argb_u8(
        value.a, value.r, value.g, value.b,
    ));
    picker.set_original_description(
        format!("Original color\n{}", describe_swatch(value, None)).into(),
    );
    sync_docked(ui, picker);
}

fn sync_docked(ui: &AppWindow, picker: &ColorPickerWindow) {
    ui.set_color_original(picker.get_original_color());
    ui.set_color_original_description(picker.get_original_description());
    ui.set_color_current_description(picker.get_current_description());
    ui.set_color_r(picker.get_red());
    ui.set_color_g(picker.get_green());
    ui.set_color_b(picker.get_blue());
    ui.set_color_a(picker.get_alpha());
    ui.set_color_h(picker.get_hue());
    ui.set_color_s(picker.get_saturation());
    ui.set_color_v(picker.get_brightness());
    ui.set_color_hex(picker.get_hex_value());
    ui.set_color_error(picker.get_error());
    ui.set_color_sv(picker.get_sv_image());
    ui.set_screen_picking(picker.get_screen_picking());
}

pub(super) fn describe_workspace_color(value: Color, slot: &str) -> String {
    let name = hoshi_controller::named_colors::name_for(value);
    format!("{slot} color\n{}", describe_swatch(value, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swatch_description_contains_exact_channels_hsv_and_optional_palette_name() {
        let red = Color::rgba(255, 0, 0, 255);
        let unnamed = describe_swatch(red, None);
        assert_eq!(unnamed, "Hex: #FF0000\nRGB: 255, 0, 0\nHSV: 0°, 100%, 100%");
        assert_eq!(describe_swatch(red, Some("Red")), format!("Red\n{unnamed}"));
        assert_eq!(describe_swatch(red, Some("")), unnamed);
        assert!(describe_swatch(Color::rgba(0, 0, 0, 128), None).contains("Hex: #00000080"));
    }

    #[test]
    fn screen_result_recolors_preview_and_cancel_or_failure_preserves_it() {
        i_slint_backend_testing::init_no_event_loop();
        let (ui, state, picker) = crate::create_app_with_colors().unwrap();
        ui.invoke_pointer_down(40.0, 40.0);
        ui.invoke_pointer_move(100.0, 100.0);
        ui.invoke_pointer_up(100.0, 100.0);
        let original = state.borrow().document().active_shape().unwrap();
        let selected = Color::rgba(11, 123, 234, 255);
        picker.set_screen_picking(true);
        finish_screen_pick(&ui.as_weak(), &state, &picker.as_weak(), Ok(Some(selected)));
        assert_eq!(state.borrow().selected_color(), selected);
        assert_eq!(state.borrow().composite()[50 * 512 + 50], selected);
        let shape = state.borrow().document().active_shape().unwrap();
        assert_eq!(
            (shape.x, shape.y, shape.width, shape.height),
            (original.x, original.y, original.width, original.height)
        );
        assert_eq!(state.borrow().document().undo_len(), 0);
        assert!(!picker.get_screen_picking());
        finish_screen_pick(&ui.as_weak(), &state, &picker.as_weak(), Ok(None));
        assert_eq!(state.borrow().document().active_shape(), Some(shape));
        finish_screen_pick(
            &ui.as_weak(),
            &state,
            &picker.as_weak(),
            Err("Permission denied".into()),
        );
        assert_eq!(state.borrow().document().active_shape(), Some(shape));
        assert_eq!(state.borrow().selected_color(), selected);
        assert_eq!(picker.get_error(), "Permission denied");
        assert_eq!(state.borrow().document().undo_len(), 0);
        picker.invoke_set_rgba(100, 80, 60, 255);
        assert_eq!(
            state.borrow().composite()[50 * 512 + 50],
            Color::rgba(100, 80, 60, 255)
        );
        picker.invoke_toggle_dock();
        finish_screen_pick(&ui.as_weak(), &state, &picker.as_weak(), Ok(Some(selected)));
        ui.invoke_edit_rgba(20, 40, 60, 255);
        assert_eq!(
            state.borrow().composite()[50 * 512 + 50],
            Color::rgba(20, 40, 60, 255)
        );
        assert_eq!(ui.get_color_r(), 20);
        assert!(state.borrow().document().active_shape().is_some());
        assert_eq!(state.borrow().tool(), hoshi_controller::Tool::Rectangle);
        assert_eq!(state.borrow().document().undo_len(), 0);
    }
}
