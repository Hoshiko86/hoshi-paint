use hoshi_core::{Color, Document, Error, FillPattern, LineStyle, Shape, ShapeKind, ShapeStyle};
const RED: Color = Color::rgba(240, 30, 20, 255);
const BLUE: Color = Color::rgba(10, 40, 220, 255);
fn rectangle(style: ShapeStyle) -> Shape {
    Shape::new(ShapeKind::Rectangle, 8, 8, 32, 24, style)
}
fn render(shape: Shape) -> Vec<Color> {
    let mut doc = Document::new(48, 48).unwrap();
    doc.begin_shape(shape).unwrap();
    let preview = doc.composite(true);
    doc.commit_shape().unwrap();
    assert_eq!(
        doc.composite(false),
        preview,
        "committed pixels must match the preview"
    );
    doc.undo().unwrap();
    assert!(doc.composite(false).iter().all(|p| p.a == 0));
    doc.redo().unwrap();
    assert_eq!(doc.composite(false), preview);
    preview
}
#[test]
fn fractional_strokes_have_exact_coverage_and_no_dark_fringe() {
    let pixels = render(rectangle(ShapeStyle::Outline {
        color: RED,
        width: 1.25,
    }));
    assert_eq!(pixels[16 * 48 + 8], RED);
    assert_eq!(pixels[16 * 48 + 9], Color::rgba(240, 30, 20, 64));
    assert_eq!(pixels[16 * 48 + 10], Color::TRANSPARENT);
    let thin = render(rectangle(ShapeStyle::Outline {
        color: RED,
        width: 0.01,
    }));
    assert_eq!(thin[16 * 48 + 8], Color::rgba(240, 30, 20, 3));
}
#[test]
fn antialiasing_preserves_translucent_edge_hue_and_can_be_disabled() {
    for kind in [ShapeKind::Rectangle, ShapeKind::Ellipse] {
        let color = Color::rgba(230, 40, 90, 128);
        let mut shape = rectangle(ShapeStyle::Fill(color));
        shape.kind = kind;
        shape.rotation_degrees = 27;
        let smooth = render(shape);
        assert!(smooth.iter().any(|p| p.a > 0 && p.a < 128));
        assert!(
            smooth
                .iter()
                .filter(|p| p.a > 0)
                .all(|p| (p.r, p.g, p.b) == (230, 40, 90))
        );
        shape.rendering.antialias = false;
        let aliased = render(shape);
        assert!(aliased.iter().all(|p| p.a == 0 || *p == color));
        assert_ne!(aliased, smooth);
    }
}
#[test]
fn dash_patterns_have_gaps_and_gaps_stay_transparent_in_both_mode() {
    // Paint.NET 5.2 draws the outline inside the shape and paints no fill under
    // the outline band, so dash gaps are transparent in Fill & Outline (#73).
    for kind in [ShapeKind::Rectangle, ShapeKind::Ellipse] {
        for rotation in [0, 27] {
            for antialias in [true, false] {
                let shape_with = |style, line_style| {
                    let mut s = rectangle(style);
                    s.kind = kind;
                    s.rotation_degrees = rotation;
                    s.rendering.antialias = antialias;
                    s.rendering.line_style = line_style;
                    s
                };
                let outline = ShapeStyle::Outline {
                    color: RED,
                    width: 2.,
                };
                let both = ShapeStyle::Both {
                    fill: BLUE,
                    outline: RED,
                    width: 2.,
                };
                let alpha = |pixels: &[Color]| pixels.iter().map(|p| u64::from(p.a)).sum::<u64>();
                let solid_outline = render(shape_with(outline, LineStyle::Solid));
                let solid_both = render(shape_with(both, LineStyle::Solid));
                for style in [
                    LineStyle::Dash,
                    LineStyle::Dot,
                    LineStyle::DashDot,
                    LineStyle::DashDotDot,
                ] {
                    let case = format!("{kind:?} {rotation}° aa={antialias} {style:?}");
                    let dashed = render(shape_with(outline, style));
                    assert!(alpha(&dashed) < alpha(&solid_outline), "{case}");
                    assert!(dashed.iter().any(|p| p.a > 0), "{case}");
                    let dashed_both = render(shape_with(both, style));
                    assert!(dashed_both.contains(&BLUE), "{case}: fill inside the band");
                    assert!(
                        alpha(&dashed_both) < alpha(&solid_both),
                        "{case}: gaps add no fill"
                    );
                    assert!(
                        solid_both
                            .iter()
                            .zip(&dashed_both)
                            .any(|(solid, dashed)| solid.a == 255 && dashed.a == 0),
                        "{case}: a gap pixel is transparent"
                    );
                    // Wherever the dashed outline paints nothing, Both mode
                    // shows only fill or nothing: never outline color.
                    assert!(
                        dashed
                            .iter()
                            .zip(&dashed_both)
                            .filter(|(o, _)| o.a == 0)
                            .all(|(_, b)| b.a == 0 || (b.r, b.g, b.b) == (BLUE.r, BLUE.g, BLUE.b)),
                        "{case}"
                    );
                    if kind == ShapeKind::Rectangle && rotation == 0 {
                        // Top band row: identical to Outline mode, gaps included.
                        assert!(
                            (8..40).all(|x| dashed_both[8 * 48 + x] == dashed[8 * 48 + x]),
                            "{case}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn patterns_use_both_colors_even_when_every_canvas_corner_has_the_same_color() {
    for pattern in [
        FillPattern::Horizontal,
        FillPattern::Vertical,
        FillPattern::ForwardDiagonal,
        FillPattern::BackwardDiagonal,
        FillPattern::Cross,
        FillPattern::DiagonalCross,
        FillPattern::Checkerboard,
    ] {
        // Uniform-corner optimizations must not erase the pattern's interior.
        let mut doc = Document::new(9, 9).unwrap();
        let mut shape = Shape::new(ShapeKind::Rectangle, 0, 0, 9, 9, ShapeStyle::Fill(RED));
        shape.rendering.fill_pattern = pattern;
        shape.rendering.pattern_color = BLUE;
        doc.begin_shape(shape).unwrap();
        let before = doc.composite(true);
        assert!(before.contains(&RED) && before.contains(&BLUE));
        doc.commit_shape().unwrap();
        assert_eq!(doc.composite(false), before);
    }
}
#[test]
fn transparent_pattern_color_is_detected_when_opaque_corners_hide_it() {
    let mut doc = Document::new(9, 9).unwrap();
    let mut shape = Shape::new(ShapeKind::Rectangle, 0, 0, 9, 9, ShapeStyle::Fill(RED));
    shape.rendering.fill_pattern = FillPattern::Cross;
    shape.rendering.pattern_color = Color::TRANSPARENT;
    doc.begin_shape(shape).unwrap();
    assert!(doc.has_transparency_fast());
    let before = doc.composite(true);
    doc.commit_shape().unwrap();
    assert_eq!(doc.composite(false), before);
    assert!(doc.has_transparency_fast());
}
#[test]
fn invalid_widths_fail_without_mutating_the_active_shape() {
    let mut doc = Document::new(48, 48).unwrap();
    let good = rectangle(ShapeStyle::Fill(RED));
    doc.begin_shape(good).unwrap();
    for width in [
        0.,
        -1.,
        0.001,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        65537.,
    ] {
        let invalid = rectangle(ShapeStyle::Outline { color: RED, width });
        assert_eq!(doc.adjust_shape(invalid), Err(Error::InvalidShape));
        assert_eq!(doc.active_shape(), Some(good));
    }
}
#[test]
fn raster_coverage_is_conserved_when_a_solid_rectangle_rotates() {
    for angle in [1, 17, 45, 89, 90, 137] {
        let mut shape = rectangle(ShapeStyle::Fill(RED));
        shape.rotation_degrees = angle;
        let pixels = render(shape);
        let area = pixels.iter().map(|p| f64::from(p.a) / 255.).sum::<f64>();
        assert!((area - 32. * 24.).abs() < 1., "angle {angle}: area {area}");
    }
}

#[test]
fn unused_line_settings_do_not_disable_constant_size_huge_fills() {
    let size = hoshi_core::MAX_DIMENSION;
    let mut doc = Document::new(size, size).unwrap();
    let mut shape = Shape::new(
        ShapeKind::Rectangle,
        0,
        0,
        size,
        size,
        ShapeStyle::Fill(RED),
    );
    shape.rendering.line_style = LineStyle::DashDotDot;
    doc.begin_shape(shape).unwrap();
    doc.commit_shape().unwrap();
    assert_eq!(doc.layers()[0].pixels().allocated_bytes(), 0);
    assert_eq!(doc.composite_pixel(size / 2, size / 2, false), RED);
}

#[test]
fn unused_style_settings_never_change_pixel_coverage() {
    let mut outline = rectangle(ShapeStyle::Outline {
        color: RED,
        width: 0.01,
    });
    let expected = render(outline);
    outline.rendering.fill_pattern = FillPattern::Cross;
    assert_eq!(
        render(outline),
        expected,
        "unused hatches must not erase thin outlines"
    );
    let mut fill = rectangle(ShapeStyle::Fill(RED));
    fill.rotation_degrees = 27;
    let expected = render(fill);
    fill.rendering.line_style = LineStyle::Dash;
    assert_eq!(
        render(fill),
        expected,
        "unused line styles must not change fill antialiasing"
    );
}
#[test]
fn equal_pattern_colors_keep_huge_fills_uniform() {
    let size = hoshi_core::MAX_DIMENSION;
    let mut doc = Document::new(size, size).unwrap();
    let mut shape = Shape::new(
        ShapeKind::Rectangle,
        0,
        0,
        size,
        size,
        ShapeStyle::Fill(RED),
    );
    shape.rendering.fill_pattern = FillPattern::Cross;
    shape.rendering.pattern_color = RED;
    doc.begin_shape(shape).unwrap();
    doc.commit_shape().unwrap();
    assert_eq!(doc.layers()[0].pixels().allocated_bytes(), 0);
}

#[test]
fn matching_fill_and_outline_colors_do_not_change_coverage_with_dash_style() {
    let mut shape = rectangle(ShapeStyle::Both {
        fill: RED,
        outline: RED,
        width: 2.25,
    });
    shape.rotation_degrees = 27;
    let expected = render(shape);
    shape.rendering.line_style = LineStyle::DashDot;
    assert_eq!(render(shape), expected);
}

#[test]
fn thick_ellipse_dash_count_uses_the_stroke_centerline() {
    let width = 8.;
    let (rx, ry) = (32. - width / 2., 20. - width / 2.);
    let mut shape = Shape::new(
        ShapeKind::Ellipse,
        8,
        8,
        64,
        40,
        ShapeStyle::Outline { color: RED, width },
    );
    shape.rendering.line_style = LineStyle::Dash;
    shape.rendering.antialias = false;
    let mut doc = Document::new(80, 56).unwrap();
    doc.begin_shape(shape).unwrap();
    let pixels = doc.composite(true);
    doc.commit_shape().unwrap();
    assert_eq!(doc.composite(false), pixels);

    let steps = 4096;
    let point = |i| {
        let angle = i as f64 * std::f64::consts::TAU / steps as f64;
        (40. + rx * angle.sin(), 28. - ry * angle.cos())
    };
    let painted = |i| {
        let (x, y) = point(i);
        pixels[y.floor() as usize * 80 + x.floor() as usize].a > 0
    };
    let perimeter = (0..steps)
        .map(|i| {
            let (x, y) = point(i);
            let (next_x, next_y) = point(i + 1);
            (next_x - x).hypot(next_y - y)
        })
        .sum::<f64>();
    // Dash repeats 3w on / 1w off. The last partial dash joins the first
    // across the closed path's seam, so only complete cycles start new runs.
    let expected = (perimeter / (4. * width)).floor() as usize;
    let runs = (0..steps)
        .filter(|&i| painted(i) && !painted((i + steps - 1) % steps))
        .count();
    assert_eq!(runs, expected, "centerline perimeter {perimeter}");
}

#[test]
fn rectangle_and_solid_ellipse_rendering_remain_byte_identical() {
    let cases = [
        (ShapeKind::Rectangle, LineStyle::Solid),
        (ShapeKind::Rectangle, LineStyle::Dash),
        (ShapeKind::Rectangle, LineStyle::Dot),
        (ShapeKind::Rectangle, LineStyle::DashDot),
        (ShapeKind::Rectangle, LineStyle::DashDotDot),
        (ShapeKind::Ellipse, LineStyle::Solid),
    ];
    let actual: Vec<_> = cases
        .into_iter()
        .map(|(kind, line_style)| {
            let mut hash = 0xcbf29ce484222325_u64;
            for width in [2.75, 8.] {
                for style in [
                    ShapeStyle::Fill(BLUE),
                    ShapeStyle::Outline { color: RED, width },
                    ShapeStyle::Both {
                        fill: BLUE,
                        outline: RED,
                        width,
                    },
                ] {
                    for rotation in [0, 27] {
                        for antialias in [false, true] {
                            let mut shape = rectangle(style);
                            shape.kind = kind;
                            shape.rendering.line_style = line_style;
                            shape.rotation_degrees = rotation;
                            shape.rendering.antialias = antialias;
                            for pixel in render(shape) {
                                for byte in [pixel.r, pixel.g, pixel.b, pixel.a] {
                                    hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
                                }
                            }
                        }
                    }
                }
            }
            hash
        })
        .collect();
    // Fingerprints captured from main before the ellipse dash-centerline fix.
    assert_eq!(
        actual,
        [
            9429211239380135253,
            13972128942793766380,
            9503033119333196172,
            18408558443046301532,
            5384846963200107273,
            1307527761561294205,
        ]
    );
}

#[test]
fn ellipse_dash_angle_remains_finite_when_the_centerline_collapses() {
    for width in [41., 65., 80.] {
        let mut shape = Shape::new(
            ShapeKind::Ellipse,
            8,
            8,
            65,
            41,
            ShapeStyle::Outline { color: RED, width },
        );
        shape.rendering.line_style = LineStyle::Dash;
        shape.rendering.antialias = false;
        let mut doc = Document::new(80, 64).unwrap();
        doc.begin_shape(shape).unwrap();
        let dashed = doc.composite(true);
        // One or both centerline radii are zero, and the entire degenerate
        // perimeter fits in the first 3w painted interval. No gap is expected,
        // including the pixel exactly at the ellipse center (40.5, 28.5).
        assert_eq!(doc.composite_pixel(40, 28, true), RED);
        shape.rendering.line_style = LineStyle::Solid;
        doc.adjust_shape(shape).unwrap();
        assert_eq!(doc.composite(true), dashed, "width {width}");
    }
}
