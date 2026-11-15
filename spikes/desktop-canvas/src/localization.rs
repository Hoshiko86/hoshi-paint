//! Locale selection and catalog fallback for the desktop adapter.

use crate::{AppWindow, ColorPickerWindow, HistoryWindow, ShapePropertiesWindow, Translations};
use serde_json::Value;
use slint::ComponentHandle;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

const EN_US: &str = include_str!("../i18n/en-US.json");
const ES_ES: &str = include_str!("../i18n/es-ES.json");
pub(crate) const REQUIRED_KEYS: &[&str] = &[
    "activate_document",
    "add_to_palette",
    "add_to_palette_tooltip",
    "all_files",
    "alpha",
    "anchor",
    "anchor_bottom",
    "anchor_bottom_left",
    "anchor_bottom_right",
    "anchor_center",
    "anchor_left",
    "anchor_right",
    "anchor_top",
    "anchor_top_left",
    "anchor_top_right",
    "antialiasing",
    "antialiasing_tooltip",
    "apply_image_settings",
    "background",
    "background_secondary",
    "background_transparent",
    "background_white",
    "bounds_error_position",
    "bounds_error_size",
    "bounds_heading",
    "bounds_height",
    "bounds_height_label",
    "bounds_tooltip",
    "bounds_width",
    "bounds_width_label",
    "bounds_x",
    "bounds_y",
    "brush",
    "brush_width",
    "cancel",
    "cancel_closing_document",
    "cancel_shape",
    "canvas_anchor",
    "canvas_label",
    "choose_palette_slot",
    "clear_color_search",
    "clear_recent",
    "cli_usage",
    "close",
    "close_document",
    "close_panel",
    "color_error_hex",
    "color_name",
    "color_name_placeholder",
    "color_name_tooltip",
    "color_palette",
    "colors",
    "colors_heading",
    "colors_tooltip",
    "commit_shape",
    "confirm_format_and_save",
    "create",
    "crop_to_selection",
    "current",
    "current_color",
    "current_state",
    "dialog_canvas_size",
    "dialog_new",
    "dialog_resize",
    "discard_and_close",
    "dock_location",
    "dock_panel",
    "dont_save",
    "editing_primary",
    "editing_secondary",
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
    "export_palette",
    "export_palette_tooltip",
    "eyedropper",
    "file_operation_failed",
    "fill",
    "fill_and_outline",
    "fill_new_area",
    "fill_pattern",
    "fill_pattern_selector",
    "finish_shape",
    "fit_canvas",
    "float_panel",
    "gamma_correction",
    "gamma_tooltip",
    "handle_move_pivot",
    "handle_resize",
    "handle_rotate",
    "height",
    "hex_apply",
    "hex_color",
    "history",
    "history_add_layer",
    "history_brush",
    "history_canvas_size",
    "history_delete_layer",
    "history_ellipse",
    "history_eraser",
    "history_flatten",
    "history_flatten_for_save",
    "history_flip_horizontal",
    "history_flip_vertical",
    "history_layer_properties",
    "history_oldest",
    "history_rectangle",
    "history_rename_layer",
    "history_reorder_layer",
    "history_resize",
    "history_resolution",
    "history_rotate_180",
    "history_rotate_clockwise",
    "history_rotate_counter_clockwise",
    "history_status",
    "history_status_cleanup",
    "history_status_commit_first",
    "history_status_no_image",
    "history_status_no_memory",
    "history_status_select_step",
    "hoshi_layered_project",
    "hsv_range",
    "hue",
    "image_background",
    "image_edit_note",
    "image_error_apply_limits",
    "image_error_invalid_choice",
    "image_error_not_positive",
    "image_error_pixel_range",
    "image_error_resolution_range",
    "image_error_whole_pixels",
    "image_height",
    "image_summary",
    "image_units",
    "image_width",
    "images_hoshi_projects",
    "import_palette",
    "import_palette_tooltip",
    "jpeg_image",
    "jpeg_lossy_warning",
    "jpeg_quality",
    "key_cancel",
    "key_commit",
    "layer",
    "layer_name",
    "line_dash",
    "line_dash_dot",
    "line_dash_dot_dot",
    "line_dot",
    "line_solid",
    "line_style",
    "line_style_selector",
    "lock_ratio",
    "lock_ratio_label",
    "lock_ratio_tooltip",
    "maintain_aspect",
    "menu_about",
    "menu_canvas_size",
    "menu_close",
    "menu_close_all",
    "menu_exit",
    "menu_file",
    "menu_flatten",
    "menu_flip_horizontal",
    "menu_flip_vertical",
    "menu_help",
    "menu_image",
    "menu_new",
    "menu_open",
    "menu_open_recent",
    "menu_resize",
    "menu_rotate_180",
    "menu_rotate_clockwise",
    "menu_rotate_counter_clockwise",
    "menu_save_all",
    "menu_save_as",
    "new_image_note",
    "no_matching_colors",
    "ok",
    "open_image",
    "open_palettes_folder",
    "open_palettes_folder_tooltip",
    "original",
    "original_color",
    "outline",
    "outline_width",
    "outline_width_label",
    "outline_width_tooltip",
    "palette_default",
    "palette_dialog_file_name",
    "palette_dialog_filter",
    "palette_dialog_import",
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
    "palette_size_tooltip",
    "pattern_backward_diagonal",
    "pattern_checkerboard",
    "pattern_cross",
    "pattern_diagonal_cross",
    "pattern_forward_diagonal",
    "pattern_horizontal",
    "pattern_solid",
    "pattern_vertical",
    "picker_editing_shape",
    "picker_next_color",
    "pixels_per_inch",
    "placement_bottom",
    "placement_float",
    "placement_left",
    "placement_right",
    "placement_top",
    "png_image",
    "primary_color",
    "rectangle",
    "redo",
    "redo_available",
    "resample_adaptive",
    "resample_bicubic",
    "resample_bicubic_smooth",
    "resample_bilinear",
    "resample_bilinear_low",
    "resample_fant",
    "resample_lanczos",
    "resample_nearest",
    "resampling",
    "resampling_label",
    "reset_colors",
    "reset_colors_label",
    "reset_palette",
    "reset_palette_tooltip",
    "reset_resampling",
    "reset_view",
    "resize_mode",
    "resize_percentage",
    "resolution",
    "resolution_label",
    "restore",
    "restore_original_color",
    "rgb_range",
    "saturation_value_area",
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
    "save_palette_as",
    "save_palette_as_tooltip",
    "save_target_changed",
    "save_target_changed_before_write",
    "screen_pick_error_closed",
    "screen_pick_error_denied",
    "screen_pick_error_invalid_color",
    "screen_pick_error_native",
    "screen_pick_error_portal",
    "screen_pick_error_unsupported",
    "screen_pick_prompt",
    "screen_picker",
    "secondary_color",
    "shape_heading",
    "shape_kind",
    "shape_kind_tooltip",
    "shape_properties",
    "shape_style",
    "shape_style_tooltip",
    "shapes",
    "shortcut_redo",
    "shortcut_undo",
    "show_all_96",
    "show_first_32",
    "size_absolute",
    "size_percentage",
    "status_document",
    "status_no_image",
    "status_shape",
    "style_heading",
    "swap_colors",
    "swap_colors_label",
    "tab_unsaved",
    "toggle_dock_tooltip",
    "tool",
    "tools_heading",
    "undo",
    "unit_name_centimeters",
    "unit_name_inches",
    "unit_name_pixels",
    "unit_pixels",
    "units",
    "unsaved_changes_warning",
    "untitled",
    "untitled_numbered",
    "use_color",
    "width",
    "window_title",
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

pub fn apply_shapes(ui: &ShapePropertiesWindow, localizer: &Localizer) {
    apply_translations(ui.global::<Translations>(), localizer);
}

fn apply_translations(translations: Translations<'_>, localizer: &Localizer) {
    let text = |key: &str| -> slint::SharedString { localizer.text(key).into() };
    translations.set_locale(localizer.locale.tag().into());
    translations.set_dialog_new(text("dialog_new"));
    translations.set_dialog_resize(text("dialog_resize"));
    translations.set_dialog_canvas_size(text("dialog_canvas_size"));
    translations.set_size_absolute(text("size_absolute"));
    translations.set_size_percentage(text("size_percentage"));
    translations.set_resize_mode(text("resize_mode"));
    translations.set_resize_percentage(text("resize_percentage"));
    translations.set_units(text("units"));
    translations.set_unit_name_pixels(text("unit_name_pixels"));
    translations.set_unit_name_inches(text("unit_name_inches"));
    translations.set_unit_name_centimeters(text("unit_name_centimeters"));
    translations.set_image_units(text("image_units"));
    translations.set_width(text("width"));
    translations.set_height(text("height"));
    translations.set_image_width(text("image_width"));
    translations.set_image_height(text("image_height"));
    translations.set_maintain_aspect(text("maintain_aspect"));
    translations.set_resolution(text("resolution"));
    translations.set_resolution_label(text("resolution_label"));
    translations.set_pixels_per_inch(text("pixels_per_inch"));
    translations.set_resampling(text("resampling"));
    translations.set_resample_bicubic(text("resample_bicubic"));
    translations.set_resample_bicubic_smooth(text("resample_bicubic_smooth"));
    translations.set_resample_bilinear(text("resample_bilinear"));
    translations.set_resample_bilinear_low(text("resample_bilinear_low"));
    translations.set_resample_adaptive(text("resample_adaptive"));
    translations.set_resample_lanczos(text("resample_lanczos"));
    translations.set_resample_fant(text("resample_fant"));
    translations.set_resample_nearest(text("resample_nearest"));
    translations.set_resampling_label(text("resampling_label"));
    translations.set_reset_resampling(text("reset_resampling"));
    translations.set_gamma_correction(text("gamma_correction"));
    translations.set_gamma_tooltip(text("gamma_tooltip"));
    translations.set_anchor(text("anchor"));
    translations.set_anchor_top_left(text("anchor_top_left"));
    translations.set_anchor_top(text("anchor_top"));
    translations.set_anchor_top_right(text("anchor_top_right"));
    translations.set_anchor_left(text("anchor_left"));
    translations.set_anchor_center(text("anchor_center"));
    translations.set_anchor_right(text("anchor_right"));
    translations.set_anchor_bottom_left(text("anchor_bottom_left"));
    translations.set_anchor_bottom(text("anchor_bottom"));
    translations.set_anchor_bottom_right(text("anchor_bottom_right"));
    translations.set_canvas_anchor(text("canvas_anchor"));
    translations.set_background(text("background"));
    translations.set_fill_new_area(text("fill_new_area"));
    translations.set_background_white(text("background_white"));
    translations.set_background_transparent(text("background_transparent"));
    translations.set_background_secondary(text("background_secondary"));
    translations.set_image_background(text("image_background"));
    translations.set_new_image_note(text("new_image_note"));
    translations.set_image_edit_note(text("image_edit_note"));
    translations.set_create(text("create"));
    translations.set_apply_image_settings(text("apply_image_settings"));
    translations.set_placement_float(text("placement_float"));
    translations.set_placement_left(text("placement_left"));
    translations.set_placement_right(text("placement_right"));
    translations.set_placement_top(text("placement_top"));
    translations.set_placement_bottom(text("placement_bottom"));
    translations.set_dock_location(text("dock_location"));
    translations.set_toggle_dock_tooltip(text("toggle_dock_tooltip"));
    translations.set_float_panel(text("float_panel"));
    translations.set_dock_panel(text("dock_panel"));
    translations.set_close_panel(text("close_panel"));
    translations.set_swap_colors_label(text("swap_colors_label"));
    translations.set_reset_colors_label(text("reset_colors_label"));
    translations.set_editing_primary(text("editing_primary"));
    translations.set_editing_secondary(text("editing_secondary"));
    translations.set_original(text("original"));
    translations.set_current(text("current"));
    translations.set_original_color(text("original_color"));
    translations.set_current_color(text("current_color"));
    translations.set_restore(text("restore"));
    translations.set_restore_original_color(text("restore_original_color"));
    translations.set_saturation_value_area(text("saturation_value_area"));
    translations.set_hue(text("hue"));
    translations.set_alpha(text("alpha"));
    translations.set_rgb_range(text("rgb_range"));
    translations.set_hsv_range(text("hsv_range"));
    translations.set_hex_color(text("hex_color"));
    translations.set_hex_apply(text("hex_apply"));
    translations.set_picker_editing_shape(text("picker_editing_shape"));
    translations.set_picker_next_color(text("picker_next_color"));
    translations.set_close(text("close"));
    translations.set_color_name_placeholder(text("color_name_placeholder"));
    translations.set_color_name(text("color_name"));
    translations.set_color_name_tooltip(text("color_name_tooltip"));
    translations.set_clear_color_search(text("clear_color_search"));
    translations.set_no_matching_colors(text("no_matching_colors"));
    translations.set_use_color(text("use_color"));
    translations.set_color_palette(text("color_palette"));
    translations.set_show_first_32(text("show_first_32"));
    translations.set_show_all_96(text("show_all_96"));
    translations.set_palette_size_tooltip(text("palette_size_tooltip"));
    translations.set_add_to_palette(text("add_to_palette"));
    translations.set_add_to_palette_tooltip(text("add_to_palette_tooltip"));
    translations.set_save_palette_as(text("save_palette_as"));
    translations.set_save_palette_as_tooltip(text("save_palette_as_tooltip"));
    translations.set_import_palette(text("import_palette"));
    translations.set_import_palette_tooltip(text("import_palette_tooltip"));
    translations.set_export_palette(text("export_palette"));
    translations.set_export_palette_tooltip(text("export_palette_tooltip"));
    translations.set_open_palettes_folder(text("open_palettes_folder"));
    translations.set_open_palettes_folder_tooltip(text("open_palettes_folder_tooltip"));
    translations.set_reset_palette(text("reset_palette"));
    translations.set_reset_palette_tooltip(text("reset_palette_tooltip"));
    translations.set_choose_palette_slot(text("choose_palette_slot"));
    translations.set_handle_move_pivot(text("handle_move_pivot"));
    translations.set_handle_rotate(text("handle_rotate"));
    translations.set_handle_resize(text("handle_resize"));
    translations.set_bounds_tooltip(text("bounds_tooltip"));
    translations.set_bounds_x(text("bounds_x"));
    translations.set_bounds_y(text("bounds_y"));
    translations.set_bounds_width_label(text("bounds_width_label"));
    translations.set_bounds_height_label(text("bounds_height_label"));
    translations.set_bounds_width(text("bounds_width"));
    translations.set_bounds_height(text("bounds_height"));
    translations.set_lock_ratio(text("lock_ratio"));
    translations.set_lock_ratio_label(text("lock_ratio_label"));
    translations.set_lock_ratio_tooltip(text("lock_ratio_tooltip"));
    translations.set_shape_heading(text("shape_heading"));
    translations.set_bounds_heading(text("bounds_heading"));
    translations.set_style_heading(text("style_heading"));
    translations.set_shape_kind(text("shape_kind"));
    translations.set_shape_kind_tooltip(text("shape_kind_tooltip"));
    translations.set_shape_style(text("shape_style"));
    translations.set_shape_style_tooltip(text("shape_style_tooltip"));
    translations.set_fill_pattern_selector(text("fill_pattern_selector"));
    translations.set_line_style_selector(text("line_style_selector"));
    translations.set_fill_pattern(text("fill_pattern"));
    translations.set_line_style(text("line_style"));
    translations.set_outline_width_label(text("outline_width_label"));
    translations.set_outline_width(text("outline_width"));
    translations.set_outline_width_tooltip(text("outline_width_tooltip"));
    translations.set_line_solid(text("line_solid"));
    translations.set_line_dash(text("line_dash"));
    translations.set_line_dot(text("line_dot"));
    translations.set_line_dash_dot(text("line_dash_dot"));
    translations.set_line_dash_dot_dot(text("line_dash_dot_dot"));
    translations.set_pattern_solid(text("pattern_solid"));
    translations.set_pattern_horizontal(text("pattern_horizontal"));
    translations.set_pattern_vertical(text("pattern_vertical"));
    translations.set_pattern_forward_diagonal(text("pattern_forward_diagonal"));
    translations.set_pattern_backward_diagonal(text("pattern_backward_diagonal"));
    translations.set_pattern_cross(text("pattern_cross"));
    translations.set_pattern_diagonal_cross(text("pattern_diagonal_cross"));
    translations.set_pattern_checkerboard(text("pattern_checkerboard"));
    translations.set_antialiasing(text("antialiasing"));
    translations.set_antialiasing_tooltip(text("antialiasing_tooltip"));
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
    translations.set_canvas_label(text("canvas_label"));
    translations.set_menu_file(text("menu_file"));
    translations.set_menu_new(text("menu_new"));
    translations.set_menu_open(text("menu_open"));
    translations.set_menu_open_recent(text("menu_open_recent"));
    translations.set_menu_save_as(text("menu_save_as"));
    translations.set_menu_save_all(text("menu_save_all"));
    translations.set_menu_close_all(text("menu_close_all"));
    translations.set_menu_exit(text("menu_exit"));
    translations.set_menu_image(text("menu_image"));
    translations.set_menu_resize(text("menu_resize"));
    translations.set_menu_canvas_size(text("menu_canvas_size"));
    translations.set_menu_flip_horizontal(text("menu_flip_horizontal"));
    translations.set_menu_flip_vertical(text("menu_flip_vertical"));
    translations.set_menu_rotate_clockwise(text("menu_rotate_clockwise"));
    translations.set_menu_rotate_counter_clockwise(text("menu_rotate_counter_clockwise"));
    translations.set_menu_rotate_180(text("menu_rotate_180"));
    translations.set_menu_flatten(text("menu_flatten"));
    translations.set_menu_help(text("menu_help"));
    translations.set_menu_about(text("menu_about"));
    translations.set_tools_heading(text("tools_heading"));
    translations.set_colors_heading(text("colors_heading"));
    translations.set_colors(text("colors"));
    translations.set_menu_close(text("menu_close"));
    translations.set_clear_recent(text("clear_recent"));
    translations.set_colors_tooltip(text("colors_tooltip"));
    translations.set_crop_to_selection(text("crop_to_selection"));
    translations.set_primary_color(text("primary_color"));
    translations.set_reset_colors(text("reset_colors"));
    translations.set_secondary_color(text("secondary_color"));
    translations.set_swap_colors(text("swap_colors"));
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
    fn floating_shape_properties_window_follows_the_selected_locale() {
        use slint::ComponentHandle;
        i_slint_backend_testing::init_no_event_loop();
        let window = ShapePropertiesWindow::new().unwrap();
        apply_shapes(&window, &Localizer::new(&select_preferred(["es-ES"])));
        let translations = window.global::<Translations>();
        assert_eq!(translations.get_bounds_width(), "Ancho de la forma");
        assert_eq!(translations.get_line_dash(), "Guiones");
        assert_eq!(translations.get_antialiasing(), "Suavizado");
    }

    #[test]
    fn color_window_labels_follow_the_selected_locale() {
        i_slint_backend_testing::init_no_event_loop();
        let window = ColorPickerWindow::new().unwrap();
        apply_colors(&window, &Localizer::new(&select_preferred(["es-ES"])));
        for label in [
            "Restaurar el color original",
            "Importar paleta",
            "Colores · ubicación del panel",
        ] {
            assert_eq!(
                i_slint_backend_testing::ElementHandle::find_by_accessible_label(&window, label)
                    .count(),
                1,
                "{label}"
            );
        }
    }

    #[test]
    fn image_dialog_labels_follow_the_selected_locale() {
        use slint::ComponentHandle;
        i_slint_backend_testing::init_no_event_loop();
        let window = AppWindow::new().unwrap();
        apply(&window, &Localizer::new(&select_preferred(["es-ES"])));
        window
            .window()
            .set_size(slint::LogicalSize::new(1120., 760.));
        window.show().unwrap();
        window.set_image_dialog_kind(2);
        for label in [
            "Aplicar la configuración de imagen",
            "Anclaje del lienzo Arriba a la izquierda",
            "Relleno del fondo de la imagen",
        ] {
            assert_eq!(
                i_slint_backend_testing::ElementHandle::find_by_accessible_label(&window, label)
                    .count(),
                1,
                "{label}"
            );
        }
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
