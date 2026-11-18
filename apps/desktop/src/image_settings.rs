//! Dialog drafts do not touch the document until Apply succeeds.
use crate::{
    AppWindow, ImageDialog, ImageField, SharedController, localization::Localizer, refresh, update,
};
use hoshi_controller::{Anchor, Color, ControllerError, ImageTransform, Resampling, ResizeOptions};
use hoshi_core::Error as CoreError;
use slint::ComponentHandle;
use std::{cell::RefCell, rc::Rc};

/// Why a dialog value or Apply was rejected. Text lives in the catalogs (#119).
#[derive(Clone, Debug, PartialEq)]
enum SettingsError {
    NotPositive,
    FractionalPixels,
    PixelRange,
    ResolutionRange,
    DocumentLimit,
    /// An out-of-range combo-box index; the UI cannot produce one.
    InvalidChoice,
    Apply(ControllerError),
}

/// Catalog keys this module uses. `localization::REQUIRED_KEYS` must list them.
#[cfg(test)]
const KEYS: &[&str] = &[
    "image_error_apply_limits",
    "image_error_invalid_choice",
    "image_error_not_positive",
    "image_error_pixel_range",
    "image_error_resolution_range",
    "image_error_whole_pixels",
    "image_summary",
];

fn describe(text: &Localizer, error: &SettingsError) -> String {
    match error {
        SettingsError::NotPositive => text.text("image_error_not_positive").into(),
        SettingsError::FractionalPixels => text.text("image_error_whole_pixels").into(),
        SettingsError::PixelRange => text.text("image_error_pixel_range").into(),
        SettingsError::ResolutionRange => text.text("image_error_resolution_range").into(),
        SettingsError::DocumentLimit => text.format(
            "error_document_limit",
            &[("max", &crate::workspace::MAX_DOCUMENTS.to_string())],
        ),
        SettingsError::InvalidChoice => text.text("image_error_invalid_choice").into(),
        SettingsError::Apply(
            error @ ControllerError::Core(
                CoreError::MemoryLimit | CoreError::HistoryIo | CoreError::HistoryCorrupt,
            ),
        ) => crate::error_text::controller(text, error),
        SettingsError::Apply(_) => text.text("image_error_apply_limits").into(),
    }
}

