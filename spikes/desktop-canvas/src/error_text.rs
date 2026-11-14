//! User-facing text for adapter and core errors (#86).
//!
//! Error types carry kind and data only; this is the one place that turns
//! them into catalog messages, so every locale gets the same coverage.

use crate::files::FileError;
use crate::localization::Localizer;
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
];

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
        ProjectError::Io(detail) => return text.format("error_file_io", &[("detail", detail)]),
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
}
