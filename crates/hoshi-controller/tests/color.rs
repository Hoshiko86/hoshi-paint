use hoshi_controller::{Color, Controller, Tool, color::*};

#[test]
fn hsv_primaries_grays_and_roundtrip() {
    for (hue, rgb) in [
        (0., (255, 0, 0)),
        (60., (255, 255, 0)),
        (120., (0, 255, 0)),
        (180., (0, 255, 255)),
        (240., (0, 0, 255)),
        (300., (255, 0, 255)),
        (360., (255, 0, 0)),
    ] {
        let color = color_from_hsv(hue, 100., 100., 37).unwrap();
        assert_eq!(color, Color::rgba(rgb.0, rgb.1, rgb.2, 37));
        assert_eq!(color_to_hsv(color).hue, hue % 360.);
    }
    for v in [0, 1, 127, 255] {
        let color = Color::rgba(v, v, v, 93);
        let hsv = color_to_hsv(color);
        assert_eq!((hsv.hue, hsv.saturation), (0., 0.));
        assert_eq!(
            color_from_hsv(hsv.hue, hsv.saturation, hsv.value, color.a).unwrap(),
            color
        );
    }
    for r in [0, 31, 128, 255] {
        for g in [0, 57, 191, 255] {
            for b in [0, 99, 234, 255] {
                let color = Color::rgba(r, g, b, 42);
                let hsv = color_to_hsv(color);
                assert_eq!(
                    color_from_hsv(hsv.hue, hsv.saturation, hsv.value, color.a).unwrap(),
                    color
                );
            }
        }
    }
}

#[test]
fn hsv_rejects_invalid_ranges_and_nonfinite_values() {
    for h in [-0.01, 360.01, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(color_from_hsv(h, 50., 50., 255).is_err());
    }
    for v in [-0.01, 100.01, f64::NAN, f64::INFINITY] {
        assert!(color_from_hsv(0., v, 50., 255).is_err());
        assert!(color_from_hsv(0., 50., v, 255).is_err());
    }
    assert!(color_from_hsv(359.999, 100., 100., 0).is_ok());
}

