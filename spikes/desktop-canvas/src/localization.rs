//! Locale selection and catalog fallback for the desktop adapter.

use crate::{AppWindow, Translations};
use serde_json::Value;
use slint::ComponentHandle;
use std::collections::BTreeMap;

const EN_US: &str = include_str!("../i18n/en-US.json");
const ES_ES: &str = include_str!("../i18n/es-ES.json");
const REQUIRED_KEYS: &[&str] = &[
    "activate_document",
    "cancel_shape",
    "close_document",
    "commit_shape",
    "fill",
    "fill_and_outline",
    "finish_shape",
    "fit_canvas",
    "history",
    "key_cancel",
    "key_commit",
    "layer",
    "layer_name",
    "outline",
    "rectangle",
    "redo",
    "reset_view",
    "shape_properties",
    "shapes",
    "shortcut_redo",
    "shortcut_undo",
    "tool",
    "undo",
    "unit_pixels",
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
    if let Ok(override_locale) = std::env::var("HOSHI_LOCALE") {
        if !override_locale.trim().is_empty() {
            return select_preferred([override_locale]);
        }
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
    entries
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
        .collect()
}

pub fn apply(ui: &AppWindow, selection: &LocaleSelection) {
    let values = catalog(selection.resolved);
    let text = |key: &str| -> slint::SharedString {
        values
            .get(key)
            .expect("complete embedded catalog")
            .as_str()
            .into()
    };
    let translations = ui.global::<Translations>();
    translations.set_locale(selection.resolved.tag().into());
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
}
