use hoshi_core::{
    Anchor, Color, Document, Error, ImageTransform, Resampling, Shape, ShapeKind, ShapeStyle,
};

fn pixel(d: &mut Document, x: i32, y: i32, color: Color) {
    d.begin_shape(Shape {
        kind: ShapeKind::Rectangle,
        x,
        y,
        width: 1,
        height: 1,
        rotation_degrees: 0,
        style: ShapeStyle::Fill(color),
    })
    .unwrap();
    d.commit_shape().unwrap();
}
fn fixture() -> Document {
    let mut d = Document::new(2, 3).unwrap();
    for i in 0..6 {
        pixel(&mut d, i % 2, i / 2, Color::rgba((i + 1) as u8, 0, 0, 255));
    }
    d
}
#[test]
fn whole_image_transforms_have_exact_pixel_mappings_and_restore_dimensions() {
    for (transform, dimensions, expected) in [
        (
            ImageTransform::FlipHorizontal,
            (2, 3),
            vec![2, 1, 4, 3, 6, 5],
        ),
        (ImageTransform::FlipVertical, (2, 3), vec![5, 6, 3, 4, 1, 2]),
        (
            ImageTransform::RotateClockwise,
            (3, 2),
            vec![5, 3, 1, 6, 4, 2],
        ),
        (
            ImageTransform::RotateCounterClockwise,
            (3, 2),
            vec![2, 4, 6, 1, 3, 5],
        ),
        (ImageTransform::Rotate180, (2, 3), vec![6, 5, 4, 3, 2, 1]),
    ] {
        let mut d = fixture();
        d.set_resolution(300.).unwrap();
        let original = d.composite(false);
        d.transform_image(transform).unwrap();
        assert_eq!((d.width(), d.height()), dimensions);
        assert_eq!(
            d.composite(false).iter().map(|c| c.r).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(d.resolution(), 300.);
        d.undo().unwrap();
        assert_eq!((d.width(), d.height()), (2, 3));
        assert_eq!(d.composite(false), original);
        d.redo().unwrap();
        assert_eq!((d.width(), d.height()), dimensions);
    }
}
#[test]
fn every_canvas_anchor_pads_and_crops_without_scaling_layers() {
    for row in 0..3 {
        for column in 0..3 {
            let mut d = Document::new(2, 2).unwrap();
            let red = Color::rgba(255, 0, 0, 255);
            pixel(&mut d, 0, 0, red);
            d.add_layer("Ink").unwrap();
            pixel(&mut d, 1, 1, Color::rgba(0, 0, 255, 128));
            d.set_layer_properties(1, false, 100).unwrap();
            let original = d
                .layers()
                .iter()
                .map(|l| l.pixels().to_vec())
                .collect::<Vec<_>>();
            let fill = Color::rgba(1, 2, 3, 100);
            let anchor = Anchor { column, row };
            d.resize_canvas(4, 4, anchor, fill).unwrap();
            assert_eq!(
                d.layers()[0].pixels()[row as usize * 4 + column as usize],
                red
            );
            assert_eq!(
                d.layers()[1].pixels()[((row + 1) * 4 + column + 1) as usize],
                Color::rgba(0, 0, 255, 128)
            );
            assert_eq!(d.layers()[1].pixels()[0], Color::TRANSPARENT);
            assert!(!d.layers()[1].visible());
            assert_eq!(d.layers()[1].opacity(), 100);
            d.resize_canvas(2, 2, anchor, Color::TRANSPARENT).unwrap();
            assert_eq!(
                d.layers()
                    .iter()
                    .map(|l| l.pixels().to_vec())
                    .collect::<Vec<_>>(),
                original
            );
            d.undo().unwrap();
            assert_eq!((d.width(), d.height()), (4, 4));
            d.undo().unwrap();
            assert_eq!((d.width(), d.height()), (2, 2));
        }
    }
}
#[test]
fn resizing_is_premultiplied_and_resolution_is_undoable_even_without_resampling() {
    for method in [
        Resampling::Nearest,
        Resampling::Bilinear,
        Resampling::Bicubic,
        Resampling::Lanczos,
    ] {
        let mut d = Document::new(2, 1).unwrap();
        pixel(&mut d, 0, 0, Color::rgba(255, 0, 0, 255));
        d.resize_image(5, 1, 300., method).unwrap();
        for color in d.layers()[0].pixels() {
            if color.a != 0 {
                assert_eq!((color.r, color.g, color.b), (255, 0, 0));
            }
        }
        assert_eq!(d.resolution(), 300.);
        d.undo().unwrap();
        assert_eq!((d.width(), d.height(), d.resolution()), (2, 1, 96.));
        d.redo().unwrap();
        assert_eq!((d.width(), d.height()), (5, 1));
        let pixels = d.composite(false);
        d.resize_image(5, 1, 144., method).unwrap();
        assert_eq!(d.composite(false), pixels);
        d.undo().unwrap();
        assert_eq!(d.resolution(), 300.);
    }
}
#[test]
fn flatten_preserves_composite_and_undo_restores_hidden_layers_and_properties() {
    let mut d = fixture();
    d.add_layer("Visible ink").unwrap();
    pixel(&mut d, 1, 1, Color::rgba(20, 220, 70, 128));
    d.set_layer_properties(1, true, 90).unwrap();
    d.add_layer("Hidden ink").unwrap();
    pixel(&mut d, 0, 0, Color::rgba(255, 255, 255, 255));
    d.set_layer_properties(2, false, 255).unwrap();
    let expected = d.composite(false);
    d.flatten().unwrap();
    assert_eq!(d.layers().len(), 1);
    assert_eq!(d.composite(false), expected);
    d.undo().unwrap();
    assert_eq!(d.layers().len(), 3);
    assert_eq!(d.selected_layer(), 2);
    assert_eq!(d.layers()[1].opacity(), 90);
    assert!(!d.layers()[2].visible());
    d.redo().unwrap();
    assert_eq!(d.composite(false), expected);
    let length = d.undo_len();
    d.flatten().unwrap();
    assert_eq!(d.undo_len(), length);
}
#[test]
fn invalid_dimensions_and_resolution_leave_pixels_and_history_unchanged() {
    let mut d = fixture();
    let pixels = d.composite(false);
    let history = d.undo_len();
    assert_eq!(
        d.resize_canvas(0, 3, Anchor { row: 1, column: 1 }, Color::TRANSPARENT),
        Err(Error::InvalidDimensions)
    );
    assert!(
        d.resize_image(65536, 65536, 96., Resampling::Bilinear)
            .is_err()
    );
    assert!(d.resize_image(3, 3, f64::NAN, Resampling::Nearest).is_err());
    assert!(d.set_resolution(0.).is_err());
    assert_eq!(d.composite(false), pixels);
    assert_eq!(d.undo_len(), history);
    assert_eq!((d.width(), d.height(), d.resolution()), (2, 3, 96.));
}
