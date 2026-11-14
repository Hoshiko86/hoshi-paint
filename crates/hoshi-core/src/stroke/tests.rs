use super::*;

#[test]
fn brush_widths_are_finite_and_bounded() {
    for width in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        0.99,
        2000.01,
    ] {
        assert_eq!(BrushTip::new(width, true), Err(Error::InvalidStroke));
    }
    for width in [BrushTip::MIN_WIDTH, BrushTip::MAX_WIDTH] {
        let tip = BrushTip::new(width, false).unwrap();
        assert_eq!(tip.width(), width);
        assert!(!tip.antialias());
    }
    assert_eq!(BrushTip::DEFAULT.width(), 2.0);
    assert!(BrushTip::DEFAULT.antialias());
}

#[test]
fn a_zero_length_segment_covers_a_disk_at_pixel_centers() {
    let mut mask = CoverageMask::new(7, 7);
    assert!(mask.is_empty());
    let point = Point::new(3.5, 3.5);
    mask.stamp_segment(point, point, BrushTip::new(2.0, false).unwrap())
        .unwrap();
    assert!(!mask.is_empty());
    for y in 0..7 {
        for x in 0..7 {
            let expected = u8::from((f64::from(x) - 3.0).hypot(f64::from(y) - 3.0) <= 1.0) * 255;
            assert_eq!(mask.coverage(x, y), expected, "({x}, {y})");
        }
    }
}

#[test]
fn antialiasing_produces_fractional_capsule_edges() {
    let mut mask = CoverageMask::new(6, 6);
    let point = Point::new(2.25, 2.5);
    mask.stamp_segment(point, point, BrushTip::new(1.0, true).unwrap())
        .unwrap();
    assert_eq!(mask.coverage(2, 2), 191);
    assert_eq!(mask.coverage(1, 2), 64);
    assert_eq!(mask.coverage(3, 2), 0);
}

