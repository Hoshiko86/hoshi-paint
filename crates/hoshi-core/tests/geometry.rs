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

#[test]
fn affine_composition_inverse_and_exact_quarter_turns() {
    use hoshi_core::geometry::Affine;
    let p = Point::new(17., -9.);
    let pivot = Point::new(3., 5.);
    assert_eq!(Affine::IDENTITY.apply(p), p);
    assert_eq!(Affine::IDENTITY.coefficients(), [1., 0., 0., 1., 0., 0.]);
    let translate = Affine::translate(7.25, -3.5);
    let rotate = Affine::rotate_about(pivot, 37.);
    let scale = Affine::scale_about(pivot, 2., -3.);
    let shear = Affine::from_coefficients([2., 0.3, 0.7, 3., 8., -4.]);
    for (t, inverse) in [
        (translate, Affine::translate(-7.25, 3.5)),
        (rotate, Affine::rotate_about(pivot, -37.)),
        (scale, Affine::scale_about(pivot, 0.5, -1. / 3.)),
        (shear, shear.inverse().unwrap()),
    ] {
        assert!(t.then(&inverse).apply(p).distance(p) < 1e-9);
        assert!(inverse.apply(t.apply(p)).distance(p) < 1e-9);
    }
    let quarter = Affine::rotate_about(pivot, 90.);
    assert_eq!(
        quarter
            .then(&quarter)
            .then(&quarter)
            .then(&quarter)
            .apply(p),
        p
    );
    assert_ne!(rotate.then(&scale).apply(p), scale.then(&rotate).apply(p));
    for angle in [0., 37., 90., 180., 270.] {
        for horizontal in [false, true] {
            for vertical in [false, true] {
                let frame = Frame {
                    angle_degrees: angle,
                    flip_horizontal: horizontal,
                    flip_vertical: vertical,
                    ..frame(0)
                };
                assert!(
                    Affine::from_frame(frame)
                        .apply(p)
                        .distance(frame.local_to_document(p))
                        < 1e-12
                );
            }
        }
    }
    for invalid in [
        Affine::scale(0., 1.),
        Affine::scale(f64::NAN, 1.),
        Affine::scale(f64::MAX, f64::MAX),
        Affine::from_coefficients([1e-320, 0., 0., 1., 0., 0.]),
    ] {
        assert!(matches!(
            invalid.inverse(),
            Err(hoshi_core::Error::InvalidShape)
        ));
    }
}

#[test]
fn affine_identity_preserves_finite_coordinate_bits() {
    use hoshi_core::geometry::Affine;
    for point in [
        Point::new(-0.0, 0.0),
        Point::new(f64::MIN_POSITIVE, -f64::MIN_POSITIVE),
        Point::new(f64::MAX, -f64::MAX),
    ] {
        let mapped = Affine::IDENTITY.apply(point);
        assert_eq!(mapped.x.to_bits(), point.x.to_bits());
        assert_eq!(mapped.y.to_bits(), point.y.to_bits());
    }
}
