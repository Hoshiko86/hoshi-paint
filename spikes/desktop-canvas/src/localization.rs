//! Locale selection and catalog fallback for the desktop adapter.

use crate::{AppWindow, ColorPickerWindow, HistoryWindow, Translations};
use serde_json::Value;
use slint::ComponentHandle;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

const EN_US: &str = include_str!("../i18n/en-US.json");
const ES_ES: &str = include_str!("../i18n/es-ES.json");
pub(crate) const REQUIRED_KEYS: &[&str] = &[
    "activate_document",
    "all_files",
    "brush",
    "brush_width",
    "cancel",
    "cancel_closing_document",
    "cancel_shape",
    "close_document",
    "commit_shape",
    "confirm_format_and_save",
    "current_state",
    "discard_and_close",
    "dont_save",
    "eraser",
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
    "eyedropper",
    "file_operation_failed",
    "fill",
    "fill_and_outline",
    "finish_shape",
    "fit_canvas",
    "history",
    "hoshi_layered_project",
    "images_hoshi_projects",
    "jpeg_image",
    "jpeg_lossy_warning",
    "jpeg_quality",
    "key_cancel",
    "key_commit",
    "layer",
    "layer_name",
    "ok",
    "open_image",
    "outline",
    "png_image",
    "rectangle",
    "redo",
    "redo_available",
    "reset_view",
    "save",
    "save_as",
    "save_before_closing",
    "save_changes_prefix",
    "save_changes_suffix",
    "save_changes_title",
    "save_configuration",
    "save_failed_preserved",
    "save_flatten_warning",
    "save_format_guidance",
    "save_target_changed",
    "save_target_changed_before_write",
    "screen_picker",
    "shape_properties",
    "shapes",
    "shortcut_redo",
    "shortcut_undo",
    "tool",
    "undo",
    "unit_pixels",
    "unsaved_changes_warning",
    "zoom_in",
    "zoom_out",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SupportedLocale {
    EnglishUnitedStates,
    SpanishSpain,
}

impl SupportedLocale {
    fn tag(self) -> &'static str {
        match self {
            Self::EnglishUnitedStates => "en-US",
            Self::SpanishSpain => "es-ES",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocaleSelection {
    requested: Vec<String>,
    resolved: SupportedLocale,
}

/// Immutable, validated presentation text for one resolved locale.
///
/// Platform adapters receive this narrow capability instead of reading locale
/// environment variables or parsing catalogs themselves.
#[derive(Clone)]
pub(super) struct Localizer {
    locale: SupportedLocale,
    values: Rc<BTreeMap<String, String>>,
    fallback: Rc<BTreeMap<String, String>>,
}

impl Localizer {
    pub(super) fn new(selection: &LocaleSelection) -> Self {
        let fallback = Rc::new(catalog(SupportedLocale::EnglishUnitedStates));
        let values = if selection.resolved == SupportedLocale::EnglishUnitedStates {
            Rc::clone(&fallback)
        } else {
            Rc::new(catalog(selection.resolved))
        };
        Self {
            locale: selection.resolved,
            values,
            fallback,
        }
    }

    pub(super) fn text<'a>(&'a self, key: &'a str) -> &'a str {
        if let Some(value) = self.values.get(key) {
            return value;
        }
        debug_assert!(false, "missing {} catalog key {key}", self.locale.tag());
        self.fallback.get(key).map(String::as_str).unwrap_or(key)
    }

    pub(super) fn format(&self, key: &str, arguments: &[(&str, &str)]) -> String {
        let template = self.text(key);
        let expected = placeholder_names(template);
        debug_assert!(
            expected.is_ok(),
            "invalid {} catalog key {key}: {expected:?}",
            self.locale.tag()
        );
        let expected = expected.unwrap_or_default();
        let supplied: BTreeSet<&str> = arguments.iter().map(|(name, _)| *name).collect();
        debug_assert_eq!(
            supplied.len(),
            arguments.len(),
            "duplicate argument for catalog key {key}"
        );
        debug_assert_eq!(
            supplied,
            expected.iter().map(String::as_str).collect(),
            "arguments do not match placeholders for {} catalog key {key}",
            self.locale.tag()
        );
        format_template(template, arguments)
    }
}

fn format_template(template: &str, arguments: &[(&str, &str)]) -> String {
    let mut value = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        value.push_str(&rest[..open]);
        let placeholder = &rest[open + 1..];
        let Some(close) = placeholder.find('}') else {
            value.push_str(&rest[open..]);
            return value;
        };
        let name = &placeholder[..close];
        let Some(replacement) = arguments
            .iter()
            .find_map(|(candidate, value)| (*candidate == name).then_some(*value))
        else {
            value.push_str(&rest[open..]);
            return value;
        };
        value.push_str(replacement);
        rest = &placeholder[close + 1..];
    }
    value.push_str(rest);
    value
}

fn placeholder_names(template: &str) -> Result<BTreeSet<String>, String> {
    let mut names = BTreeSet::new();
    let mut rest = template;
    while let Some(open) = rest.find(['{', '}']) {
        if rest.as_bytes()[open] == b'}' {
            return Err("closing brace has no opening brace".into());
        }
        let placeholder = &rest[open + 1..];
        let close = placeholder
            .find('}')
            .ok_or_else(|| "placeholder has no closing brace".to_owned())?;
        let name = &placeholder[..close];
        if name.is_empty()
            || !name
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_')
        {
            return Err(format!("invalid placeholder name {{{name}}}"));
        }
        names.insert(name.to_owned());
        rest = &placeholder[close + 1..];
    }
    Ok(names)
}

fn supported_locale(requested: &str) -> Option<SupportedLocale> {
    let normalized = requested.trim().replace('_', "-");
    let language = normalized.split('-').next().unwrap_or_default();
    match language {
        value if value.eq_ignore_ascii_case("es") => Some(SupportedLocale::SpanishSpain),
        value if value.eq_ignore_ascii_case("en") => Some(SupportedLocale::EnglishUnitedStates),
        _ => None,
    }
}

pub fn select_preferred<I, S>(requested: I) -> LocaleSelection
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let requested: Vec<String> = requested
        .into_iter()
        .map(|value| value.as_ref().trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect();
    let resolved = requested
        .iter()
        .find_map(|value| supported_locale(value))
        .unwrap_or(SupportedLocale::EnglishUnitedStates);
    LocaleSelection {
        requested,
        resolved,
    }
}

pub fn system_selection() -> LocaleSelection {
    if let Ok(override_locale) = std::env::var("HOSHI_LOCALE")
        && !override_locale.trim().is_empty()
    {
        return select_preferred([override_locale]);
    }
    select_preferred(sys_locale::get_locales())
}

fn catalog(locale: SupportedLocale) -> BTreeMap<String, String> {
    let source = match locale {
        SupportedLocale::EnglishUnitedStates => EN_US,
        SupportedLocale::SpanishSpain => ES_ES,
    };
    let Value::Object(entries) = serde_json::from_str(source).expect("embedded locale catalog")
    else {
        unreachable!("catalog root is an object")
    };
    let values: BTreeMap<String, String> = entries
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                value
                    .as_str()
                    .expect("catalog values are strings")
                    .to_owned(),
            )
        })
        .collect();
    assert_eq!(
        values.keys().map(String::as_str).collect::<Vec<_>>(),
        REQUIRED_KEYS,
        "embedded {} catalog does not match the required schema",
        locale.tag()
    );
    let Value::Object(english_entries) =
        serde_json::from_str(EN_US).expect("embedded en-US catalog")
    else {
        unreachable!("catalog root is an object")
    };
    for (key, value) in &values {
        let actual = placeholder_names(value)
            .unwrap_or_else(|error| panic!("invalid {} catalog key {key}: {error}", locale.tag()));
        let reference = english_entries[key]
            .as_str()
            .expect("catalog values are strings");
        let expected = placeholder_names(reference)
            .unwrap_or_else(|error| panic!("invalid en-US catalog key {key}: {error}"));
        assert_eq!(
            actual,
            expected,
            "placeholder schema differs for {} catalog key {key}",
            locale.tag()
        );
    }
    values
}

