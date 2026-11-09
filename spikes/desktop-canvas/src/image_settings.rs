//! Dialog drafts do not touch the document until Apply succeeds.
use crate::{AppWindow, SharedController, refresh, update};
use hoshi_controller::{Anchor, Color, ImageTransform, Resampling, ResizeOptions};
use slint::ComponentHandle;
use std::{cell::RefCell, rc::Rc};

struct Draft {
    original: (u32, u32),
    size: (u32, u32),
    resolution: f64,
    units: i32,
    ratio: f64,
}
impl Draft {
    fn factor(&self) -> f64 {
        match self.units {
            1 => self.resolution,
            2 => self.resolution / 2.54,
            _ => 1.0,
        }
    }
    fn dimension(&self, value: &str) -> Result<u32, String> {
        let number = positive(value)?;
        if self.units == 0 && number.fract() != 0.0 {
            return Err("Pixel dimensions must be whole numbers.".into());
        }
        pixel_count(number * self.factor())
    }
    fn text(&self, pixels: u32) -> slint::SharedString {
        if self.units == 0 {
            pixels.to_string().into()
        } else {
            format!("{:.6}", f64::from(pixels) / self.factor())
                .trim_end_matches('0')
                .trim_end_matches('.')
                .into()
        }
    }
    fn sync(&self, ui: &AppWindow) {
        ui.set_image_width_text(self.text(self.size.0));
        ui.set_image_height_text(self.text(self.size.1));
        ui.set_image_summary(format!("{} × {} pixels · {:.3} × {:.3} inches\nUp to 262,144 pixels per dimension; memory use depends on image content.", self.size.0, self.size.1, f64::from(self.size.0) / self.resolution, f64::from(self.size.1) / self.resolution).into());
    }
}
fn positive(text: &str) -> Result<f64, String> {
    let value = text
        .trim()
        .parse::<f64>()
        .map_err(|_| "Enter a valid positive number.".to_string())?;
    if !value.is_finite() || value <= 0.0 {
        return Err("Enter a valid positive number.".into());
    }
    Ok(value)
}
fn pixel_count(value: f64) -> Result<u32, String> {
    if !value.is_finite() || value < 0.5 || value.round() > f64::from(u32::MAX) {
        return Err("Dimensions must be at least one pixel and within the image limit.".into());
    }
    Ok(value.round() as u32)
}
fn percent_size(draft: &Draft, text: &str) -> Result<(u32, u32), String> {
    let scale = positive(text)? / 100.0;
    Ok((
        pixel_count(f64::from(draft.original.0) * scale)?,
        pixel_count(f64::from(draft.original.1) * scale)?,
    ))
}

