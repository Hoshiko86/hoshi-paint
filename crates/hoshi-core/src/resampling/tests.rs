use super::*;
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
