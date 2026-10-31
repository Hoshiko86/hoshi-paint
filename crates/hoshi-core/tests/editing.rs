use hoshi_core::{Color, Document, Error, MAX_LAYERS, Shape, ShapeKind, ShapeStyle};

const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);

fn rectangle(x: i32, y: i32, width: u32, height: u32, color: Color) -> Shape {
    Shape {
        kind: ShapeKind::Rectangle,
        x,
        y,
        width,
        height,
        style: ShapeStyle::Fill(color),
    }
}

#[test]
fn dimensions_are_validated_before_allocation() {
    for (w, h) in [(0, 1), (1, 0), (u32::MAX, u32::MAX), (1025, 1024)] {
        assert!(matches!(Document::new(w, h), Err(Error::InvalidDimensions)));
    }
    assert!(Document::new(1024, 1024).is_ok());
}

#[test]
fn adjust_preview_cancel_preserve_document_and_history() {
    let mut doc = Document::new(4, 4).unwrap();
    doc.begin_shape(rectangle(0, 0, 2, 2, RED)).unwrap();
    assert_eq!(doc.composite(true)[0], RED);
    assert!(doc.composite(false).iter().all(|p| p.a == 0));
    doc.adjust_shape(rectangle(2, 2, 2, 2, BLUE)).unwrap();
    assert_eq!(doc.composite(true)[0], Color::TRANSPARENT);
    assert_eq!(doc.composite(true)[10], BLUE);
    assert_eq!(doc.undo_len(), 0);
    assert!(doc.layers()[0].pixels().iter().all(|p| p.a == 0));
    doc.cancel_shape();
    assert!(doc.composite(true).iter().all(|p| p.a == 0));
    assert_eq!(doc.undo_len(), 0);
}

#[test]
fn commit_is_one_action_with_exact_undo_redo() {
    let mut doc = Document::new(4, 4).unwrap();
    doc.begin_shape(rectangle(-1, -1, 3, 3, RED)).unwrap();
    let preview = doc.composite(true);
    assert!(doc.commit_shape().unwrap());
    assert_eq!(doc.composite(false), preview);
    assert_eq!(preview.iter().filter(|&&p| p == RED).count(), 4);
    assert_eq!(doc.undo_len(), 1);
    assert!(doc.undo().unwrap());
    assert!(doc.composite(false).iter().all(|p| p.a == 0));
    assert!(doc.redo().unwrap());
    assert_eq!(doc.composite(false), preview);
    assert!(!doc.redo().unwrap());
}

#[test]
fn new_mutation_invalidates_redo_but_noop_does_not() {
    let mut doc = Document::new(2, 2).unwrap();
    doc.begin_shape(rectangle(0, 0, 1, 1, RED)).unwrap();
    doc.commit_shape().unwrap();
    doc.undo().unwrap();
    doc.begin_shape(rectangle(i32::MAX, i32::MIN, 2, 2, RED))
        .unwrap();
    assert!(!doc.commit_shape().unwrap());
    assert_eq!(doc.redo_len(), 1);
    doc.begin_shape(rectangle(1, 1, 1, 1, BLUE)).unwrap();
    doc.commit_shape().unwrap();
    assert_eq!(doc.redo_len(), 0);
}

#[test]
fn outline_and_fill_have_distinct_pixels() {
    let mut doc = Document::new(5, 5).unwrap();
    let mut shape = rectangle(0, 0, 5, 5, RED);
    shape.style = ShapeStyle::Both {
        fill: RED,
        outline: BLUE,
        width: 1,
    };
    doc.begin_shape(shape).unwrap();
    let pixels = doc.composite(true);
    assert_eq!(pixels.iter().filter(|&&p| p == BLUE).count(), 16);
    assert_eq!(pixels.iter().filter(|&&p| p == RED).count(), 9);
    shape.style = ShapeStyle::Outline {
        color: BLUE,
        width: 1,
    };
    doc.adjust_shape(shape).unwrap();
    assert_eq!(doc.composite(true)[12], Color::TRANSPARENT);
}