pub fn apply(ui: &AppWindow, localizer: &Localizer) {
    apply_translations(ui.global::<Translations>(), localizer);
}

pub fn apply_history(ui: &HistoryWindow, localizer: &Localizer) {
    apply_translations(ui.global::<Translations>(), localizer);
}

pub fn apply_colors(ui: &ColorPickerWindow, localizer: &Localizer) {
    apply_translations(ui.global::<Translations>(), localizer);
}

fn apply_translations(translations: Translations<'_>, localizer: &Localizer) {
    let text = |key: &str| -> slint::SharedString { localizer.text(key).into() };
    translations.set_locale(localizer.locale.tag().into());
    translations.set_brush(text("brush"));
    translations.set_brush_width(text("brush_width"));
    translations.set_eraser(text("eraser"));
    translations.set_error_history_corrupt(text("error_history_corrupt"));
    translations.set_error_history_io(text("error_history_io"));
    translations.set_error_memory_limit(text("error_memory_limit"));
    translations.set_eyedropper(text("eyedropper"));
    translations.set_screen_picker(text("screen_picker"));
    translations.set_tool(text("tool"));
    translations.set_rectangle(text("rectangle"));
    translations.set_fill(text("fill"));
    translations.set_outline(text("outline"));
    translations.set_fill_and_outline(text("fill_and_outline"));
    translations.set_shapes(text("shapes"));
    translations.set_finish_shape(text("finish_shape"));
    translations.set_undo(text("undo"));
    translations.set_redo(text("redo"));
    translations.set_history(text("history"));
    translations.set_commit_shape(text("commit_shape"));
    translations.set_cancel_shape(text("cancel_shape"));
    translations.set_shape_properties(text("shape_properties"));
    translations.set_zoom_out(text("zoom_out"));
    translations.set_fit_canvas(text("fit_canvas"));
    translations.set_reset_view(text("reset_view"));
    translations.set_zoom_in(text("zoom_in"));
    translations.set_activate_document(text("activate_document"));
    translations.set_close_document(text("close_document"));
    translations.set_layer(text("layer"));
    translations.set_layer_name(text("layer_name"));
    translations.set_unit_pixels(text("unit_pixels"));
    translations.set_key_commit(text("key_commit"));
    translations.set_key_cancel(text("key_cancel"));
    translations.set_shortcut_undo(text("shortcut_undo"));
    translations.set_shortcut_redo(text("shortcut_redo"));
    translations.set_save_changes_title(text("save_changes_title"));
    translations.set_save_changes_prefix(text("save_changes_prefix"));
    translations.set_save_changes_suffix(text("save_changes_suffix"));
    translations.set_unsaved_changes_warning(text("unsaved_changes_warning"));
    translations.set_save(text("save"));
    translations.set_dont_save(text("dont_save"));
    translations.set_cancel(text("cancel"));
    translations.set_save_before_closing(text("save_before_closing"));
    translations.set_discard_and_close(text("discard_and_close"));
    translations.set_cancel_closing_document(text("cancel_closing_document"));
    translations.set_save_configuration(text("save_configuration"));
    translations.set_jpeg_quality(text("jpeg_quality"));
    translations.set_save_format_guidance(text("save_format_guidance"));
    translations.set_confirm_format_and_save(text("confirm_format_and_save"));
    translations.set_file_operation_failed(text("file_operation_failed"));
    translations.set_ok(text("ok"));
    translations.set_current_state(text("current_state"));
    translations.set_redo_available(text("redo_available"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_selection_normalizes_tags_and_falls_back_deterministically() {
        assert_eq!(select_preferred(["es_MX"]).resolved.tag(), "es-ES");
        assert_eq!(select_preferred([" es-ES "]).resolved.tag(), "es-ES");
        assert_eq!(select_preferred(["en-GB"]).resolved.tag(), "en-US");
        assert_eq!(
            select_preferred(["invalid", "es-MX"]).resolved.tag(),
            "es-ES"
        );
        assert_eq!(select_preferred(["invalid"]).resolved.tag(), "en-US");
    }

    #[test]
    fn every_catalog_has_the_same_nonempty_keys() {
        let english = catalog(SupportedLocale::EnglishUnitedStates);
        let spanish = catalog(SupportedLocale::SpanishSpain);
        assert_eq!(
            english.keys().collect::<Vec<_>>(),
            spanish.keys().collect::<Vec<_>>()
        );
        assert!(
            english
                .values()
                .chain(spanish.values())
                .all(|value| !value.trim().is_empty())
        );
        assert_eq!(
            english.keys().map(String::as_str).collect::<Vec<_>>(),
            REQUIRED_KEYS
        );
    }

    #[test]
    fn formatting_replaces_named_arguments_without_reinterpreting_values() {
        let localizer = Localizer::new(&select_preferred(["en-US"]));
        let rendered = localizer.format(
            "save_failed_preserved",
            &[
                ("path", "{error}.png"),
                ("error", "disk {path} unavailable"),
            ],
        );
        assert!(rendered.contains("{error}.png"));
        assert!(rendered.contains("disk {path} unavailable"));
    }

    #[test]
    fn formatting_preserves_remaining_template_when_arguments_or_braces_are_missing() {
        assert_eq!(
            format_template(
                "Saved {path}: {missing}; {error}",
                &[("path", "a.png"), ("error", "disk")]
            ),
            "Saved a.png: {missing}; {error}"
        );
        assert_eq!(
            format_template("Saved {path}: {unclosed", &[("path", "a.png")]),
            "Saved a.png: {unclosed"
        );
    }

    #[test]
    fn placeholder_schema_rejects_malformed_names_and_braces() {
        assert!(placeholder_names("missing {close").is_err());
        assert!(placeholder_names("stray close}").is_err());
        assert!(placeholder_names("bad {Error}").is_err());
        assert_eq!(
            placeholder_names("{path}: {error}; again {path}").unwrap(),
            BTreeSet::from(["error".to_owned(), "path".to_owned()])
        );
    }
}
