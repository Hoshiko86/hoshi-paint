//! Document brush/eraser stroke lifecycle (DRAW-1.2, #43): isolated preview,
//! one history entry per committed stroke, and edits blocked while stroking.

use hoshi_core::geometry::Point;
use hoshi_core::{
    Anchor, BrushTip, Color, Document, Error, Shape, ShapeKind, ShapeStyle, StrokeBlend,
    StrokeStyle,
};

const RED: Color = Color::rgba(255, 0, 0, 255);
const WHITE: Color = Color::rgba(255, 255, 255, 255);

fn brush(color: Color) -> StrokeStyle {
    StrokeStyle {
        tip: BrushTip::new(3.0, false).unwrap(),
        blend: StrokeBlend::Paint(color),
    }
}

fn eraser() -> StrokeStyle {
    StrokeStyle {
        tip: BrushTip::new(3.0, false).unwrap(),
        blend: StrokeBlend::Erase,
    }
}

fn draw_line(doc: &mut Document, style: StrokeStyle) {
    doc.begin_stroke(style, Point::new(2.5, 8.5)).unwrap();
    for x in [5.5, 9.5, 13.5] {
        doc.extend_stroke(Point::new(x, 8.5)).unwrap();
    }
}

fn last_label(doc: &Document) -> &'static str {
    doc.history_step(doc.history_position()).unwrap().label
}

#[test]
fn one_brush_stroke_is_one_history_entry_and_undo_redo_restore_exactly() {
    let mut doc = Document::new(16, 16).unwrap();
    let blank = doc.composite(false);
    draw_line(&mut doc, brush(RED));
    assert!(doc.commit_stroke().unwrap());
    assert!(!doc.has_active_stroke());
    assert_eq!(doc.undo_len(), 1);
    assert_eq!(last_label(&doc), "Brush");
    let painted = doc.composite(false);
    assert_eq!(painted[8 * 16 + 8], RED);
    assert!(doc.undo().unwrap());
    assert_eq!(doc.composite(false), blank);
    assert!(doc.redo().unwrap());
    assert_eq!(doc.composite(false), painted);
}

#[test]
fn preview_is_isolated_until_commit_then_matches_exactly() {
    let mut doc = Document::new(16, 16).unwrap();
    let layer_before = doc.layers()[0].pixels().clone();
    draw_line(&mut doc, brush(RED));
    assert!(doc.has_active_stroke());
    assert_eq!(*doc.layers()[0].pixels(), layer_before);
    let preview = doc.composite(true);
    assert_ne!(preview, doc.composite(false));
    assert_eq!(doc.undo_len(), 0);
    assert!(doc.commit_stroke().unwrap());
    assert_eq!(doc.composite(false), preview);
}

#[test]
fn eraser_clears_alpha_under_the_path() {
    let mut doc = Document::new_with_background(16, 16, 96.0, WHITE).unwrap();
    draw_line(&mut doc, eraser());
    assert!(doc.commit_stroke().unwrap());
    assert_eq!(last_label(&doc), "Eraser");
    let pixels = doc.composite(false);
    assert_eq!(pixels[8 * 16 + 8], Color::TRANSPARENT);
    assert_eq!(pixels[0], WHITE);
}

#[test]
fn no_op_and_cancelled_strokes_add_no_history() {
    let mut doc = Document::new(16, 16).unwrap();
    // Erasing a transparent layer changes nothing.
    draw_line(&mut doc, eraser());
    assert!(!doc.commit_stroke().unwrap());
    // A stroke entirely off canvas covers nothing.
    doc.begin_stroke(brush(RED), Point::new(-20.0, -20.0))
        .unwrap();
    doc.extend_stroke(Point::new(-10.0, -20.0)).unwrap();
    assert!(!doc.commit_stroke().unwrap());
    // Cancel discards the preview.
    draw_line(&mut doc, brush(RED));
    doc.cancel_stroke();
    assert!(!doc.has_active_stroke());
    assert_eq!(doc.undo_len(), 0);
    assert!(doc.composite(true).iter().all(|p| *p == Color::TRANSPARENT));
}

#[test]
fn begin_stroke_commits_an_active_shape_first() {
    let mut doc = Document::new(16, 16).unwrap();
    doc.begin_shape(Shape::new(
        ShapeKind::Rectangle,
        1,
        1,
        4,
        4,
        ShapeStyle::Fill(WHITE),
    ))
    .unwrap();
    draw_line(&mut doc, brush(RED));
    assert!(doc.active_shape().is_none());
    assert!(doc.commit_stroke().unwrap());
    assert_eq!(doc.undo_len(), 2);
}

#[test]
fn edits_are_rejected_while_stroking_and_change_nothing() {
    let mut doc = Document::new(16, 16).unwrap();
    draw_line(&mut doc, brush(RED));
    let snapshot = doc.snapshot();
    let shape = Shape::new(ShapeKind::Rectangle, 0, 0, 2, 2, ShapeStyle::Fill(RED));
    assert_eq!(doc.add_layer("Top"), Err(Error::ActiveStroke));
    assert_eq!(doc.undo(), Err(Error::ActiveStroke));
    assert_eq!(doc.redo(), Err(Error::ActiveStroke));
    assert_eq!(doc.begin_shape(shape), Err(Error::ActiveStroke));
    assert_eq!(doc.select_layer(0), Err(Error::ActiveStroke));
    assert_eq!(
        doc.resize_canvas(32, 32, Anchor { column: 1, row: 1 }, Color::TRANSPARENT),
        Err(Error::ActiveStroke)
    );
    assert!(doc.encode_project().is_err());
    assert_eq!(
        doc.begin_stroke(brush(RED), Point::new(1.0, 1.0)),
        Err(Error::ActiveStroke)
    );
    assert_eq!(doc.snapshot(), snapshot);
    assert_eq!(doc.layers().len(), 1);
    assert_eq!(doc.undo_len(), 0);
    assert!(doc.has_active_stroke());
    assert!(doc.commit_stroke().unwrap());
}

#[test]
fn invalid_points_and_missing_strokes_are_rejected() {
    let mut doc = Document::new(16, 16).unwrap();
    assert_eq!(
        doc.begin_stroke(brush(RED), Point::new(f64::NAN, 1.0)),
        Err(Error::InvalidStroke)
    );
    assert_eq!(
        doc.extend_stroke(Point::new(1.0, 1.0)),
        Err(Error::NoActiveStroke)
    );
    assert!(!doc.commit_stroke().unwrap());
    doc.begin_stroke(brush(RED), Point::new(1.0, 1.0)).unwrap();
    assert_eq!(
        doc.extend_stroke(Point::new(f64::INFINITY, 1.0)),
        Err(Error::InvalidStroke)
    );
    assert!(doc.has_active_stroke());
}
