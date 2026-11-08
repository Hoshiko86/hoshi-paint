use hoshi_core::{
    Color, Document, Error, ImageTransform, MAX_DIMENSION, MemoryReservation, Shape, ShapeKind,
    ShapeStyle, memory_budget, memory_usage,
};
fn shape(x: i32, y: i32, w: u32, h: u32, color: Color) -> Shape {
    Shape {
        kind: ShapeKind::Rectangle,
        x,
        y,
        width: w,
        height: h,
        rotation_degrees: 0,
        style: ShapeStyle::Fill(color),
    }
}
#[test]
fn maximum_canvas_edits_are_sparse_transactional_and_share_history_tiles() {
    let max = MAX_DIMENSION;
    let mut d = Document::new(max, max).unwrap();
    assert_eq!(d.layers()[0].pixels().allocated_bytes(), 0);
    assert!(
        d.try_composite(false).is_err(),
        "dense compatibility API must fail before allocating a256GiB buffer"
    );
    let red = Color::rgba(255, 0, 0, 255);
    d.begin_shape(shape((max - 20) as i32, (max - 20) as i32, 17, 17, red))
        .unwrap();
    d.commit_shape().unwrap();
    assert!(d.layers()[0].pixels().allocated_bytes() <= 4 * 128 * 128 * 4);
    assert_eq!(d.composite_pixel(max - 5, max - 5, false), red);
    assert_eq!(d.composite_pixel(0, 0, false), Color::TRANSPARENT);
    let edited = d.snapshot();
    d.undo().unwrap();
    assert_eq!(d.layers()[0].pixels().allocated_bytes(), 0);
    d.redo().unwrap();
    assert_eq!(d.snapshot(), edited);
    d.transform_image(ImageTransform::Rotate180).unwrap();
    assert_eq!(d.composite_pixel(4, 4, false), red);
    d.undo().unwrap();
    assert_eq!(d.snapshot(), edited);
    d.add_layer("Empty").unwrap();
    assert_eq!(d.layers()[1].pixels().allocated_bytes(), 0);
    d.flatten().unwrap();
    assert_eq!(d.layers().len(), 1);
    assert_eq!(d.composite_pixel(max - 5, max - 5, false), red);
    // Exhaust the explicit budget without consuming physical RAM. A failed edit
    // must retain both the committed state and the editable preview.
    let before = d.snapshot();
    let reservation = MemoryReservation::new(memory_budget() - memory_usage()).unwrap();
    let preview = shape(0, 0, 12, 12, red);
    d.begin_shape(preview).unwrap();
    assert_eq!(d.commit_shape(), Err(Error::MemoryLimit));
    assert_eq!(d.snapshot(), before);
    assert_eq!(d.active_shape(), Some(preview));
    drop(reservation);
    d.commit_shape().unwrap();
    assert_eq!(d.composite_pixel(0, 0, false), red);
    // A uniform huge layer and a full-canvas fill have constant-size storage.
    let mut white =
        Document::new_with_background(max, max, 96., Color::rgba(255, 255, 255, 255)).unwrap();
    assert!(!white.has_transparency());
    assert_eq!(white.layers()[0].pixels().allocated_bytes(), 0);
    // A full-canvas shape also uses the constant-storage uniform fill shortcut.
    let mut uniform = Document::new(max, max).unwrap();
    uniform.begin_shape(shape(0, 0, max, max, red)).unwrap();
    uniform.commit_shape().unwrap();
    assert_eq!(uniform.layers()[0].pixels().allocated_bytes(), 0);
    assert!(!uniform.has_transparency());
    white
        .resize_canvas(
            max - 1,
            max - 1,
            hoshi_core::Anchor { column: 1, row: 1 },
            Color::TRANSPARENT,
        )
        .unwrap();
    assert_eq!(white.layers()[0].pixels().allocated_bytes(), 0);
    assert!(Document::new(max + 1, 1).is_err());
}