#[test]
fn ellipse_excludes_corners_and_includes_center() {
    let mut doc = Document::new(5, 5).unwrap();
    let mut shape = rectangle(0, 0, 5, 5, RED);
    shape.kind = ShapeKind::Ellipse;
    doc.begin_shape(shape).unwrap();
    let preview = doc.composite(true);
    for i in [0, 4, 20, 24] {
        assert_eq!(preview[i], Color::TRANSPARENT);
    }
    assert_eq!(preview[12], RED);
    doc.commit_shape().unwrap();
    assert_eq!(doc.composite(false), preview);
}

#[test]
fn source_over_opacity_visibility_and_order_match_preview() {
    let mut doc = Document::new(1, 1).unwrap();
    doc.begin_shape(rectangle(0, 0, 1, 1, BLUE)).unwrap();
    doc.commit_shape().unwrap();
    let top = doc.add_layer("Ink").unwrap();
    doc.set_layer_properties(top, true, 128).unwrap();
    doc.begin_shape(rectangle(0, 0, 1, 1, RED)).unwrap();
    let preview = doc.composite(true);
    assert_eq!(preview, vec![Color::rgba(128, 0, 127, 255)]);
    doc.commit_shape().unwrap();
    assert_eq!(doc.composite(false), preview);
    doc.set_layer_properties(top, false, 128).unwrap();
    assert_eq!(doc.composite(false), vec![BLUE]);
    doc.undo().unwrap();
    assert_eq!(doc.composite(false), preview);
    doc.move_layer(top, 0).unwrap();
    assert_eq!(doc.selected_layer(), 0);
    assert_eq!(doc.composite(false), vec![BLUE]);
    doc.undo().unwrap();
    assert_eq!(doc.layers()[1].name(), "Ink");
}

#[test]
fn active_shape_is_committed_before_undo_or_layer_switch() {
    let mut doc = Document::new(2, 2).unwrap();
    doc.add_layer("top").unwrap();
    doc.begin_shape(rectangle(0, 0, 1, 1, RED)).unwrap();
    assert_eq!(doc.redo(), Err(Error::ActiveShape));
    doc.select_layer(0).unwrap();
    assert!(doc.active_shape().is_none());
    assert_eq!(doc.layers()[1].pixels()[0], RED);
    doc.undo().unwrap();
    assert_eq!(doc.layers()[1].pixels()[0], Color::TRANSPARENT);
    doc.begin_shape(rectangle(0, 0, 1, 1, BLUE)).unwrap();
    doc.undo().unwrap();
    assert!(doc.active_shape().is_none());
    assert!(doc.composite(false).iter().all(|p| p.a == 0));
}

#[test]
fn invalid_operations_do_not_change_state() {
    let mut doc = Document::new(2, 2).unwrap();
    assert_eq!(doc.delete_layer(0), Err(Error::LastLayer));
    assert_eq!(doc.select_layer(4), Err(Error::InvalidLayer));
    assert_eq!(
        doc.begin_shape(rectangle(0, 0, 0, 1, RED)),
        Err(Error::InvalidShape)
    );
    doc.begin_shape(rectangle(0, 0, 1, 1, RED)).unwrap();
    assert_eq!(
        doc.adjust_shape(rectangle(0, 0, 1, 0, RED)),
        Err(Error::InvalidShape)
    );
    assert_eq!(doc.rename_layer(0, "oops"), Err(Error::ActiveShape));
    assert_eq!(doc.layers()[0].name(), "Background");
    assert_eq!(doc.active_shape(), Some(rectangle(0, 0, 1, 1, RED)));
    assert_eq!(doc.undo_len(), 0);
}

#[test]
fn transparent_shapes_do_not_create_history() {
    let mut doc = Document::new(2, 2).unwrap();
    doc.begin_shape(rectangle(0, 0, 2, 2, Color::TRANSPARENT))
        .unwrap();
    assert!(!doc.commit_shape().unwrap());
    assert_eq!(doc.undo_len(), 0);
}

