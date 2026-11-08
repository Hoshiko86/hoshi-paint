use hoshi_controller::{Anchor, Color, ColorSlot, Controller, ImageTransform, Resampling};
#[test]
fn rejected_image_edits_preserve_preview_and_successful_edits_commit_before_transforming() {
    let mut c = Controller::new(80, 40).unwrap();
    c.pointer_down(10., 10.).unwrap();
    c.pointer_move(30., 20.).unwrap();
    c.pointer_up().unwrap();
    let preview = c.document().active_shape();
    let pixels = c.composite();
    assert!(c.resize_image(0, 40, 96., Resampling::Nearest).is_err());
    assert!(
        c.resize_canvas(
            80,
            40,
            Anchor { row: 4, column: 1 },
            Color::rgba(0, 0, 0, 0)
        )
        .is_err()
    );
    assert_eq!(c.document().active_shape(), preview);
    assert_eq!(c.document().undo_len(), 0);
    c.resize_image(40, 20, 300., Resampling::Nearest).unwrap();
    assert_eq!(c.document().undo_len(), 2);
    assert!(c.document().active_shape().is_none());
    c.undo().unwrap();
    assert_eq!(
        (
            c.document().width(),
            c.document().height(),
            c.document().resolution()
        ),
        (80, 40, 96.)
    );
    assert_eq!(c.composite(), pixels);
    c.redo().unwrap();
    c.transform_image(ImageTransform::RotateClockwise).unwrap();
    assert_eq!((c.document().width(), c.document().height()), (20, 40));
}
#[test]
fn creating_an_image_clears_document_history_but_preserves_workspace_colors() {
    let mut c = Controller::new(20, 20).unwrap();
    c.set_primary_color(Color::rgba(10, 20, 30, 40)).unwrap();
    c.set_secondary_color(Color::rgba(100, 120, 130, 200))
        .unwrap();
    c.set_color_slot(ColorSlot::Secondary);
    c.pointer_down(2., 2.).unwrap();
    c.pointer_move(8., 8.).unwrap();
    c.pointer_up().unwrap();
    c.commit().unwrap();
    c.new_image(40, 30, 144., Color::rgba(255, 255, 255, 255))
        .unwrap();
    assert_eq!(
        (
            c.document().width(),
            c.document().height(),
            c.document().resolution()
        ),
        (40, 30, 144.)
    );
    assert_eq!(c.primary_color(), Color::rgba(10, 20, 30, 40));
    assert_eq!(c.color_slot(), ColorSlot::Secondary);
    assert_eq!(c.document().undo_len(), 0);
    assert!(
        c.composite()
            .iter()
            .all(|c| *c == Color::rgba(255, 255, 255, 255))
    );
}

#[test]
fn unavailable_flatten_does_not_commit_an_editable_shape() {
    let mut c = Controller::new(40, 40).unwrap();
    c.pointer_down(5., 5.).unwrap();
    c.pointer_move(20., 20.).unwrap();
    c.pointer_up().unwrap();
    let shape = c.document().active_shape();
    c.flatten().unwrap();
    assert_eq!(c.document().active_shape(), shape);
    assert_eq!(c.document().undo_len(), 0);
}
