use hoshi_core::geometry::{Frame, Point, ResizeConstraints, ResizeHandle};
use hoshi_core::{Color, Shape, ShapeKind, ShapeStyle};

fn frame(angle: i32) -> Frame {
    let mut shape = Shape::new(
        ShapeKind::Rectangle,
        40,
        50,
        80,
        40,
        ShapeStyle::Fill(Color::rgba(0, 0, 0, 255)),
    );
    shape.rotation_degrees = angle;
    Frame::from_shape(shape)
}

#[test]
fn every_handle_preserves_its_anchor_or_center_at_arbitrary_rotations() {
    for angle in [0, 37, 90, 183] {
        for handle in ResizeHandle::ALL {
            for from_center in [false, true] {
                for preserve_aspect in [false, true] {
                    let original = frame(angle);
                    let grab = original.handle(handle);
                    let (sx, sy) = handle.signs();
                    let movement = original.vector_to_document(Point::new(sx * 20., sy * 10.));
                    let resized = original.resized(
                        handle,
                        grab,
                        grab.translated(movement.x, movement.y),
                        ResizeConstraints {
                            from_center,
                            preserve_aspect,
                        },
                    );
                    if from_center {
                        assert!(resized.center.distance(original.center) < 1e-9);
                    } else {
                        let opposite = ResizeHandle::ALL[(handle.index() + 4) % 8];
                        assert!(
                            resized.handle(opposite).distance(original.handle(opposite)) < 1e-9
                        );
                    }
                    if preserve_aspect {
                        assert!((resized.width / resized.height - 2.).abs() < 0.04);
                    }
                }
            }
        }
    }
}

#[test]
fn crossing_handles_mirrors_local_geometry_without_accumulating_flips() {
    let original = frame(37);
    let handle = ResizeHandle::BottomRight;
    let fixed = original.handle(ResizeHandle::TopLeft);
    let delta = original.vector_to_document(Point::new(-30., -20.));
    let pointer = fixed.translated(delta.x, delta.y);
    let resized = original.resized(
        handle,
        original.handle(handle),
        pointer,
        ResizeConstraints::default(),
    );
    assert!(resized.flip_horizontal && resized.flip_vertical);
    assert_eq!((resized.width, resized.height), (30., 20.));
    let local = Point::new(7., -4.);
    let restored = resized.prepare().to_local(resized.local_to_document(local));
    assert!(local.distance(restored) < 1e-9);
    let unchanged = original.resized(
        handle,
        original.handle(handle),
        original.handle(handle),
        ResizeConstraints::default(),
    );
    assert!(!unchanged.flip_horizontal && !unchanged.flip_vertical);
}

#[test]
fn rotation_about_custom_pivot_moves_center_and_preserves_size() {
    let original = frame(0);
    let pivot = Point::new(20., 30.);
    let rotated = original.rotated_about(pivot, 90.);
    assert!(rotated.center.distance(Point::new(-20., 90.)) < 1e-9);
    assert_eq!((rotated.width, rotated.height), (80., 40.));
    assert!(
        rotated
            .rotated_about(pivot, 0.)
            .center
            .distance(original.center)
            < 1e-9
    );
}
