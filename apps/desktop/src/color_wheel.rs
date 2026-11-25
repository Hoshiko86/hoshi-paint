//! Hue increases counter-clockwise from red at the right; radius is saturation.
use hoshi_controller::color;
use slint::{Rgba8Pixel, SharedPixelBuffer};

/// Maps square-local coordinates to hue in degrees and saturation in percent, clamped to the rim.
pub(crate) fn hue_saturation_at(x: f64, y: f64, size: f64) -> (f64, f64) {
    if !x.is_finite() || !y.is_finite() || !size.is_finite() || size <= 0.0 {
        return (0.0, 0.0);
    }
    let radius = size / 2.0;
    let dx = x - radius;
    let dy = radius - y;
    (
        dy.atan2(dx).to_degrees().rem_euclid(360.0),
        (dx.hypot(dy) / radius).min(1.0) * 100.0,
    )
}

/// Returns the square-local marker center for a hue and saturation clamped to the rim.
pub(crate) fn marker(hue: f64, saturation: f64, size: f64) -> (f64, f64) {
    let radius = if size.is_finite() {
        size.max(0.0) / 2.0
    } else {
        0.0
    };
    let hue = if hue.is_finite() {
        hue.rem_euclid(360.0)
    } else {
        0.0
    }
    .to_radians();
    let saturation = if saturation.is_finite() {
        saturation.clamp(0.0, 100.0) / 100.0
    } else {
        0.0
    };
    (
        radius + radius * saturation * hue.cos(),
        radius - radius * saturation * hue.sin(),
    )
}

/// Renders a circular HSV wheel with transparent corners and a one-pixel antialiased rim.
pub(crate) fn render(size: u32, value: f64) -> SharedPixelBuffer<Rgba8Pixel> {
    let mut pixels = SharedPixelBuffer::new(size, size);
    let diameter = f64::from(size.saturating_sub(1));
    let radius = diameter / 2.0;
    let value = if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    };
    for (index, pixel) in pixels.make_mut_slice().iter_mut().enumerate() {
        let x = (index % size as usize) as f64;
        let y = (index / size as usize) as f64;
        // Pixel centers span 0..size-1; integrate coverage across the one-pixel rim.
        let coverage = (radius + 0.5 - (x - radius).hypot(y - radius)).clamp(0.0, 1.0);
        let (hue, saturation) = hue_saturation_at(x, y, diameter);
        if let Ok(color) =
            color::color_from_hsv(hue, saturation, value, (coverage * 255.0).round() as u8)
        {
            *pixel = Rgba8Pixel {
                r: color.r,
                g: color.g,
                b: color.b,
                a: color.a,
            };
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_cardinal_rim_and_outside_points_follow_the_wheel_convention() {
        assert_eq!(hue_saturation_at(50.0, 50.0, 100.0), (0.0, 0.0));
        for (x, y, hue) in [
            (100.0, 50.0, 0.0),
            (50.0, 0.0, 90.0),
            (0.0, 50.0, 180.0),
            (50.0, 100.0, 270.0),
        ] {
            assert_eq!(hue_saturation_at(x, y, 100.0), (hue, 100.0));
            assert_eq!(
                hue_saturation_at(50.0 + (x - 50.0) * 3.0, 50.0 + (y - 50.0) * 3.0, 100.0),
                (hue, 100.0)
            );
        }
    }

    #[test]
    fn marker_round_trips_across_hues_saturations_and_sizes() {
        for size in [1.0, 37.5, 180.0, 256.0] {
            for hue in [0.0, 17.0, 90.0, 180.0, 270.0, 359.0] {
                for saturation in [0.5, 20.0, 70.0, 100.0] {
                    let (x, y) = marker(hue, saturation, size);
                    let (h, s) = hue_saturation_at(x, y, size);
                    let error = (h - hue).abs();
                    assert!(error.min(360.0 - error) < 0.5);
                    assert!((s - saturation).abs() < 0.5);
                }
            }
        }
        assert_eq!(marker(90.0, 0.0, 100.0), (50.0, 50.0));
        assert_eq!(marker(360.0, 200.0, 100.0), (100.0, 50.0));
        assert_eq!(marker(0.0, -1.0, 100.0), (50.0, 50.0));
    }

    #[test]
    fn render_has_transparent_corners_red_rim_and_black_at_zero_value() {
        let image = render(65, 100.0);
        let pixels = image.as_slice();
        for index in [0, 64, 64 * 65, 65 * 65 - 1] {
            assert_eq!(pixels[index].a, 0);
        }
        let red = pixels[32 * 65 + 64];
        assert_eq!((red.r, red.g, red.b), (255, 0, 0));
        assert!(red.a > 0 && red.a < 255);
        assert_eq!(pixels[32 * 65 + 32].a, 255);
        assert!(
            render(65, 0.0)
                .as_slice()
                .iter()
                .all(|p| (p.r, p.g, p.b) == (0, 0, 0))
        );
        assert!(render(0, 100.0).as_slice().is_empty());
        assert_eq!(render(1, 100.0).as_slice().len(), 1);
    }

    #[test]
    fn invalid_geometry_is_safe_and_non_finite_value_renders_black() {
        for size in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(hue_saturation_at(10.0, 10.0, size), (0.0, 0.0));
            assert_eq!(marker(f64::NAN, f64::NAN, size), (0.0, 0.0));
        }
        assert_eq!(hue_saturation_at(f64::NAN, 0.0, 100.0), (0.0, 0.0));
        assert_eq!(hue_saturation_at(0.0, f64::INFINITY, 100.0), (0.0, 0.0));
        assert_eq!(marker(f64::NAN, f64::NAN, 100.0), (50.0, 50.0));
        assert!(
            render(3, f64::NAN)
                .as_slice()
                .iter()
                .all(|p| p.r == 0 && p.g == 0 && p.b == 0)
        );
    }
}