#[test]
fn strict_hex_forms_and_alpha() {
    for (text, color) in [
        ("#aBc", Color::rgba(170, 187, 204, 255)),
        ("#1234", Color::rgba(17, 34, 51, 68)),
        ("#12abEF", Color::rgba(18, 171, 239, 255)),
        ("#12345678", Color::rgba(18, 52, 86, 120)),
    ] {
        assert_eq!(parse_hex(text), Ok(color));
        assert_eq!(parse_hex(&format_hex(color)), Ok(color));
    }
    assert_eq!(format_hex(Color::rgba(1, 2, 3, 4)), "#01020304");
    for bad in [
        "",
        "abc",
        " #abc",
        "#abc ",
        "#12",
        "#12345",
        "#1234567",
        "#123456789",
        "#ggg",
        "#１２３",
        "##abc",
        "#12\n",
    ] {
        assert!(parse_hex(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn eyedropper_samples_document_alpha_without_history() {
    let mut c = Controller::new(16, 16).unwrap();
    let color = Color::rgba(27, 99, 213, 87);
    c.set_primary_color(color).unwrap();
    c.pointer_down(2., 3.).unwrap();
    c.pointer_move(7., 9.).unwrap();
    c.pointer_up().unwrap();
    c.commit().unwrap();
    c.set_tool(Tool::Eyedropper).unwrap();
    assert!(c.document().active_shape().is_none());
    assert_eq!(c.document().undo_len(), 1);
    let merged = c.composite()[4 * 16 + 3];
    assert_eq!(merged, color);
    c.set_primary_color(Color::rgba(255, 0, 0, 255)).unwrap();
    c.pan(21., -13.).unwrap();
    c.zoom_at(0., 0., 2.5).unwrap();
    let p = c.viewport().to_logical(3.8, 4.2);
    c.pointer_down(p.0, p.1).unwrap();
    assert_eq!(c.primary_color(), color);
    let outside = c.viewport().to_logical(16., 4.);
    c.pointer_move(outside.0, outside.1).unwrap();
    assert_eq!(c.primary_color(), color);
    let empty = c.viewport().to_logical(0.1, 0.1);
    c.pointer_move(empty.0, empty.1).unwrap();
    assert_eq!(c.primary_color().a, 0);
    c.pointer_up().unwrap();
    c.set_primary_color(color).unwrap();
    c.pointer_move(empty.0, empty.1).unwrap();
    assert_eq!(c.primary_color(), color);
    let outside = c.viewport().to_logical(-0.01, 0.);
    c.pointer_down(outside.0, outside.1).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(c.primary_color(), color);
    assert_eq!(c.document().undo_len(), 1);
    assert!(c.pointer_down(f64::NAN, 0.).is_err());
}

#[test]
fn eyedropper_recolors_editable_shape_from_underlying_pixels_then_commit_undo() {
    let mut c = Controller::new(128, 128).unwrap();
    let source = Color::rgba(27, 99, 213, 87);
    c.set_primary_color(source).unwrap();
    c.pointer_down(10., 10.).unwrap();
    c.pointer_move(100., 100.).unwrap();
    c.pointer_up().unwrap();
    c.commit().unwrap();
    let committed = c.composite();
    c.set_primary_color(Color::rgba(255, 0, 0, 255)).unwrap();
    c.pointer_down(30., 30.).unwrap();
    c.pointer_move(80., 80.).unwrap();
    c.pointer_up().unwrap();
    let original = c.document().active_shape().unwrap();
    c.set_tool(Tool::Eyedropper).unwrap();
    assert_eq!(c.document().active_shape(), Some(original));
    c.zoom_at(0., 0., 2.5).unwrap();
    c.pan(17., -9.).unwrap();
    let sample = c.viewport().to_logical(20., 20.);
    c.pointer_down(sample.0, sample.1).unwrap();
    c.pointer_up().unwrap();
    assert_eq!(c.primary_color(), source);
    let recolored = c.document().active_shape().unwrap();
    assert_eq!(recolored.style, hoshi_core::ShapeStyle::Fill(source));
    assert_eq!(
        (recolored.x, recolored.y, recolored.width, recolored.height),
        (30, 30, 50, 50)
    );
    assert_eq!(c.document().undo_len(), 1);
    c.set_tool(Tool::Rectangle).unwrap();
    assert_eq!(c.document().active_shape(), Some(recolored));
    let preview = c.composite();
    c.commit().unwrap();
    assert_eq!(c.document().undo_len(), 2);
    assert_eq!(c.composite(), preview);
    c.undo().unwrap();
    assert_eq!(c.composite(), committed);
}

#[test]
fn eyedropper_freezes_visible_translucent_preview_for_each_drag() {
    let mut c = Controller::new(128, 128).unwrap();
    c.set_primary_color(Color::rgba(20, 60, 220, 140)).unwrap();
    c.pointer_down(10., 10.).unwrap();
    c.pointer_move(100., 100.).unwrap();
    c.pointer_up().unwrap();
    c.commit().unwrap();
    c.set_primary_color(Color::rgba(230, 40, 10, 100)).unwrap();
    c.pointer_down(30., 30.).unwrap();
    c.pointer_move(80., 80.).unwrap();
    c.pointer_up().unwrap();
    let visible = c.composite()[50 * 128 + 50];
    c.set_tool(Tool::Eyedropper).unwrap();
    c.pointer_down(50., 50.).unwrap();
    assert_eq!(c.primary_color(), visible);
    assert_ne!(c.composite()[50 * 128 + 50], visible);
    for _ in 0..3 {
        c.pointer_move(50., 50.).unwrap();
        assert_eq!(c.primary_color(), visible);
    }
    c.pointer_up().unwrap();
    let updated = c.composite()[50 * 128 + 50];
    c.pointer_down(50., 50.).unwrap();
    assert_eq!(c.primary_color(), updated);
    c.pointer_up().unwrap();
    assert!(c.document().active_shape().is_some());
    assert_eq!(c.document().undo_len(), 1);
}