/// The dialog being applied; `ImageDialog::Closed` has no Apply.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenDialog {
    NewImage,
    Resize,
    CanvasSize,
}

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
    fn dimension(&self, value: &str) -> Result<u32, SettingsError> {
        let number = positive(value)?;
        if self.units == 0 && number.fract() != 0.0 {
            return Err(SettingsError::FractionalPixels);
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
    fn sync(&self, ui: &AppWindow, text: &Localizer) {
        ui.set_image_width_text(self.text(self.size.0));
        ui.set_image_height_text(self.text(self.size.1));
        let (width, height) = (self.size.0.to_string(), self.size.1.to_string());
        let width_in = format!("{:.3}", f64::from(self.size.0) / self.resolution);
        let height_in = format!("{:.3}", f64::from(self.size.1) / self.resolution);
        ui.set_image_summary(
            text.format(
                "image_summary",
                &[
                    ("width", &width),
                    ("height", &height),
                    ("width_in", &width_in),
                    ("height_in", &height_in),
                ],
            )
            .into(),
        );
    }
}
fn positive(text: &str) -> Result<f64, SettingsError> {
    let value = text
        .trim()
        .parse::<f64>()
        .map_err(|_| SettingsError::NotPositive)?;
    if !value.is_finite() || value <= 0.0 {
        return Err(SettingsError::NotPositive);
    }
    Ok(value)
}
fn pixel_count(value: f64) -> Result<u32, SettingsError> {
    if !value.is_finite() || value < 0.5 || value.round() > f64::from(u32::MAX) {
        return Err(SettingsError::PixelRange);
    }
    Ok(value.round() as u32)
}
fn percent_size(draft: &Draft, text: &str) -> Result<(u32, u32), SettingsError> {
    let scale = positive(text)? / 100.0;
    Ok((
        pixel_count(f64::from(draft.original.0) * scale)?,
        pixel_count(f64::from(draft.original.1) * scale)?,
    ))
}

pub(super) fn bind(ui: &AppWindow, state: SharedController, text: &Localizer) {
    let draft = Rc::new(RefCell::new(Draft {
        original: (512, 512),
        size: (512, 512),
        resolution: 96.0,
        units: 0,
        ratio: 1.0,
    }));
    let (main, controller, data, words) =
        (ui.as_weak(), state.clone(), draft.clone(), text.clone());
    ui.on_show_image_settings(move |kind| {
        if let Some(ui) = main.upgrade() {
            if ui.get_modal_visible()
                || kind == ImageDialog::Closed
                || (kind != ImageDialog::NewImage && !controller.borrow().has_document())
            {
                return;
            }
            {
                // A drag that cannot finish keeps the canvas as it is; the
                // dialog simply does not open.
                let mut c = controller.borrow_mut();
                if c.pointer_up().is_err() || c.set_aspect_lock(false).is_err() {
                    return;
                }
            }
            ui.invoke_clear_navigation_state();
            crate::panel_hosts::close_adjustment_panels(&ui);
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
            ui.set_image_aspect(kind != ImageDialog::CanvasSize);
            ui.set_image_percentage(false);
            ui.set_image_percent_text("100".into());
            ui.set_image_anchor(4);
            ui.set_image_background(0);
            ui.set_image_resampling(0);
            ui.set_image_gamma_correction(true);
            ui.set_image_resolution_text(document.resolution().to_string().into());
            ui.set_image_error("".into());
            data.borrow().sync(&ui, &words);
            ui.set_image_dialog_kind(kind);
        }
    });
    let (main, data, words) = (ui.as_weak(), draft.clone(), text.clone());
    ui.on_edit_image_setting(move |which, value| {
        let Some(ui) = main.upgrade() else {
            return;
        };
        if ui.get_image_dialog_kind() == ImageDialog::Closed {
            return;
        }
        match which {
            ImageField::Width => ui.set_image_width_text(value.clone()),
            ImageField::Height => ui.set_image_height_text(value.clone()),
            ImageField::Resolution => ui.set_image_resolution_text(value.clone()),
            ImageField::Percent => ui.set_image_percent_text(value.clone()),
        }
        let mut d = data.borrow_mut();
        let result = (|| -> Result<(), SettingsError> {
            match which {
                ImageField::Width => {
                    d.size.0 = d.dimension(&value)?;
                    if ui.get_image_aspect() {
                        d.size.1 = pixel_count(f64::from(d.size.0) / d.ratio)?;
                    }
                }
                ImageField::Height => {
                    d.size.1 = d.dimension(&value)?;
                    if ui.get_image_aspect() {
                        d.size.0 = pixel_count(f64::from(d.size.1) * d.ratio)?;
                    }
                }
                ImageField::Resolution => {
                    let resolution = positive(&value)?;
                    if !(1.0..=100_000.0).contains(&resolution) {
                        return Err(SettingsError::ResolutionRange);
                    }
                    d.resolution = resolution;
                }
                ImageField::Percent => {
                    d.size = percent_size(&d, &value)?;
                }
            }
            let width_text = ui.get_image_width_text();
            let height_text = ui.get_image_height_text();
            d.sync(&ui, &words);
            // Preserve the user's unfinished formatting/caret in the active field.
            match which {
                ImageField::Width => ui.set_image_width_text(width_text),
                ImageField::Height => ui.set_image_height_text(height_text),
                ImageField::Resolution | ImageField::Percent => {}
            }
            Ok(())
        })();
        ui.set_image_error(
            result
                .err()
                .map(|error| describe(&words, &error))
                .unwrap_or_default()
                .into(),
        );
    });
    let (main, data, words) = (ui.as_weak(), draft.clone(), text.clone());
    ui.on_select_image_units(move |units| {
        if let Some(ui) = main.upgrade() {
            data.borrow_mut().units = units.clamp(0, 2);
            ui.set_image_units(units.clamp(0, 2));
            data.borrow().sync(&ui, &words);
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
    let (main, data, words) = (ui.as_weak(), draft.clone(), text.clone());
    ui.on_select_image_percentage(move |percentage| {
        if let Some(ui) = main.upgrade() {
            ui.set_image_percentage(percentage);
            if percentage {
                let result = percent_size(&data.borrow(), &ui.get_image_percent_text());
                match result {
                    Ok(size) => data.borrow_mut().size = size,
                    Err(e) => {
                        ui.set_image_error(describe(&words, &e).into());
                        return;
                    }
                }
            }
            data.borrow().sync(&ui, &words);
        }
    });
    let (main, controller, data, words) = (ui.as_weak(), state.clone(), draft, text.clone());
    ui.on_apply_image_settings(move || {
        let Some(ui) = main.upgrade() else {
            return;
        };
        let kind = match ui.get_image_dialog_kind() {
            ImageDialog::Closed => return,
            ImageDialog::NewImage => OpenDialog::NewImage,
            ImageDialog::Resize => OpenDialog::Resize,
            ImageDialog::CanvasSize => OpenDialog::CanvasSize,
        };
        let result = (|| -> Result<(), SettingsError> {
            let d = data.borrow();
            let (width, height) = if ui.get_image_percentage() && kind == OpenDialog::Resize {
                percent_size(&d, &ui.get_image_percent_text())?
            } else {
                (
                    d.dimension(&ui.get_image_width_text())?,
                    d.dimension(&ui.get_image_height_text())?,
                )
            };
            let resolution = if kind == OpenDialog::CanvasSize {
                d.resolution
            } else {
                positive(&ui.get_image_resolution_text())?
            };
            let mut c = controller.borrow_mut();
            let secondary = c.secondary_color();
            let result = match kind {
                OpenDialog::NewImage => {
                    if c.tabs().len() >= crate::workspace::MAX_DOCUMENTS {
                        return Err(SettingsError::DocumentLimit);
                    }
                    let background = match ui.get_image_background() {
                        0 => Color::rgba(255, 255, 255, 255),
                        1 => Color::rgba(0, 0, 0, 0),
                        2 => secondary,
                        _ => return Err(SettingsError::InvalidChoice),
                    };
                    c.new_image(width, height, resolution, background)
                }
                OpenDialog::Resize => {
                    let method = match ui.get_image_resampling() {
                        0 => Resampling::Bicubic,
                        1 => Resampling::BicubicSmooth,
                        2 => Resampling::Bilinear,
                        3 => Resampling::BilinearLowQuality,
                        4 => Resampling::AdaptiveSharp,
                        5 => Resampling::Lanczos,
                        6 => Resampling::Fant,
                        7 => Resampling::Nearest,
                        _ => return Err(SettingsError::InvalidChoice),
                    };
                    c.resize_image(
                        width,
                        height,
                        resolution,
                        ResizeOptions {
                            method,
                            gamma_correction: ui.get_image_gamma_correction(),
                        },
                    )
                }
                OpenDialog::CanvasSize => {
                    let anchor = ui.get_image_anchor();
                    if !(0..=8).contains(&anchor) {
                        return Err(SettingsError::InvalidChoice);
                    }
                    let fill = match ui.get_image_background() {
                        0 => Color::rgba(0, 0, 0, 0),
                        1 => secondary,
                        _ => return Err(SettingsError::InvalidChoice),
                    };
                    c.resize_canvas(
                        width,
                        height,
                        Anchor {
                            column: (anchor % 3) as u8,
                            row: (anchor / 3) as u8,
                        },
                        fill,
                    )
                }
            };
            result.map_err(SettingsError::Apply)?;
            ui.invoke_close_image_settings();
            refresh(&ui, &c);
            Ok(())
        })();
        if let Err(error) = result {
            ui.set_image_error(describe(&words, &error).into());
        }
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

#[cfg(test)]
mod tests {
    use super::{KEYS, SettingsError, describe};
    use crate::localization::{Localizer, REQUIRED_KEYS, select_preferred};
    use hoshi_controller::ControllerError;
    use hoshi_core::Error as CoreError;

    #[test]
    fn every_image_dialog_key_is_required_in_every_catalog() {
        for key in KEYS {
            assert!(
                REQUIRED_KEYS.contains(key),
                "{key} missing from REQUIRED_KEYS"
            );
        }
    }

    #[test]
    fn image_dialog_errors_follow_the_selected_locale() {
        let english = Localizer::new(&select_preferred(["en-US"]));
        let spanish = Localizer::new(&select_preferred(["es-ES"]));
        assert_eq!(
            describe(&english, &SettingsError::NotPositive),
            "Enter a valid positive number."
        );
        assert_eq!(
            describe(&spanish, &SettingsError::NotPositive),
            "Introduce un número positivo válido."
        );
        assert!(describe(&spanish, &SettingsError::DocumentLimit).contains("32"));
        // Memory and history failures reuse the shared error catalog (#86).
        assert_eq!(
            describe(
                &english,
                &SettingsError::Apply(ControllerError::Core(CoreError::MemoryLimit))
            ),
            english.text("error_memory_limit")
        );
        assert_eq!(
            describe(
                &spanish,
                &SettingsError::Apply(ControllerError::InvalidInput)
            ),
            spanish.text("image_error_apply_limits")
        );
    }
}