pub(super) fn bind(ui: &AppWindow, state: SharedController) {
    let draft = Rc::new(RefCell::new(Draft {
        original: (512, 512),
        size: (512, 512),
        resolution: 96.0,
        units: 0,
        ratio: 1.0,
    }));
    let (main, controller, data) = (ui.as_weak(), state.clone(), draft.clone());
    ui.on_show_image_settings(move |kind| {
        if let Some(ui) = main.upgrade() {
            if ui.get_modal_visible()
                || !(0..=2).contains(&kind)
                || (kind != 0 && !controller.borrow().has_document())
            {
                return;
            }
            {
                let mut c = controller.borrow_mut();
                c.pointer_up().expect("end gesture before image settings");
                c.set_aspect_lock(false).expect("gesture has ended");
            }
            ui.invoke_clear_navigation_state();
            ui.invoke_hide_colors();
            ui.invoke_close_history();
            let c = controller.borrow();
            let document = c.document();
            let size = (document.width(), document.height());
            *data.borrow_mut() = Draft {
                original: size,
                size,
                resolution: document.resolution(),
                units: 0,
                ratio: f64::from(size.0) / f64::from(size.1),
            };
            ui.set_image_units(0);
            ui.set_image_aspect(kind != 2);
            ui.set_image_percentage(false);
            ui.set_image_percent_text("100".into());
            ui.set_image_anchor(4);
            ui.set_image_background(0);
            ui.set_image_resampling(0);
            ui.set_image_gamma_correction(true);
            ui.set_image_resolution_text(document.resolution().to_string().into());
            ui.set_image_error("".into());
            data.borrow().sync(&ui);
            ui.set_image_dialog_kind(kind);
        }
    });
    let (main, data) = (ui.as_weak(), draft.clone());
    ui.on_edit_image_setting(move |which, value| {
        let Some(ui) = main.upgrade() else {
            return;
        };
        if ui.get_image_dialog_kind() < 0 {
            return;
        }
        match which {
            0 => ui.set_image_width_text(value.clone()),
            1 => ui.set_image_height_text(value.clone()),
            2 => ui.set_image_resolution_text(value.clone()),
            3 => ui.set_image_percent_text(value.clone()),
            _ => return,
        }
        let mut d = data.borrow_mut();
        let result = (|| -> Result<(), String> {
            match which {
                0 => {
                    d.size.0 = d.dimension(&value)?;
                    if ui.get_image_aspect() {
                        d.size.1 = pixel_count(f64::from(d.size.0) / d.ratio)?;
                    }
                }
                1 => {
                    d.size.1 = d.dimension(&value)?;
                    if ui.get_image_aspect() {
                        d.size.0 = pixel_count(f64::from(d.size.1) * d.ratio)?;
                    }
                }
                2 => {
                    let resolution = positive(&value)?;
                    if !(1.0..=100_000.0).contains(&resolution) {
                        return Err("Resolution must be 1–100,000 pixels/inch.".into());
                    }
                    d.resolution = resolution;
                }
                3 => {
                    d.size = percent_size(&d, &value)?;
                }
                _ => unreachable!(),
            }
            let width_text = ui.get_image_width_text();
            let height_text = ui.get_image_height_text();
            d.sync(&ui);
            // Preserve the user's unfinished formatting/caret in the active field.
            if which == 0 {
                ui.set_image_width_text(width_text);
            }
            if which == 1 {
                ui.set_image_height_text(height_text);
            }
            Ok(())
        })();
        ui.set_image_error(result.err().unwrap_or_default().into());
    });
    let (main, data) = (ui.as_weak(), draft.clone());
    ui.on_select_image_units(move |units| {
        if let Some(ui) = main.upgrade() {
            data.borrow_mut().units = units.clamp(0, 2);
            ui.set_image_units(units.clamp(0, 2));
            data.borrow().sync(&ui);
            ui.set_image_error("".into());
        }
    });
    let (main, data) = (ui.as_weak(), draft.clone());
    ui.on_select_image_aspect(move |checked| {
        if let Some(ui) = main.upgrade() {
            let mut d = data.borrow_mut();
            d.ratio = f64::from(d.size.0) / f64::from(d.size.1);
            ui.set_image_aspect(checked);
        }
    });
    let (main, data) = (ui.as_weak(), draft.clone());
    ui.on_select_image_percentage(move |percentage| {
        if let Some(ui) = main.upgrade() {
            ui.set_image_percentage(percentage);
            if percentage {
                let result = percent_size(&data.borrow(), &ui.get_image_percent_text());
                match result {
                    Ok(size) => data.borrow_mut().size = size,
                    Err(e) => {
                        ui.set_image_error(e.into());
                        return;
                    }
                }
            }
            data.borrow().sync(&ui);
        }
    });
    let (main, controller, data) = (ui.as_weak(), state.clone(), draft);
    ui.on_apply_image_settings(move || {
        let Some(ui) = main.upgrade() else { return; };
        let kind = ui.get_image_dialog_kind();
        if !(0..=2).contains(&kind) { return; }
        let result = (|| -> Result<(), String> {
            let d = data.borrow();
            let (width, height) = if ui.get_image_percentage() && kind == 1 { percent_size(&d, &ui.get_image_percent_text())? } else { (d.dimension(&ui.get_image_width_text())?, d.dimension(&ui.get_image_height_text())?) };
            let resolution = if kind == 2 { d.resolution } else { positive(&ui.get_image_resolution_text())? };
            let mut c = controller.borrow_mut();
            let secondary = c.secondary_color();
            let result = match kind {
                0 => {
                    if c.tabs().len() >= crate::workspace::MAX_DOCUMENTS { return Err("Close a document before creating more (prototype limit: 32).".into()); }
                    let background = match ui.get_image_background() { 0 => Color::rgba(255,255,255,255), 1 => Color::rgba(0,0,0,0), 2 => secondary, _ => return Err("Choose a valid background.".into()) };
                    c.new_image(width, height, resolution, background)
                }
                1 => {
                    let method = match ui.get_image_resampling() { 0 => Resampling::Bicubic, 1 => Resampling::BicubicSmooth, 2 => Resampling::Bilinear, 3 => Resampling::BilinearLowQuality, 4 => Resampling::AdaptiveSharp, 5 => Resampling::Lanczos, 6 => Resampling::Fant, 7 => Resampling::Nearest, _ => return Err("Choose a valid resampling method.".into()) };
                    c.resize_image(width, height, resolution, ResizeOptions { method, gamma_correction: ui.get_image_gamma_correction() })
                }
                2 => {
                    let anchor = ui.get_image_anchor();
                    if !(0..=8).contains(&anchor) { return Err("Choose an anchor.".into()); }
                    let fill = match ui.get_image_background() { 0 => Color::rgba(0,0,0,0), 1 => secondary, _ => return Err("Choose a valid fill.".into()) };
                    c.resize_canvas(width, height, Anchor { column: (anchor % 3) as u8, row: (anchor / 3) as u8 }, fill)
                }
                _ => unreachable!(),
            };
            result.map_err(|e| match e {
                hoshi_controller::ControllerError::Core(hoshi_core::Error::MemoryLimit) => "This operation exceeds the shared image-memory budget. Close other images or choose a smaller size.".to_string(),
                hoshi_controller::ControllerError::Core(hoshi_core::Error::HistoryIo | hoshi_core::Error::HistoryCorrupt) => e.to_string(),
                _ => "Cannot apply: dimensions must be 1–262,144 pixels; resolution must be 1–100,000 pixels/inch.".to_string(),
            })?;
            ui.invoke_close_image_settings();
            refresh(&ui, &c);
            Ok(())
        })();
        if let Err(error) = result { ui.set_image_error(error.into()); }
    });
    let (main, controller) = (ui.as_weak(), state);
    ui.on_image_command(move |command| {
        update(&main, &controller, |c| match command {
            0 => c.transform_image(ImageTransform::FlipHorizontal),
            1 => c.transform_image(ImageTransform::FlipVertical),
            2 => c.transform_image(ImageTransform::RotateClockwise),
            3 => c.transform_image(ImageTransform::RotateCounterClockwise),
            4 => c.transform_image(ImageTransform::Rotate180),
            5 => c.flatten(),
            _ => Err(hoshi_controller::ControllerError::InvalidInput),
        });
    });
}
