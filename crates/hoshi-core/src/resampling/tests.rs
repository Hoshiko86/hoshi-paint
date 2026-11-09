use super::*;
// Retain the dense hand-computed reference fixtures while production filtering
// reads sparse tiles and returns sparse tiles.
fn resize(
    input: &[Color],
    source: (u32, u32),
    target: (u32, u32),
    options: ResizeOptions,
) -> Vec<Color> {
    let input = PixelStore::from_vec(source.0, source.1, input.to_vec()).unwrap();
    super::resize(&input, source, target, options)
        .unwrap()
        .to_vec()
}
const METHODS: [Resampling; 8] = [
    Resampling::Bicubic,
    Resampling::BicubicSmooth,
    Resampling::Bilinear,
    Resampling::BilinearLowQuality,
    Resampling::AdaptiveSharp,
    Resampling::Lanczos,
    Resampling::Fant,
    Resampling::Nearest,
];
#[test]
fn identity_and_nearest_preserve_all_stored_rgba_bits_even_with_gamma() {
    let input = [
        Color::rgba(1, 2, 3, 0),
        Color::rgba(15, 83, 204, 1),
        Color::rgba(50, 60, 70, 128),
    ];
    for method in METHODS {
        for gamma_correction in [false, true] {
            let options = ResizeOptions {
                method,
                gamma_correction,
            };
            assert_eq!(resize(&input, (3, 1), (3, 1), options), input);
        }
    }
    let out = resize(
        &input,
        (3, 1),
        (6, 1),
        ResizeOptions {
            method: Resampling::Nearest,
            gamma_correction: true,
        },
    );
    assert_eq!(
        out,
        vec![input[0], input[0], input[1], input[1], input[2], input[2]]
    );
}
#[test]
fn gamma_correction_averages_light_instead_of_encoded_channel_values() {
    let input = [Color::rgba(0, 0, 0, 255), Color::rgba(255, 255, 255, 255)];
    for gamma_correction in [false, true] {
        let out = resize(
            &input,
            (2, 1),
            (1, 1),
            ResizeOptions {
                method: Resampling::Fant,
                gamma_correction,
            },
        );
        let value = if gamma_correction { 188 } else { 128 };
        assert_eq!(out, vec![Color::rgba(value, value, value, 255)]);
    }
}
#[test]
fn fant_uses_fractional_pixel_coverage_and_filters_alpha_as_coverage() {
    let input = [
        Color::rgba(255, 0, 0, 255),
        Color::rgba(0, 255, 0, 255),
        Color::rgba(0, 0, 255, 255),
    ];
    assert_eq!(
        resize(&input, (3, 1), (2, 1), Resampling::Fant.into()),
        vec![Color::rgba(170, 85, 0, 255), Color::rgba(0, 85, 170, 255)]
    );
    let input = [Color::rgba(255, 0, 0, 255), Color::rgba(0, 0, 255, 0)];
    for method in METHODS {
        if method == Resampling::Nearest {
            continue;
        }
        for gamma_correction in [false, true] {
            let out = resize(
                &input,
                (2, 1),
                (1, 1),
                ResizeOptions {
                    method,
                    gamma_correction,
                },
            );
            assert_eq!(out, vec![Color::rgba(255, 0, 0, 128)], "{method:?}");
        }
    }
}
#[test]
fn smooth_bicubic_uses_bspline_weights_and_sharp_bicubic_interpolates() {
    assert!((cubic(0., 1., 0.) - 2. / 3.).abs() < 1e-12);
    assert!((cubic(1., 1., 0.) - 1. / 6.).abs() < 1e-12);
    assert_eq!(cubic(0., 0., 0.5), 1.);
    assert_eq!(cubic(1., 0., 0.5), 0.);
    let input = [
        Color::rgba(0, 0, 0, 255),
        Color::rgba(255, 255, 255, 255),
        Color::rgba(0, 0, 0, 255),
    ];
    let sharp = resize(&input, (3, 1), (9, 1), Resampling::Bicubic.into());
    let smooth = resize(&input, (3, 1), (9, 1), Resampling::BicubicSmooth.into());
    assert_eq!(sharp[4].r, 255);
    assert_eq!(smooth[4].r, 170);
}
#[test]
fn bilinear_low_quality_aliases_on_reduction_but_matches_bilinear_when_enlarging() {
    let input: Vec<_> = (0..8)
        .map(|x| Color::rgba(if x % 2 == 0 { 0 } else { 255 }, 0, 0, 255))
        .collect();
    let hq = resize(&input, (8, 1), (3, 1), Resampling::Bilinear.into());
    let lq = resize(
        &input,
        (8, 1),
        (3, 1),
        Resampling::BilinearLowQuality.into(),
    );
    assert_ne!(hq, lq);
    assert_eq!(
        resize(&input, (8, 1), (17, 1), Resampling::Bilinear.into()),
        resize(
            &input,
            (8, 1),
            (17, 1),
            Resampling::BilinearLowQuality.into()
        )
    );
    assert_eq!(
        resize(&input, (8, 1), (17, 1), Resampling::Fant.into()),
        resize(&input, (8, 1), (17, 1), Resampling::Bilinear.into())
    );
}
#[test]
fn filters_preserve_constant_colors_edges_and_single_pixel_axes() {
    for method in METHODS {
        for gamma_correction in [false, true] {
            for (source, target) in [
                ((3, 2), (7, 5)),
                ((7, 5), (3, 2)),
                ((1, 8), (8, 1)),
                ((8, 1), (1, 8)),
            ] {
                let color = Color::rgba(37, 119, 213, 83);
                let input = vec![color; (source.0 * source.1) as usize];
                let output = resize(
                    &input,
                    source,
                    target,
                    ResizeOptions {
                        method,
                        gamma_correction,
                    },
                );
                assert_eq!(
                    output,
                    vec![color; (target.0 * target.1) as usize],
                    "{method:?}"
                );
            }
        }
    }
}
#[test]
fn adaptive_policy_selects_sharp_filters_for_each_scale_direction() {
    let input: Vec<_> = (0..7).map(|i| Color::rgba(i * 35, 70, 20, 255)).collect();
    assert_eq!(
        resize(&input, (7, 1), (3, 1), Resampling::AdaptiveSharp.into()),
        resize(&input, (7, 1), (3, 1), Resampling::Lanczos.into())
    );
    assert_eq!(
        resize(&input, (7, 1), (13, 1), Resampling::AdaptiveSharp.into()),
        resize(&input, (7, 1), (13, 1), Resampling::Bicubic.into())
    );
}

