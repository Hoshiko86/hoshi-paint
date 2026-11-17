use hoshi_controller::{
    BrushSettings, BrushWidth, Color, ColorSlot, ColorState, Controller, FillPattern, LineStyle,
    ShapeMode, ShapeSettings, ShapeStyle, ToolSettings,
};

fn custom_settings() -> ToolSettings {
    ToolSettings {
        brush: BrushSettings {
            width: BrushWidth::new(4.5).unwrap(),
        },
        fill: FillPattern::Cross,
        antialias: false,
        shape: ShapeSettings {
            mode: ShapeMode::Both,
            line_style: LineStyle::Dash,
        },
    }
}

#[test]
fn defaults_and_round_trip_match_legacy_queries() {
    let mut c = Controller::new(64, 64).unwrap();
    assert_eq!(c.settings(), ToolSettings::default());
    assert_eq!(c.colors(), ColorState::default());
    assert_eq!(c.stroke_width_pixels(), 2.);
    assert_eq!(c.stroke_width(), 2);
    assert_eq!(c.brush_width(), 2.);
    assert!(c.antialiasing());
    assert_eq!(c.shape_mode(), ShapeMode::Fill);
    assert_eq!(c.line_style(), LineStyle::Solid);
    assert_eq!(c.fill_pattern(), FillPattern::Solid);
    assert_eq!(c.primary_color(), c.colors().primary);
    assert_eq!(c.secondary_color(), c.colors().secondary);
    assert_eq!(c.selected_color(), c.colors().selected());
    assert_eq!(c.color_slot(), ColorSlot::Primary);
    c.set_settings(custom_settings()).unwrap();
    assert_eq!(c.settings(), custom_settings());
    assert_eq!(c.stroke_width(), 5);
    assert_eq!(c.stroke_width_pixels(), 4.5);
    assert_eq!(c.brush_width(), 4.5);
}

#[test]
fn settings_and_colors_restyle_preview_without_history_or_pixel_edits() {
    let mut c = Controller::new(64, 64).unwrap();
    c.pointer_down(8., 8.).unwrap();
    c.pointer_move(48., 40.).unwrap();
    c.pointer_up().unwrap();
    let before = c.document().active_shape().unwrap();
    let pixels = c.document().snapshot();
    c.set_settings(custom_settings()).unwrap();
    let colors = ColorState {
        primary: Color::rgba(10, 20, 30, 255),
        secondary: Color::rgba(40, 50, 60, 128),
        slot: ColorSlot::Secondary,
    };
    c.set_colors(colors).unwrap();
    assert_eq!(c.colors(), colors);
    let mut expected = before;
    expected.style = ShapeStyle::Both {
        fill: colors.secondary,
        outline: colors.primary,
        width: 4.5,
    };
    expected.rendering.antialias = false;
    expected.rendering.line_style = LineStyle::Dash;
    expected.rendering.fill_pattern = FillPattern::Cross;
    expected.rendering.pattern_color = colors.primary;
    assert_eq!(c.document().active_shape(), Some(expected));
    assert_eq!(c.document().undo_len(), 0);
    assert_eq!(c.history_position(), 0);
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(
                c.document().composite_pixel(x, y, false),
                pixels.composite_pixel(x, y, None)
            );
        }
    }
    let preview = c.composite();
    c.commit().unwrap();
    assert_eq!(c.document().undo_len(), 1);
    assert_eq!(c.composite(), preview);
    c.undo().unwrap();
    assert!(c.composite().iter().all(|color| color.a == 0));
    c.redo().unwrap();
    assert_eq!(c.composite(), preview);
}

#[test]
fn legacy_toolbar_setters_change_only_their_field() {
    let mut c = Controller::new(64, 64).unwrap();
    c.set_settings(custom_settings()).unwrap();
    let colors = c.colors();
    let mut expected = c.settings();
    c.set_antialiasing(true).unwrap();
    expected.antialias = true;
    assert_eq!(c.settings(), expected);
    c.set_line_style(LineStyle::Dot).unwrap();
    expected.shape.line_style = LineStyle::Dot;
    assert_eq!(c.settings(), expected);
    c.set_fill_pattern(FillPattern::Vertical).unwrap();
    expected.fill = FillPattern::Vertical;
    assert_eq!(c.settings(), expected);
    c.set_shape_mode(ShapeMode::Outline).unwrap();
    expected.shape.mode = ShapeMode::Outline;
    assert_eq!(c.settings(), expected);
    c.set_stroke_width_pixels(3.25).unwrap();
    expected.brush.width = BrushWidth::new(3.25).unwrap();
    assert_eq!(c.settings(), expected);
    c.set_stroke_width(6).unwrap();
    expected.brush.width = BrushWidth::new(6.).unwrap();
    assert_eq!(c.settings(), expected);
    c.set_brush_width(7.).unwrap();
    expected.brush.width = BrushWidth::new(7.).unwrap();
    assert_eq!(c.settings(), expected);
    for invalid in [0., f64::NAN, f64::INFINITY, 65_537.] {
        assert!(c.set_stroke_width_pixels(invalid).is_err());
        assert_eq!(c.settings(), expected);
        assert!(c.set_brush_width(invalid).is_err());
        assert_eq!(c.settings(), expected);
    }
    assert_eq!(c.colors(), colors);
}

#[test]
fn legacy_color_setters_change_only_their_field_and_keep_settings() {
    let mut c = Controller::new(64, 64).unwrap();
    c.set_settings(custom_settings()).unwrap();
    let settings = c.settings();
    let mut expected = c.colors();
    let a = Color::rgba(1, 2, 3, 4);
    let b = Color::rgba(5, 6, 7, 8);
    c.set_primary_color(a).unwrap();
    expected.primary = a;
    assert_eq!(c.colors(), expected);
    c.set_secondary_color(b).unwrap();
    expected.secondary = b;
    assert_eq!(c.colors(), expected);
    c.set_color_slot(ColorSlot::Secondary);
    expected.slot = ColorSlot::Secondary;
    assert_eq!(c.colors(), expected);
    c.set_selected_color(a).unwrap();
    expected.secondary = a;
    assert_eq!(c.colors(), expected);
    c.set_secondary_color(b).unwrap();
    expected.secondary = b;
    c.swap_colors().unwrap();
    expected = expected.swapped();
    assert_eq!(c.colors(), expected);
    c.reset_colors().unwrap();
    expected = ColorState {
        slot: ColorSlot::Secondary,
        ..ColorState::default()
    };
    assert_eq!(c.colors(), expected);
    c.set_color_slot(ColorSlot::Primary);
    expected.slot = ColorSlot::Primary;
    c.set_selected_color(b).unwrap();
    expected.primary = b;
    assert_eq!(c.colors(), expected);
    assert_eq!(c.settings(), settings);
}
