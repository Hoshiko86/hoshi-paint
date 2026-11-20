//! Complete semantic palettes, parsed before any window is updated.
use crate::{
    AppWindow, ColorPickerWindow, DesktopTheme, HistoryWindow, Palette, ShapePropertiesWindow,
};
use serde_json::{Map, Value};
use slint::{Color, ComponentHandle, language::ColorScheme};
use std::collections::BTreeMap;

/// Standard-widget and semantic-palette scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scheme {
    /// Compiled dark palette.
    Dark,
    /// Built-in light palette.
    Light,
}
/// A fully resolved palette; every semantic token is present.
#[derive(Debug)]
pub(crate) struct Theme {
    scheme: Scheme,
    colors: [Color; N],
}
macro_rules! tokens {
    ($($field:ident: $variant:ident => ($key:literal, $setter:ident)),* $(,)?) => {
        const N: usize = [$(stringify!($variant)),*].len();
        /// A semantic color role, independent of its concrete palette value.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(crate) enum Token {
            $(/// A color role defined in the Slint theme global.
            $variant),*
        }
        impl Token {
            /// Complete token vocabulary, in palette storage order.
            pub(crate) const ALL: [Token; N] = [$(Token::$variant),*];
            fn key(self) -> &'static str {
                match self { $(Token::$variant => $key),* }
            }
        }
        impl std::ops::Index<Token> for Theme {
            type Output = Color;
            fn index(&self, token: Token) -> &Color {
                // Destructuring proves the enum-to-storage mapping is complete.
                let [$($field),*] = &self.colors;
                match token { $(Token::$variant => $field),* }
            }
        }
        fn apply_colors(theme: &Theme, global: &DesktopTheme<'_>) {
            for token in Token::ALL {
                let color = theme[token];
                match token { $(Token::$variant => global.$setter(color)),* }
            }
        }
    };
}
tokens! {
    window_background: WindowBackground => ("window-background", set_window_background),
    workspace_background: WorkspaceBackground => ("workspace-background", set_workspace_background),
    pasteboard_background: PasteboardBackground => ("pasteboard-background", set_pasteboard_background),
    panel_background: PanelBackground => ("panel-background", set_panel_background),
    raised_background: RaisedBackground => ("raised-background", set_raised_background),
    tab_background: TabBackground => ("tab-background", set_tab_background),
    border: Border => ("border", set_border),
    strong_border: StrongBorder => ("strong-border", set_strong_border),
    accent: Accent => ("accent", set_accent),
    selected_tool: SelectedTool => ("selected-tool", set_selected_tool),
    text: Text => ("text", set_text),
    muted_text: MutedText => ("muted-text", set_muted_text),
    subtle_text: SubtleText => ("subtle-text", set_subtle_text),
    status_text: StatusText => ("status-text", set_status_text),
    error_text: ErrorText => ("error-text", set_error_text),
    panel_border: PanelBorder => ("panel-border", set_panel_border),
    field_background: FieldBackground => ("field-background", set_field_background),
    field_hover: FieldHover => ("field-hover", set_field_hover),
    field_disabled: FieldDisabled => ("field-disabled", set_field_disabled),
    field_border: FieldBorder => ("field-border", set_field_border),
    focus_ring: FocusRing => ("focus-ring", set_focus_ring),
    highlight_background: HighlightBackground => ("highlight-background", set_highlight_background),
    hover_background: HoverBackground => ("hover-background", set_hover_background),
    preview_ink: PreviewInk => ("preview-ink", set_preview_ink),
    checker_light: CheckerLight => ("checker-light", set_checker_light),
    checker_dark: CheckerDark => ("checker-dark", set_checker_dark),
    scrim: Scrim => ("scrim", set_scrim),
    handle_stroke: HandleStroke => ("handle-stroke", set_handle_stroke),
    handle_fill: HandleFill => ("handle-fill", set_handle_fill),
    handle_pivot_fill: HandlePivotFill => ("handle-pivot-fill", set_handle_pivot_fill),
}

