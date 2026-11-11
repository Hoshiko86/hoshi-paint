//! Locale selection and catalog fallback for the desktop adapter.

use crate::{AppWindow, Translations};
use serde_json::Value;
use slint::ComponentHandle;
use std::collections::BTreeMap;

const DEFAULT_LOCALE: &str = "en-US";
const EN_US: &str = include_str!("../i18n/en-US.json");
const ES_ES: &str = include_str!("../i18n/es-ES.json");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocaleSelection {
    pub requested: String,
    pub resolved: &'static str,
}

pub fn select(requested: &str) -> LocaleSelection {
    let normalized = requested.replace('_', "-");
    let language = normalized.split('-').next().unwrap_or_default();
    let resolved = match normalized.as_str() {
        "es-ES" => "es-ES",
        "en-US" => "en-US",
        _ if language.eq_ignore_ascii_case("es") => "es-ES",
        _ => DEFAULT_LOCALE,
    };
    LocaleSelection {
        requested: requested.to_owned(),
        resolved,
    }
}

pub fn system_selection() -> LocaleSelection {
    let requested = std::env::var("HOSHI_LOCALE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(sys_locale::get_locale)
        .unwrap_or_else(|| DEFAULT_LOCALE.to_owned());
    select(&requested)
}

fn catalog(locale: &str) -> BTreeMap<String, String> {
    let source = if locale == "es-ES" { ES_ES } else { EN_US };
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
    translations.set_locale(selection.resolved.into());
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_selection_normalizes_tags_and_falls_back_deterministically() {
        assert_eq!(select("es_MX").resolved, "es-ES");
        assert_eq!(select("es-ES").resolved, "es-ES");
        assert_eq!(select("en-GB").resolved, "en-US");
        assert_eq!(select("invalid").resolved, DEFAULT_LOCALE);
    }

    #[test]
    fn every_catalog_has_the_same_nonempty_keys() {
        let english = catalog("en-US");
        let spanish = catalog("es-ES");
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
    }
}
