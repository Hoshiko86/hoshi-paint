use super::Color;

// The pre-optimization formula is deliberately independent of the identity
// branches so changes remain byte-for-byte compatible at rounding boundaries.
fn reference_over(source: Color, destination: Color, opacity: u8) -> Color {
    let alpha = u64::from(source.a) * u64::from(opacity);
    let back = u64::from(destination.a) * (65_025 - alpha);
    let denominator = alpha * 255 + back;
    if denominator == 0 {
        return Color::TRANSPARENT;
    }
    let channel = |source: u8, target: u8| {
        ((u64::from(source) * alpha * 255 + u64::from(target) * back + denominator / 2)
            / denominator) as u8
    };
    Color::rgba(
        channel(source.r, destination.r),
        channel(source.g, destination.g),
        channel(source.b, destination.b),
        ((denominator + 32_512) / 65_025) as u8,
    )
}

#[test]
fn source_over_fast_paths_match_reference_for_every_source_alpha_and_opacity() {
    for (r, g, b) in [(19, 87, 131), (255, 1, 0)] {
        for (dr, dg, db) in [(13, 197, 251), (0, 0, 0)] {
            for destination_alpha in [0, 1, 127, 254, 255] {
                let destination = Color::rgba(dr, dg, db, destination_alpha);
                for source_alpha in 0..=255 {
                    let source = Color::rgba(r, g, b, source_alpha);
                    for opacity in 0..=255 {
                        assert_eq!(
                            source.over(destination, opacity),
                            reference_over(source, destination, opacity),
                            "source={source:?} destination={destination:?} opacity={opacity}",
                        );
                    }
                }
            }
        }
    }
    // Low-opacity source colors can round to zero alpha without zeroing RGB.
    // A later transparent layer must still canonicalize those RGB values.
    let rounded = Color::rgba(19, 87, 131, 1).over(Color::TRANSPARENT, 1);
    assert_eq!(rounded, Color::rgba(19, 87, 131, 0));
    assert_eq!(Color::TRANSPARENT.over(rounded, 255), Color::TRANSPARENT);
}