#[test]
fn two_dimensional_low_quality_uses_only_the_closest_four_pixels() {
    let input: Vec<_> = (0..16)
        .map(|i| {
            let white = matches!(i, 5 | 6 | 9 | 10);
            Color::rgba(if white { 255 } else { 0 }, 0, 0, 255)
        })
        .collect();
    assert_eq!(
        resize(
            &input,
            (4, 4),
            (1, 1),
            Resampling::BilinearLowQuality.into()
        ),
        vec![Color::rgba(255, 0, 0, 255)]
    );
    assert_eq!(
        resize(&input, (4, 4), (1, 1), Resampling::Fant.into()),
        vec![Color::rgba(64, 0, 0, 255)]
    );
}
#[test]
fn fractional_area_in_two_dimensions_matches_hand_computed_coverage() {
    // 3 by 2 -> 2 by 1: each output covers 1.5 columns and both rows.
    let input = [
        Color::rgba(255, 0, 0, 255),
        Color::rgba(0, 255, 0, 255),
        Color::rgba(0, 0, 255, 255),
        Color::rgba(0, 0, 0, 255),
        Color::rgba(0, 0, 0, 255),
        Color::rgba(0, 0, 0, 255),
    ];
    assert_eq!(
        resize(&input, (3, 2), (2, 1), Resampling::Fant.into()),
        vec![Color::rgba(85, 43, 0, 255), Color::rgba(0, 43, 85, 255)]
    );
}