#[test]
fn clipped_scanlines_match_capsule_distance_in_every_direction() {
    for (from, to) in [
        (Point::new(-4.5, 0.5), Point::new(35.5, 23.5)),
        (Point::new(8.0, 40.0), Point::new(22.0, -10.0)),
        (Point::new(18.5, 2.5), Point::new(18.5, 35.5)),
        (Point::new(-2.0, 12.0), Point::new(36.0, 12.0)),
        (Point::new(12.0, 12.0), Point::new(12.0, 12.0)),
    ] {
        for antialias in [false, true] {
            let tip = BrushTip::new(3.5, antialias).unwrap();
            let mut mask = CoverageMask::new(32, 32);
            mask.stamp_segment(from, to, tip).unwrap();
            let dx = to.x - from.x;
            let dy = to.y - from.y;
            for y in 0..32 {
                for x in 0..32 {
                    let center = Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
                    let t = if from == to {
                        0.0
                    } else {
                        (((center.x - from.x) * dx + (center.y - from.y) * dy)
                            / (dx * dx + dy * dy))
                            .clamp(0.0, 1.0)
                    };
                    let distance = center.distance(Point::new(from.x + t * dx, from.y + t * dy));
                    let expected = if antialias {
                        ((tip.width() / 2.0 + 0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8
                    } else {
                        u8::from(distance <= tip.width() / 2.0) * 255
                    };
                    assert_eq!(
                        mask.coverage(x, y),
                        expected,
                        "{from:?} -> {to:?}, ({x}, {y}), AA={antialias}"
                    );
                }
            }
        }
    }
}

#[test]
fn overlapping_and_reversed_segments_combine_with_maximum() {
    let mut mask = CoverageMask::new(24, 12);
    let from = Point::new(3.0, 4.0);
    let to = Point::new(20.0, 7.0);
    mask.stamp_segment(from, to, BrushTip::DEFAULT).unwrap();
    let before: Vec<_> = (0..12)
        .flat_map(|y| (0..24).map(move |x| (x, y)))
        .map(|(x, y)| mask.coverage(x, y))
        .collect();
    mask.stamp_segment(to, from, BrushTip::DEFAULT).unwrap();
    mask.stamp_segment(from, to, BrushTip::DEFAULT).unwrap();
    let after: Vec<_> = (0..12)
        .flat_map(|y| (0..24).map(move |x| (x, y)))
        .map(|(x, y)| mask.coverage(x, y))
        .collect();
    assert_eq!(before, after);
}

#[test]
fn off_canvas_segments_clip_and_outside_segments_allocate_nothing() {
    let mut mask = CoverageMask::new(137, 7);
    let tip = BrushTip::new(1.0, false).unwrap();
    mask.stamp_segment(Point::new(-10.0, -10.0), Point::new(-5.0, -5.0), tip)
        .unwrap();
    assert!(mask.is_empty());
    mask.stamp_segment(Point::new(-5.0, 3.5), Point::new(150.0, 3.5), tip)
        .unwrap();
    assert_eq!(mask.coverage(0, 3), 255);
    assert_eq!(mask.coverage(136, 3), 255);
    assert_eq!(mask.coverage(137, 3), 0);
    assert_eq!(mask.coverage(0, 7), 0);
    let mut rects: Vec<_> = mask.covered_rects().collect();
    rects.sort_unstable();
    assert_eq!(rects, [(0, 0, 128, 7), (128, 0, 137, 7)]);
}

#[test]
fn a_long_thin_stroke_allocates_only_its_tile_row() {
    let mut mask = CoverageMask::new(crate::MAX_DIMENSION, crate::MAX_DIMENSION);
    mask.stamp_segment(
        Point::new(-5.0, 0.5),
        Point::new(f64::from(crate::MAX_DIMENSION) + 5.0, 0.5),
        BrushTip::new(1.0, false).unwrap(),
    )
    .unwrap();
    assert_eq!(
        mask.tiles.len(),
        (crate::MAX_DIMENSION / TILE_SIZE) as usize
    );
    assert!(
        mask.covered_rects()
            .all(|(_, top, _, bottom)| top == 0 && bottom == TILE_SIZE)
    );
    assert_eq!(mask.coverage(crate::MAX_DIMENSION - 1, 0), 255);
    assert_eq!(mask.coverage(0, 1), 0);
}

#[test]
fn a_long_diagonal_does_not_allocate_its_bounding_rectangle() {
    let size = crate::MAX_DIMENSION;
    let mut mask = CoverageMask::new(size, size);
    mask.stamp_segment(
        Point::new(0.5, 0.5),
        Point::new(f64::from(size) - 0.5, f64::from(size) - 0.5),
        BrushTip::new(1.0, false).unwrap(),
    )
    .unwrap();
    assert_eq!(mask.tiles.len(), (size / TILE_SIZE) as usize);
    assert_eq!(mask.coverage(size - 1, size - 1), 255);
    assert_eq!(mask.coverage(size - 1, 0), 0);
}

#[test]
fn nonfinite_endpoints_are_rejected_without_mutation() {
    let mut mask = CoverageMask::new(10, 10);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            mask.stamp_segment(
                Point::new(value, 1.0),
                Point::new(2.0, 2.0),
                BrushTip::DEFAULT
            ),
            Err(Error::InvalidStroke)
        );
        assert!(mask.is_empty());
    }
}

#[test]
fn extreme_finite_endpoints_keep_horizontal_and_diagonal_coverage_finite() {
    let mut horizontal = CoverageMask::new(16, 16);
    let tip = BrushTip::new(1.0, false).unwrap();
    horizontal
        .stamp_segment(Point::new(-f64::MAX, 3.5), Point::new(f64::MAX, 3.5), tip)
        .unwrap();
    for x in 0..16 {
        assert_eq!(horizontal.coverage(x, 3), 255);
        assert_eq!(horizontal.coverage(x, 4), 0);
    }
    let mut diagonal = CoverageMask::new(16, 16);
    diagonal
        .stamp_segment(
            Point::new(-f64::MAX, -f64::MAX),
            Point::new(f64::MAX, f64::MAX),
            tip,
        )
        .unwrap();
    for y in 0..16 {
        for x in 0..16 {
            assert_eq!(diagonal.coverage(x, y), u8::from(x == y) * 255);
        }
    }
}

#[test]
fn empty_documents_and_zero_coverage_allocate_nothing() {
    for (width, height) in [(0, 0), (0, 8), (8, 0)] {
        let mut mask = CoverageMask::new(width, height);
        mask.stamp_segment(
            Point::new(2.0, 2.0),
            Point::new(4.0, 4.0),
            BrushTip::DEFAULT,
        )
        .unwrap();
        assert!(mask.is_empty());
    }
    let mut mask = CoverageMask::new(8, 8);
    let point = Point::new(-1.0, -1.0);
    mask.stamp_segment(point, point, BrushTip::new(1.0, true).unwrap())
        .unwrap();
    assert!(mask.is_empty());
}

