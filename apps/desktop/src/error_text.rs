//! User-facing text for adapter and core errors (#86).
//!
//! Error types carry kind and data only; this is the one place that turns
//! them into catalog messages, so every locale gets the same coverage.

use crate::files::FileError;
use crate::localization::Localizer;
use crate::palette_store::PaletteStoreError;
use crate::screen_pick::ScreenPickError;
use crate::workspace::WorkspaceError;
use hoshi_controller::ControllerError;
use hoshi_core::{Error as CoreError, MAX_DIMENSION, ProjectError};

/// Catalog keys this module uses. `localization::REQUIRED_KEYS` must list them.
#[cfg(test)]
pub(crate) const KEYS: &[&str] = &[
    "error_action_unavailable",
    "error_commit_shape_before_save",
    "error_document_limit",
    "error_document_missing",
    "error_file_16bit",
    "error_file_channel_depth",
    "error_file_dimensions",
    "error_file_icc",
    "error_file_image",
    "error_file_invalid_image",
    "error_file_invalid_project",
    "error_file_io",
    "error_file_missing_name",
    "error_file_missing_parent",
    "error_file_not_regular",
    "error_file_pdn_open",
    "error_file_pdn_save",
    "error_file_png",
    "error_file_png_color",
    "error_file_png_depth",
    "error_file_png_empty",
    "error_file_png_rows",
    "error_file_png_truncated",
    "error_file_read_only",
    "error_file_save_format",
    "error_flatten_required",
    "error_history_corrupt",
    "error_history_io",
    "error_jpeg_density",
    "error_jpeg_dimensions",
    "error_jpeg_transparency",
    "error_memory_limit",
    "error_no_document_to_save",
    "error_open_limit",
    "error_path_open_in_tab",
    "error_project_checksum",
    "error_project_invalid_data",
    "error_project_invalid_document",
    "error_project_version",
    "palette_error_color",
    "palette_error_folder",
    "palette_error_io",
    "palette_error_is_folder",
    "palette_error_list",
    "palette_error_no_storage",
    "palette_error_not_utf8",
    "palette_error_open_folder",
    "palette_error_read",
    "palette_error_read_only",
    "palette_error_rollback",
    "palette_error_selection",
    "palette_error_settings_destination",
    "palette_error_settings_invalid",
    "palette_error_settings_read",
    "palette_error_settings_version",
    "palette_error_settings_write",
    "palette_error_slot",
    "palette_error_temporary",
    "palette_error_too_large",
    "palette_error_unicode_path",
    "palette_error_unknown_color_name",
    "palette_error_write",
    "screen_pick_error_closed",
    "screen_pick_error_denied",
    "screen_pick_error_invalid_color",
    "screen_pick_error_native",
    "screen_pick_error_portal",
    "screen_pick_error_unsupported",
];

pub(crate) fn palette(text: &Localizer, error: &PaletteStoreError) -> String {
    let detail = |key: &str, error: &dyn std::fmt::Display| {
        text.format(key, &[("error", &error.to_string())])
    };
    let path_detail = |key: &str, path: &std::path::Path, error: &std::io::Error| {
        text.format(
            key,
            &[
                ("path", &path.display().to_string()),
                ("error", &error.to_string()),
            ],
        )
    };
    let key = match error {
        PaletteStoreError::SettingsRead(error) => {
            return detail("palette_error_settings_read", error);
        }
        PaletteStoreError::SettingsWrite(error) => {
            return detail("palette_error_settings_write", error);
        }
        PaletteStoreError::Folder(error) => return detail("palette_error_folder", error),
        PaletteStoreError::List(error) => return detail("palette_error_list", error),
        PaletteStoreError::Io(error) => return detail("palette_error_io", error),
        PaletteStoreError::Read { path, source } => {
            return path_detail("palette_error_read", path, source);
        }
        PaletteStoreError::Write { path, source } => {
            return path_detail("palette_error_write", path, source);
        }
        PaletteStoreError::InvalidColor { line } => {
            return text.format("palette_error_color", &[("line", &line.to_string())]);
        }
        PaletteStoreError::RollbackFailed { error, rollback } => {
            return text.format(
                "palette_error_rollback",
                &[
                    ("error", &palette(text, error)),
                    ("rollback", &palette(text, rollback)),
                ],
            );
        }
        PaletteStoreError::SettingsInvalid => "palette_error_settings_invalid",
        PaletteStoreError::SettingsVersion => "palette_error_settings_version",
        PaletteStoreError::InvalidSlot => "palette_error_slot",
        PaletteStoreError::InvalidSelection => "palette_error_selection",
        PaletteStoreError::NoStorage => "palette_error_no_storage",
        PaletteStoreError::NonUnicodePath => "palette_error_unicode_path",
        PaletteStoreError::TooLarge => "palette_error_too_large",
        PaletteStoreError::NotUtf8 => "palette_error_not_utf8",
        PaletteStoreError::SettingsDestination => "palette_error_settings_destination",
        PaletteStoreError::IsFolder => "palette_error_is_folder",
        PaletteStoreError::ReadOnly => "palette_error_read_only",
    };
    text.text(key).into()
}