/// Theme syntax and resolution failures, before any palette is applied.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ThemeError {
    /// Malformed JSON, preserving the parser's source error.
    #[error("theme json: {0}")]
    Json(#[from] serde_json::Error),
    /// Unsupported format version; zero also denotes a missing version.
    #[error("theme format {0}")]
    Format(u64),
    /// A token name is not part of the semantic vocabulary.
    #[error("unknown theme token {0}")]
    UnknownToken(String),
    /// A token refers to an undefined palette entry.
    #[error("unknown theme palette name {0}")]
    UnknownPaletteName(String),
    /// A color value has invalid syntax or shape.
    #[error("bad theme color {token}: {value}")]
    BadColor { token: String, value: String },
    /// Neither the input nor its base defines this token.
    #[error("missing theme token {0}")]
    MissingToken(&'static str),
    /// The scheme or named base is not supported.
    #[error("bad theme scheme {0}")]
    BadScheme(String),
}
fn scheme(value: &Value) -> Result<Scheme, ThemeError> {
    match value.as_str() {
        Some("dark") => Ok(Scheme::Dark),
        Some("light") => Ok(Scheme::Light),
        _ => Err(ThemeError::BadScheme(
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned),
        )),
    }
}
fn color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let [a, b, c, d] = u32::from_str_radix(hex, 16).ok()?.to_be_bytes();
    Some(if hex.len() == 6 {
        Color::from_argb_u8(255, b, c, d)
    } else {
        Color::from_argb_u8(d, a, b, c)
    })
}
fn bad_color(token: &str, value: &Value) -> ThemeError {
    ThemeError::BadColor {
        token: token.to_owned(),
        value: value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned),
    }
}
fn palette(value: Option<&Value>) -> Result<BTreeMap<&str, Color>, ThemeError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let Some(entries) = value.as_object() else {
        return Err(bad_color("palette", value));
    };
    entries
        .iter()
        .map(|(name, value)| {
            let parsed = value
                .as_str()
                .and_then(color)
                .ok_or_else(|| bad_color(name, value))?;
            Ok((name.as_str(), parsed))
        })
        .collect()
}
fn resolve(
    token: Token,
    value: &Value,
    palette: &BTreeMap<&str, Color>,
) -> Result<Color, ThemeError> {
    let Some(text) = value.as_str() else {
        return Err(bad_color(token.key(), value));
    };
    if text.starts_with('#') {
        color(text).ok_or_else(|| bad_color(token.key(), value))
    } else {
        palette
            .get(text)
            .copied()
            .ok_or_else(|| ThemeError::UnknownPaletteName(text.to_owned()))
    }
}
/// Resolve a complete version-one theme, optionally inheriting a resolved base.
pub(crate) fn parse(json: &str, base: Option<&Theme>) -> Result<Theme, ThemeError> {
    let value: Value = serde_json::from_str(json)?;
    let version = value.get("format").and_then(Value::as_u64).unwrap_or(0);
    if version != 1 {
        return Err(ThemeError::Format(version));
    }
    let scheme = scheme(value.get("scheme").unwrap_or(&Value::Null))?;
    let named = value
        .get("base")
        .map(|v| self::scheme(v).and_then(builtin))
        .transpose()?;
    let base = named.as_ref().or(base);
    let palette = palette(value.get("palette"))?;
    let tokens: Option<&Map<String, Value>> = match value.get("tokens") {
        None => None,
        Some(Value::Object(tokens)) => Some(tokens),
        Some(value) => return Err(bad_color("tokens", value)),
    };
    if let Some(tokens) = tokens {
        for name in tokens.keys() {
            if !Token::ALL.into_iter().any(|token| token.key() == name) {
                return Err(ThemeError::UnknownToken(name.to_owned()));
            }
        }
    }
    let mut colors = [Color::default(); N];
    for (token, slot) in Token::ALL.into_iter().zip(colors.iter_mut()) {
        *slot = if let Some(value) = tokens.and_then(|t| t.get(token.key())) {
            resolve(token, value, &palette)?
        } else if let Some(base) = base {
            base[token]
        } else {
            return Err(ThemeError::MissingToken(token.key()));
        };
    }
    Ok(Theme { scheme, colors })
}
/// Parse an embedded complete palette; callers keep defaults on failure.
pub(crate) fn builtin(scheme: Scheme) -> Result<Theme, ThemeError> {
    parse(
        match scheme {
            Scheme::Dark => include_str!("../themes/dark.json"),
            Scheme::Light => include_str!("../themes/light.json"),
        },
        None,
    )
}
/// Apply every semantic token and the matching standard-widget scheme to all roots.
pub(crate) fn apply(
    theme: &Theme,
    ui: &AppWindow,
    history: &HistoryWindow,
    colors: &ColorPickerWindow,
    shapes: &ShapePropertiesWindow,
) {
    for global in [
        ui.global::<DesktopTheme>(),
        history.global::<DesktopTheme>(),
        colors.global::<DesktopTheme>(),
        shapes.global::<DesktopTheme>(),
    ] {
        apply_colors(theme, &global);
    }
    let scheme = match theme.scheme {
        Scheme::Dark => ColorScheme::Dark,
        Scheme::Light => ColorScheme::Light,
    };
    for palette in [
        ui.global::<Palette>(),
        history.global::<Palette>(),
        colors.global::<Palette>(),
        shapes.global::<Palette>(),
    ] {
        palette.set_color_scheme(scheme);
    }
}
/// Read the untouched standard-widget palette before the first theme application.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "OS-following start returns with #244")
)]
pub(crate) fn system_scheme(ui: &AppWindow) -> Scheme {
    match ui.global::<Palette>().get_color_scheme() {
        ColorScheme::Light => Scheme::Light,
        // ColorScheme is non-exhaustive; unknown and future schemes use Dark.
        _ => Scheme::Dark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeSet, error::Error};

    fn declarations() -> BTreeMap<&'static str, &'static str> {
        include_str!("../ui/theme.slint")
            .lines()
            .filter_map(|line| {
                let line = line.trim().strip_prefix("in-out property <color> ")?;
                let (key, value) = line.split_once(':')?;
                Some((key, value.trim().trim_end_matches(';')))
            })
            .collect()
    }

    #[test]
    fn token_vocabulary_builtins_and_compiled_dark_defaults_match() {
        let source = declarations();
        let keys = Token::ALL
            .into_iter()
            .map(Token::key)
            .collect::<BTreeSet<_>>();
        assert_eq!(keys.len(), N);
        assert_eq!(keys, source.keys().copied().collect());
        for input in [
            include_str!("../themes/dark.json"),
            include_str!("../themes/light.json"),
        ] {
            let value: Value = serde_json::from_str(input).unwrap();
            let actual: BTreeSet<_> = value["tokens"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(keys, actual);
            assert!(parse(input, None).is_ok());
        }
        let dark = builtin(Scheme::Dark).unwrap();
        for token in Token::ALL {
            assert_eq!(
                color(source[token.key()]).unwrap(),
                dark[token],
                "{}",
                token.key()
            );
        }
    }
    fn luminance(color: Color) -> f64 {
        [color.red(), color.green(), color.blue()]
            .into_iter()
            .map(|v| f64::from(v) / 255.)
            .map(|v| {
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            })
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(v, weight)| v * weight)
            .sum()
    }
    fn contrast(a: Color, b: Color) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    #[test]
    fn builtins_meet_every_required_text_and_component_contrast_floor() {
        use Token::*;
        assert!(
            (contrast(color("#000000").unwrap(), color("#ffffff").unwrap()) - 21.).abs() < 0.000001
        );
        let pairs = [
            (Text, PanelBackground, 4.5),
            (MutedText, PanelBackground, 4.5),
            (SubtleText, PanelBackground, 4.5),
            (ErrorText, PanelBackground, 4.5),
            (Text, FieldBackground, 4.5),
            (Text, HighlightBackground, 4.5),
            (Text, HoverBackground, 4.5),
            (Text, TabBackground, 4.5),
            (SubtleText, FieldDisabled, 4.5),
            (StatusText, WindowBackground, 4.5),
            (MutedText, WorkspaceBackground, 4.5),
            (FieldBorder, PanelBackground, 3.),
            (PanelBorder, PanelBackground, 3.),
            (FocusRing, PanelBackground, 3.),
            (FocusRing, FieldBackground, 3.),
            (PreviewInk, FieldBackground, 3.),
        ];
        for scheme in [Scheme::Dark, Scheme::Light] {
            let theme = builtin(scheme).unwrap();
            for (foreground, background, floor) in pairs {
                let actual = contrast(theme[foreground], theme[background]);
                assert!(
                    actual >= floor,
                    "{scheme:?} {} on {}: {actual} < {floor}",
                    foreground.key(),
                    background.key()
                );
            }
        }
    }
    #[test]
    fn parser_preserves_all_error_kinds_and_the_json_source() {
        let json = parse("{", None).unwrap_err();
        assert!(matches!(json, ThemeError::Json(_)));
        assert!(json.source().is_some());
        assert!(matches!(
            parse(r#"{"format":2}"#, None),
            Err(ThemeError::Format(2))
        ));
        assert!(
            matches!(parse(r##"{"format":1,"scheme":"dark","tokens":{"typo":"#112233"}}"##, None), Err(ThemeError::UnknownToken(name)) if name == "typo")
        );
        assert!(
            matches!(parse(r#"{"format":1,"scheme":"dark","tokens":{"window-background":"missing"}}"#, None), Err(ThemeError::UnknownPaletteName(name)) if name == "missing")
        );
        assert!(
            matches!(parse(r##"{"format":1,"scheme":"dark","tokens":{"window-background":"#12"}}"##, None), Err(ThemeError::BadColor { token, value }) if token == "window-background" && value == "#12")
        );
        assert!(matches!(
            parse(r#"{"format":1,"scheme":"dark","tokens":{}}"#, None),
            Err(ThemeError::MissingToken("window-background"))
        ));
        assert!(
            matches!(parse(r#"{"format":1,"scheme":"sepia"}"#, None), Err(ThemeError::BadScheme(name)) if name == "sepia")
        );
        assert!(
            matches!(parse(r#"{"format":1,"scheme":"dark","base":"sepia"}"#, None), Err(ThemeError::BadScheme(name)) if name == "sepia")
        );
    }
    #[test]
    fn palette_references_and_both_base_paths_resolve_without_losing_alpha() {
        let dark = builtin(Scheme::Dark).unwrap();
        let parsed = parse(r##"{"format":1,"scheme":"light","palette":{"ink":"#12345678"},"tokens":{"text":"ink"}}"##, Some(&dark)).unwrap();
        assert_eq!(parsed.scheme, Scheme::Light);
        let text = parsed[Token::Text];
        assert_eq!(
            (text.red(), text.green(), text.blue(), text.alpha()),
            (0x12, 0x34, 0x56, 0x78)
        );
        for token in Token::ALL {
            if token != Token::Text {
                assert_eq!(parsed[token], dark[token]);
            }
        }
        let parsed = parse(
            r##"{"format":1,"scheme":"light","base":"light","tokens":{"text":"#123456"}}"##,
            None,
        )
        .unwrap();
        assert_eq!(
            parsed[Token::Text],
            Color::from_argb_u8(255, 0x12, 0x34, 0x56)
        );
        assert_eq!(
            parsed[Token::FieldBackground],
            builtin(Scheme::Light).unwrap()[Token::FieldBackground]
        );
    }
    #[test]
    fn malformed_palette_objects_and_color_values_are_rejected() {
        let base = builtin(Scheme::Dark).unwrap();
        for value in [
            "#12345",
            "#1234567",
            "#123456789",
            "#12345g",
            "#é1234",
            "#+12345",
        ] {
            let input = serde_json::json!({"format":1,"scheme":"dark","tokens":{"text":value}});
            assert!(
                matches!(
                    parse(&input.to_string(), Some(&base)),
                    Err(ThemeError::BadColor { .. })
                ),
                "{value}"
            );
        }
        for input in [
            r#"{"format":1,"scheme":"dark","palette":[]}"#,
            r#"{"format":1,"scheme":"dark","palette":{"ink":4}}"#,
            r#"{"format":1,"scheme":"dark","tokens":[]}"#,
            r#"{"format":1,"scheme":"dark","tokens":{"text":4}}"#,
        ] {
            assert!(matches!(
                parse(input, Some(&base)),
                Err(ThemeError::BadColor { .. })
            ));
        }
    }
    fn read_colors(global: &DesktopTheme<'_>) -> [Color; N] {
        [
            global.get_window_background(),
            global.get_workspace_background(),
            global.get_pasteboard_background(),
            global.get_panel_background(),
            global.get_raised_background(),
            global.get_tab_background(),
            global.get_border(),
            global.get_strong_border(),
            global.get_accent(),
            global.get_selected_tool(),
            global.get_text(),
            global.get_muted_text(),
            global.get_subtle_text(),
            global.get_status_text(),
            global.get_error_text(),
            global.get_panel_border(),
            global.get_field_background(),
            global.get_field_hover(),
            global.get_field_disabled(),
            global.get_field_border(),
            global.get_focus_ring(),
            global.get_highlight_background(),
            global.get_hover_background(),
            global.get_preview_ink(),
            global.get_checker_light(),
            global.get_checker_dark(),
            global.get_scrim(),
            global.get_handle_stroke(),
            global.get_handle_fill(),
            global.get_handle_pivot_fill(),
        ]
    }
    #[test]
    fn applying_unique_tokens_reaches_every_global_on_every_window() {
        i_slint_backend_testing::init_no_event_loop();
        let (ui, history, colors, shapes) = (
            AppWindow::new().unwrap(),
            HistoryWindow::new().unwrap(),
            ColorPickerWindow::new().unwrap(),
            ShapePropertiesWindow::new().unwrap(),
        );
        let theme = Theme {
            scheme: Scheme::Light,
            colors: std::array::from_fn(|i| Color::from_argb_u8(255, i as u8 + 1, 31, 91)),
        };
        apply(&theme, &ui, &history, &colors, &shapes);
        for global in [
            ui.global::<DesktopTheme>(),
            history.global::<DesktopTheme>(),
            colors.global::<DesktopTheme>(),
            shapes.global::<DesktopTheme>(),
        ] {
            assert_eq!(read_colors(&global), theme.colors);
        }
    }
    #[test]
    fn system_reads_the_original_widget_palette_and_unknown_falls_back_to_dark() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = AppWindow::new().unwrap();
        assert_eq!(
            ui.global::<Palette>().get_color_scheme(),
            ColorScheme::Unknown
        );
        assert_eq!(system_scheme(&ui), Scheme::Dark);
        ui.global::<Palette>().set_color_scheme(ColorScheme::Light);
        assert_eq!(system_scheme(&ui), Scheme::Light);
        ui.global::<Palette>().set_color_scheme(ColorScheme::Dark);
        assert_eq!(system_scheme(&ui), Scheme::Dark);
    }
    #[test]
    fn desktop_startup_and_light_application_theme_all_four_window_roots() {
        i_slint_backend_testing::init_no_event_loop();
        let app = crate::create_app_with_all_panels().unwrap();
        let dark = builtin(Scheme::Dark).unwrap();
        for global in [
            app.ui.global::<DesktopTheme>(),
            app.history.global::<DesktopTheme>(),
            app.colors.global::<DesktopTheme>(),
            app.shapes.global::<DesktopTheme>(),
        ] {
            assert_eq!(global.get_text(), dark[Token::Text]);
        }
        let light = builtin(Scheme::Light).unwrap();
        apply(&light, &app.ui, &app.history, &app.colors, &app.shapes);
        for global in [
            app.ui.global::<DesktopTheme>(),
            app.history.global::<DesktopTheme>(),
            app.colors.global::<DesktopTheme>(),
            app.shapes.global::<DesktopTheme>(),
        ] {
            assert_eq!(global.get_text(), light[Token::Text]);
        }
        for palette in [
            app.ui.global::<Palette>(),
            app.history.global::<Palette>(),
            app.colors.global::<Palette>(),
            app.shapes.global::<Palette>(),
        ] {
            assert_eq!(palette.get_color_scheme(), ColorScheme::Light);
        }
        assert_eq!(app.controller.borrow().document().undo_len(), 0);
    }
}