#[test]
fn erase_rounds_alpha_and_canonicalizes_fully_erased_pixels() {
    let destination = Color::rgba(21, 42, 63, 200);
    assert_eq!(
        StrokeBlend::Erase.apply(destination, 255),
        Color::TRANSPARENT
    );
    assert_eq!(
        StrokeBlend::Erase.apply(destination, 128),
        Color::rgba(21, 42, 63, 100)
    );
    assert_eq!(
        StrokeBlend::Erase.apply(Color::rgba(21, 42, 63, 1), 254),
        Color::TRANSPARENT
    );
}

#[test]
fn zero_coverage_and_zero_alpha_paint_preserve_destination_exactly() {
    for destination in [Color::rgba(1, 2, 3, 0), Color::rgba(40, 50, 60, 255)] {
        for blend in [
            StrokeBlend::Paint(Color::rgba(9, 8, 7, 255)),
            StrokeBlend::Erase,
        ] {
            assert_eq!(blend.apply(destination, 0), destination);
        }
        assert_eq!(
            StrokeBlend::Paint(Color::rgba(9, 8, 7, 0)).apply(destination, 255),
            destination
        );
    }
}

#[test]
fn paint_quantizes_coverage_alpha_before_source_over() {
    let source = Color::rgba(100, 80, 60, 127);
    let destination = Color::rgba(20, 40, 60, 90);
    let expected = Color { a: 64, ..source }.over(destination, 255);
    assert_eq!(StrokeBlend::Paint(source).apply(destination, 128), expected);
}

#[test]
fn failed_multi_tile_reservation_preserves_existing_pixels_and_releases_pending_tiles() {
    const CHILD: &str = "HOSHI_STROKE_ATOMICITY_TEST";
    if std::env::var_os(CHILD).is_none() {
        // The shared budget is process-global: isolate this failure injection
        // from concurrently running document/resampling tests.
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "stroke::tests::failed_multi_tile_reservation_preserves_existing_pixels_and_releases_pending_tiles", "--nocapture"])
            .env(CHILD, "1").output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(
            stdout.lines().any(|line| line.trim() == "running 1 test"),
            "isolated stroke atomicity test ran no test; is the name wrong?\n{stdout}"
        );
        return;
    }
    let bytes = u64::from(TILE_SIZE * TILE_SIZE);
    crate::configure_memory_budget(bytes * 3).unwrap();
    let mut mask = CoverageMask::new(TILE_SIZE * 5, 4);
    let point = Point::new(0.5, 0.5);
    let tip = BrushTip::new(1.0, false).unwrap();
    mask.stamp_segment(point, point, tip).unwrap();
    let used = crate::memory_usage();
    assert_eq!(used, bytes);
    assert_eq!(
        mask.stamp_segment(point, Point::new(f64::from(TILE_SIZE * 5) - 0.5, 0.5), tip),
        Err(Error::MemoryLimit)
    );
    assert_eq!(crate::memory_usage(), used);
    assert_eq!(mask.covered_rects().count(), 1);
    assert_eq!(mask.coverage(0, 0), 255);
    assert_eq!(mask.coverage(1, 0), 0);
    assert_eq!(mask.coverage(TILE_SIZE, 0), 0);
    drop(mask);
    assert_eq!(crate::memory_usage(), 0);
}

#[test]
fn reversed_segments_have_identical_coverage() {
    // Regression for the Codex review finding on #57: roundoff in the line
    // distance made hard-edge boundary pixels depend on segment direction.
    let a = Point::new(-3.5, -0.5);
    let b = Point::new(12.5, 11.5);
    for antialias in [false, true] {
        let tip = BrushTip::new(2.0, antialias).unwrap();
        let mut forward = CoverageMask::new(16, 16);
        let mut backward = CoverageMask::new(16, 16);
        forward.stamp_segment(a, b, tip).unwrap();
        backward.stamp_segment(b, a, tip).unwrap();
        for y in 0..16 {
            for x in 0..16 {
                assert_eq!(
                    forward.coverage(x, y),
                    backward.coverage(x, y),
                    "({x}, {y}), AA={antialias}"
                );
            }
        }
    }
}