#[test]
fn layer_actions_and_selection_roundtrip_through_history() {
    let mut doc = Document::new(1, 1).unwrap();
    doc.add_layer("second").unwrap();
    doc.rename_layer(1, "renamed").unwrap();
    doc.add_layer("third").unwrap();
    doc.delete_layer(0).unwrap();
    assert_eq!(doc.selected_layer(), 1);
    assert_eq!(doc.layers()[0].name(), "renamed");
    doc.undo().unwrap();
    assert_eq!(doc.selected_layer(), 2);
    assert_eq!(doc.layers().len(), 3);
    doc.redo().unwrap();
    assert_eq!(doc.layers().len(), 2);
    assert_eq!(doc.layers()[1].name(), "third");
}

#[test]
fn history_retains_only_the_documented_number_of_actions() {
    let mut doc = Document::new(1, 1).unwrap();
    for i in 0..30 {
        doc.rename_layer(0, format!("layer {i}")).unwrap();
    }
    assert_eq!(doc.undo_len(), 16);
    for _ in 0..16 {
        assert!(doc.undo().unwrap());
    }
    assert!(!doc.undo().unwrap());
    for _ in 0..16 {
        assert!(doc.redo().unwrap());
    }
    assert_eq!(doc.layers()[0].name(), "layer 29");
}

#[test]
fn layer_limit_rejects_allocation_without_changing_state_or_redo() {
    let mut doc = Document::new(1, 1).unwrap();
    for index in 1..MAX_LAYERS {
        doc.add_layer(format!("Layer {index}")).unwrap();
    }
    doc.rename_layer(MAX_LAYERS - 1, "renamed").unwrap();
    doc.undo().unwrap();
    let selected = doc.selected_layer();
    let undo = doc.undo_len();
    assert_eq!(doc.add_layer("too many"), Err(Error::LayerLimit));
    assert_eq!(doc.layers().len(), MAX_LAYERS);
    assert_eq!(doc.selected_layer(), selected);
    assert_eq!(doc.undo_len(), undo);
    assert_eq!(doc.redo_len(), 1);
    doc.redo().unwrap();
    assert_eq!(doc.layers()[MAX_LAYERS - 1].name(), "renamed");

    doc.delete_layer(0).unwrap();
    assert_eq!(doc.add_layer("replacement"), Ok(MAX_LAYERS - 1));
}

#[test]
fn low_opacity_preserves_color_precision_before_final_rounding() {
    let mut doc = Document::new(1, 1).unwrap();
    doc.begin_shape(rectangle(0, 0, 1, 1, Color::rgba(0, 0, 255, 1)))
        .unwrap();
    doc.commit_shape().unwrap();
    let top = doc.add_layer("Low opacity").unwrap();
    doc.set_layer_properties(top, true, 23).unwrap();
    doc.begin_shape(rectangle(0, 0, 1, 1, Color::rgba(255, 0, 0, 1)))
        .unwrap();

    // Source-over with effective source alpha 23/65025, not rounded to zero.
    let expected = vec![Color::rgba(21, 0, 234, 1)];
    assert_eq!(doc.composite(true), expected);
    doc.commit_shape().unwrap();
    assert_eq!(doc.composite(false), expected);
    doc.undo().unwrap();
    assert_eq!(doc.composite(false), vec![Color::rgba(0, 0, 255, 1)]);
    doc.redo().unwrap();
    assert_eq!(doc.composite(false), expected);
}

#[test]
fn moving_any_layer_preserves_selected_layer_identity() {
    for selected in 0..4 {
        for from in 0..4 {
            for to in 0..4 {
                let mut doc = Document::new(1, 1).unwrap();
                for index in 1..4 {
                    doc.add_layer(format!("Layer {index}")).unwrap();
                }
                doc.select_layer(selected).unwrap();
                let selected_name = doc.layers()[selected].name().to_owned();
                doc.move_layer(from, to).unwrap();
                assert_eq!(doc.layers()[doc.selected_layer()].name(), selected_name);
                if from != to {
                    doc.undo().unwrap();
                    assert_eq!(doc.selected_layer(), selected);
                    doc.redo().unwrap();
                    assert_eq!(doc.layers()[doc.selected_layer()].name(), selected_name);
                }
            }
        }
    }
}