#[test]
fn maximum_uniform_canvas_resizes_without_axis_or_image_buffers() {
    let color = Color::rgba(29, 119, 211, 83);
    let input = PixelStore::new(262_144, 262_144, color);
    for method in METHODS {
        let output = super::resize(
            &input,
            (262_144, 262_144),
            (131_072, 262_144),
            ResizeOptions {
                method,
                gamma_correction: true,
            },
        )
        .unwrap();
        assert!(output.is_uniform());
        assert_eq!(output.get(131_071, 262_143), color);
    }
}
#[test]
fn maximum_sparse_canvas_filters_only_edited_regions() {
    let mut input = PixelStore::new(262_144, 262_144, Color::TRANSPARENT);
    input
        .set(130_000, 170_000, Color::rgba(255, 30, 20, 255))
        .unwrap();
    let output = super::resize(
        &input,
        (262_144, 262_144),
        (131_072, 131_072),
        Resampling::Fant.into(),
    )
    .unwrap();
    assert_eq!(output.get(65_000, 85_000), Color::rgba(255, 30, 20, 64));
    assert_eq!(output.get(0, 0), Color::TRANSPARENT);
    assert_eq!(output.get(131_071, 131_071), Color::TRANSPARENT);
    assert!(output.allocated_rects().count() <= 4);
}
#[test]
fn anisotropic_scanline_pass_order_preserves_transparent_colors() {
    let mut input = PixelStore::new(4096, 2, Color::TRANSPARENT);
    for x in 0..4096 {
        input.set(x, 1, Color::rgba(30, 70, 180, 255)).unwrap();
    }
    for method in [Resampling::Fant, Resampling::Bilinear, Resampling::Lanczos] {
        let output = super::resize(&input, (4096, 2), (2, 4096), method.into()).unwrap();
        assert_eq!(output.get(0, 4095), Color::rgba(30, 70, 180, 255));
        assert_eq!(output.get(1, 0), Color::TRANSPARENT);
        assert!((output.get(0, 2048).a as i32 - 128).abs() <= 1);
    }
}

#[test]
fn shrinking_a_maximum_sparse_canvas_does_not_scan_its_implicit_background() {
    let mut input = PixelStore::new(262_144, 262_144, Color::TRANSPARENT);
    input
        .set(130_000, 170_000, Color::rgba(255, 30, 20, 255))
        .unwrap();
    let output =
        super::resize(&input, (262_144, 262_144), (2, 2), Resampling::Fant.into()).unwrap();
    assert!(output.is_uniform());
    assert_eq!(output.get(0, 0), Color::TRANSPARENT);
}

#[test]
fn sparse_and_uniform_transparent_rgb_is_only_retained_by_copying_filters() {
    let hidden = Color::rgba(19, 83, 204, 0);
    let uniform = PixelStore::new(262_144, 262_144, hidden);
    for method in METHODS {
        let output = super::resize(
            &uniform,
            (262_144, 262_144),
            (131_072, 262_144),
            method.into(),
        )
        .unwrap();
        assert_eq!(
            output.get(0, 0),
            if method == Resampling::Nearest {
                hidden
            } else {
                Color::TRANSPARENT
            }
        );
    }
    let mut sparse = PixelStore::new(262_144, 262_144, Color::TRANSPARENT);
    sparse.set(130_001, 170_001, hidden).unwrap();
    for method in [Resampling::Nearest, Resampling::Fant] {
        let output = super::resize(
            &sparse,
            (262_144, 262_144),
            (131_072, 131_072),
            method.into(),
        )
        .unwrap();
        assert_eq!(
            output.get(65_000, 85_000),
            if method == Resampling::Nearest {
                hidden
            } else {
                Color::TRANSPARENT
            }
        );
        if method == Resampling::Fant {
            assert!(output.is_uniform());
        }
    }
}

#[test]
fn oversized_nonuniform_enlargement_fails_before_allocating_its_output() {
    let input = PixelStore::from_vec(
        2,
        3,
        vec![
            Color::rgba(255, 0, 0, 255),
            Color::rgba(0, 255, 0, 255),
            Color::rgba(0, 0, 255, 255),
            Color::rgba(0, 255, 255, 255),
            Color::rgba(255, 0, 255, 255),
            Color::rgba(255, 255, 0, 255),
        ],
    )
    .unwrap();
    for method in METHODS {
        assert!(
            matches!(
                super::resize(&input, (2, 3), (65_536, 65_536), method.into()),
                Err(Error::MemoryLimit)
            ),
            "{method:?}"
        );
    }
}
#[test]
fn overlapping_filter_regions_are_only_budgeted_once() {
    // Two rectangles occupy four tiles each, but their common tile column must
    // not count twice in the memory estimate.
    let (bytes, _) = output_footprint(&[(0, 0, 256, 256), (128, 0, 384, 256)], (384, 256)).unwrap();
    assert_eq!(bytes, 6 * 128 * 128 * 4);
}

#[test]
fn output_footprint_clips_partial_edge_tiles_to_actual_dimensions() {
    for (width, height) in [(1, 262_144), (262_144, 1)] {
        let (bytes, max_row) = output_footprint(&[(0, 0, width, height)], (width, height)).unwrap();
        assert_eq!(bytes, 1024 * 1024);
        assert_eq!(max_row, width);
    }
    let (bytes, max_row) = output_footprint(&[(0, 0, 129, 129)], (129, 129)).unwrap();
    assert_eq!(bytes, 129 * 129 * 4);
    assert_eq!(max_row, 129);
}