pub(crate) fn screen_pick(text: &Localizer, error: &ScreenPickError) -> String {
    let key = match error {
        ScreenPickError::Portal { detail } => {
            return text.format("screen_pick_error_portal", &[("detail", detail)]);
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        ScreenPickError::Unsupported => "screen_pick_error_unsupported",
        ScreenPickError::InvalidColor => "screen_pick_error_invalid_color",
        ScreenPickError::Denied => "screen_pick_error_denied",
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        ScreenPickError::Native { .. } => "screen_pick_error_native",
    };
    text.text(key).into()
}

pub(crate) fn workspace(text: &Localizer, error: &WorkspaceError) -> String {
    let max = crate::workspace::MAX_DOCUMENTS.to_string();
    match error {
        WorkspaceError::Controller(error) => controller(text, error),
        WorkspaceError::File(error) => file(text, error),
        WorkspaceError::MissingDocument => text.text("error_document_missing").into(),
        WorkspaceError::DocumentLimit => text.format("error_document_limit", &[("max", &max)]),
        WorkspaceError::OpenLimit => text.format("error_open_limit", &[("max", &max)]),
        WorkspaceError::NoDocument => text.text("error_no_document_to_save").into(),
        WorkspaceError::PathOpenInAnotherTab => text.text("error_path_open_in_tab").into(),
        WorkspaceError::FlattenRequired => text.text("error_flatten_required").into(),
        WorkspaceError::JpegDensityTooHigh => text.text("error_jpeg_density").into(),
        WorkspaceError::JpegDimensionsTooLarge => text.text("error_jpeg_dimensions").into(),
    }
}

pub(crate) fn file(text: &Localizer, error: &FileError) -> String {
    let detail = |key: &str, detail: &dyn std::fmt::Display| {
        text.format(key, &[("detail", &detail.to_string())])
    };
    let key = match error {
        FileError::Io(error) => return detail("error_file_io", error),
        FileError::Image(error) => return detail("error_file_image", error),
        FileError::PngDecode(error) => return detail("error_file_png", error),
        FileError::PngEncode(error) => return detail("error_file_png", error),
        FileError::InvalidProject(error) => {
            return detail("error_file_invalid_project", &project(text, error));
        }
        FileError::Project(error) => return project(text, error),
        FileError::InvalidDimensions => {
            return text.format(
                "error_file_dimensions",
                &[("max", &MAX_DIMENSION.to_string())],
            );
        }
        FileError::MemoryLimit => "error_memory_limit",
        FileError::InvalidImage(_) | FileError::InvalidPng(_) => "error_file_invalid_image",
        FileError::PdnUnsupported => "error_file_pdn_save",
        FileError::UnsupportedFormat => "error_file_save_format",
        FileError::MissingParent | FileError::MissingSaveParent => "error_file_missing_parent",
        FileError::MissingFilename => "error_file_missing_name",
        FileError::NotARegularFile => "error_file_not_regular",
        FileError::PdnImportUnsupported => "error_file_pdn_open",
        FileError::UnsupportedChannelDepth => "error_file_channel_depth",
        FileError::UnsupportedIccProfile | FileError::UnsupportedPngIccProfile => "error_file_icc",
        FileError::Unsupported16Bit => "error_file_16bit",
        FileError::UnsupportedPngDepth => "error_file_png_depth",
        FileError::TruncatedPngRows => "error_file_png_truncated",
        FileError::InvalidPngRowDimensions => "error_file_png_rows",
        FileError::MissingPngPixels => "error_file_png_empty",
        FileError::UnsupportedPngColorType => "error_file_png_color",
        FileError::ActiveShape => "error_commit_shape_before_save",
        FileError::JpegDensityTooHigh => "error_jpeg_density",
        FileError::JpegDimensionsTooLarge => "error_jpeg_dimensions",
        FileError::JpegTransparency => "error_jpeg_transparency",
        FileError::ReadOnlyDestination => "error_file_read_only",
    };
    text.text(key).into()
}

pub(crate) fn project(text: &Localizer, error: &ProjectError) -> String {
    let key = match error {
        ProjectError::InvalidData => "error_project_invalid_data",
        ProjectError::UnsupportedVersion(version) => {
            return text.format(
                "error_project_version",
                &[("version", &version.to_string())],
            );
        }
        ProjectError::InvalidDocument => "error_project_invalid_document",
        ProjectError::ActiveShape => "error_commit_shape_before_save",
        ProjectError::Checksum => "error_project_checksum",
        ProjectError::MemoryLimit => "error_memory_limit",
        ProjectError::Io(error) => {
            let detail = |key: &str, detail: &dyn std::fmt::Display| {
                text.format(key, &[("detail", &detail.to_string())])
            };
            return detail("error_file_io", error);
        }
    };
    text.text(key).into()
}

pub(crate) fn controller(text: &Localizer, error: &ControllerError) -> String {
    let key = match error {
        ControllerError::Core(CoreError::HistoryIo) => "error_history_io",
        ControllerError::Core(CoreError::HistoryCorrupt) => "error_history_corrupt",
        ControllerError::Core(CoreError::MemoryLimit) => "error_memory_limit",
        other => {
            return text.format(
                "error_action_unavailable",
                &[("detail", &other.to_string())],
            );
        }
    };
    text.text(key).into()
}

#[cfg(test)]
mod tests {
    use super::KEYS;
    use crate::localization::REQUIRED_KEYS;

    #[test]
    fn every_error_key_is_required_in_every_catalog() {
        for key in KEYS {
            assert!(
                REQUIRED_KEYS.contains(key),
                "{key} missing from REQUIRED_KEYS"
            );
        }
    }

    #[test]
    fn palette_errors_follow_the_selected_locale() {
        use crate::localization::{Localizer, select_preferred};
        use crate::palette_store::PaletteStoreError;
        let english = Localizer::new(&select_preferred(["en-US"]));
        let spanish = Localizer::new(&select_preferred(["es-ES"]));
        let color = PaletteStoreError::InvalidColor { line: 7 };
        assert_eq!(
            super::palette(&english, &color),
            "Palette line 7 must contain eight ARGB hexadecimal digits."
        );
        assert_eq!(
            super::palette(&spanish, &color),
            "La línea 7 de la paleta debe contener ocho dígitos hexadecimales ARGB."
        );
        let rollback = PaletteStoreError::RollbackFailed {
            error: Box::new(PaletteStoreError::ReadOnly),
            rollback: Box::new(PaletteStoreError::IsFolder),
        };
        assert_eq!(
            super::palette(&spanish, &rollback),
            "El destino de la paleta es de solo lectura. Tampoco se pudo restaurar el archivo de paleta: Elige un archivo, no una carpeta."
        );
    }

    #[test]
    fn screen_pick_errors_follow_the_selected_locale() {
        use crate::localization::{Localizer, select_preferred};
        use crate::screen_pick::ScreenPickError;
        let spanish = Localizer::new(&select_preferred(["es-ES"]));
        assert_eq!(
            super::screen_pick(&spanish, &ScreenPickError::Denied),
            "El escritorio denegó o no pudo completar el muestreo de pantalla."
        );
        let portal = ScreenPickError::Portal {
            detail: "org.freedesktop.DBus.Error.ServiceUnknown".into(),
        };
        let text = super::screen_pick(&spanish, &portal);
        assert!(text.starts_with("El selector de color necesita"), "{text}");
        assert!(
            text.ends_with("org.freedesktop.DBus.Error.ServiceUnknown"),
            "{text}"
        );
    }
}
