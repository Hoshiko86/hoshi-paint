//! Canvas Color Picker (`Tool::Eyedropper`) sampling contract; see
//! `docs/V0_0_0.md`, "Color Picker and Screen Picker".

use hoshi_controller::{Color, ColorSlot, Controller, Tool};
use hoshi_core::{Document, Shape, ShapeKind, ShapeStyle};

const WHITE: Color = Color::rgba(255, 255, 255, 255);
const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);
const TRANSLUCENT: Color = Color::rgba(10, 200, 30, 90);
const PRIMARY: Color = Color::rgba(1, 2, 3, 255);
const SECONDARY: Color = Color::rgba(4, 5, 6, 255);
/// BLUE at layer opacity 128 source-over opaque WHITE, rounded to nearest.
const HALF_BLUE_ON_WHITE: Color = Color::rgba(127, 127, 255, 255);

fn fill(document: &mut Document, layer: usize, x: i32, width: u32, color: Color) {
    document.select_layer(layer).unwrap();
    let shape = Shape::new(
        ShapeKind::Rectangle,
        x,
        0,
        width,
        4,
        ShapeStyle::Fill(color),
    );
    document.begin_shape(shape).unwrap();
    assert!(document.commit_shape().unwrap());
}

/// 8x4 canvas, columns (bottom to top):
/// 0..2 white background under a hidden red layer;
/// 2..4 white background under a blue layer at 50% opacity;
/// 4..6 a translucent layer over transparency; 6..8 fully transparent.
fn layered_document() -> Document {
    let mut document = Document::new(8, 4).unwrap();
    fill(&mut document, 0, 0, 4, WHITE);
    let hidden = document.add_layer("Hidden").unwrap();
    fill(&mut document, hidden, 0, 2, RED);
    document.set_layer_properties(hidden, false, 255).unwrap();
    let half = document.add_layer("Half").unwrap();
    fill(&mut document, half, 2, 2, BLUE);
    document.set_layer_properties(half, true, 128).unwrap();
    let alpha = document.add_layer("Alpha").unwrap();
    fill(&mut document, alpha, 4, 2, TRANSLUCENT);
    document
}

fn picker(document: Document) -> Controller {
    let mut c = Controller::new(8, 4).unwrap();
    // Set slots before installing the document so an active preview keeps its style.
    c.set_primary_color(PRIMARY).unwrap();
    c.set_secondary_color(SECONDARY).unwrap();
    c.replace_document(document);
    c.set_tool(Tool::Eyedropper).unwrap();
    c
}

/// Click at a document coordinate, converted through the current viewport.
fn click(c: &mut Controller, x: f64, y: f64) {
    let p = c.viewport().to_logical(x, y);
    c.pointer_down(p.0, p.1).unwrap();
    c.pointer_up().unwrap();
}

#[test]
fn samples_visible_composite_with_layer_opacity_and_alpha() {
    let mut c = picker(layered_document());
    let history = c.document().undo_len();
    c.zoom_at(0., 0., 3.).unwrap();
    c.pan(11., -7.).unwrap();

    for (x, y, expected) in [
        (1.5, 1.5, WHITE),               // hidden red layer excluded
        (3.99, 0.2, HALF_BLUE_ON_WHITE), // floor(3.99) = 3: 50% layer blended
        (4.0, 3.99, TRANSLUCENT),        // straight alpha preserved
        (7.5, 2.5, Color::TRANSPARENT),  // empty pixel samples (0,0,0,0)
    ] {
        click(&mut c, x, y);
        assert_eq!(c.primary_color(), expected, "at ({x}, {y})");
        assert_eq!(
            c.primary_color(),
            c.document()
                .composite_pixel(x.floor() as u32, y.floor() as u32, true)
        );
        assert_eq!(c.secondary_color(), SECONDARY);
    }
    assert_eq!(Color::TRANSPARENT, Color::rgba(0, 0, 0, 0));
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn secondary_slot_receives_the_sample() {
    let mut c = picker(layered_document());
    let history = c.document().undo_len();
    c.set_color_slot(ColorSlot::Secondary);
    click(&mut c, 2.5, 1.);
    assert_eq!(c.secondary_color(), HALF_BLUE_ON_WHITE);
    assert_eq!(c.primary_color(), PRIMARY);
    assert_eq!(c.color_slot(), ColorSlot::Secondary);
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn outside_canvas_is_ignored_for_clicks_and_drags() {
    let mut c = picker(layered_document());
    let history = c.document().undo_len();
    for (x, y) in [(-0.01, 1.), (8., 1.), (1., -0.5), (1., 4.), (100., 100.)] {
        click(&mut c, x, y);
        assert_eq!(
            (c.primary_color(), c.secondary_color()),
            (PRIMARY, SECONDARY)
        );
    }
    // A drag that leaves the canvas keeps the last in-canvas sample.
    let inside = c.viewport().to_logical(4.5, 1.);
    c.pointer_down(inside.0, inside.1).unwrap();
    assert_eq!(c.primary_color(), TRANSLUCENT);
    let outside = c.viewport().to_logical(9., 1.);
    c.pointer_move(outside.0, outside.1).unwrap();
    assert_eq!(c.primary_color(), TRANSLUCENT);
    c.pointer_up().unwrap();
    assert_eq!(c.secondary_color(), SECONDARY);
    assert_eq!(c.document().undo_len(), history);
}

#[test]
fn samples_uncommitted_shape_preview_without_history() {
    let mut document = layered_document();
    let history = document.undo_len();
    let preview = Color::rgba(200, 100, 50, 255);
    let shape = Shape::new(ShapeKind::Rectangle, 6, 0, 2, 4, ShapeStyle::Fill(preview));
    document.begin_shape(shape).unwrap();
    let mut c = picker(document);
    c.set_color_slot(ColorSlot::Secondary);
    click(&mut c, 6.5, 2.);
    assert_eq!(c.secondary_color(), preview);
    assert!(c.document().active_shape().is_some());
    assert_eq!(c.document().undo_len(), history);
}
