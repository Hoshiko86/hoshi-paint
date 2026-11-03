//! Color entry conversions. HSV uses degrees and percentages; alpha is independent.
use crate::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub hue: f64,
    pub saturation: f64,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorInputError {
    InvalidHsv,
    InvalidHex,
}

pub fn color_to_hsv(color: Color) -> Hsv {
    let (r, g, b) = (
        f64::from(color.r) / 255.0,
        f64::from(color.g) / 255.0,
        f64::from(color.b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        (60.0 * ((g - b) / delta)).rem_euclid(360.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    Hsv {
        hue,
        saturation: if max == 0.0 { 0.0 } else { delta / max * 100.0 },
        value: max * 100.0,
    }
}

/// Hue 360 is equivalent to 0. Inputs outside the documented ranges are rejected.
pub fn color_from_hsv(
    hue: f64,
    saturation: f64,
    value: f64,
    alpha: u8,
) -> Result<Color, ColorInputError> {
    if !hue.is_finite()
        || !saturation.is_finite()
        || !value.is_finite()
        || !(0.0..=360.0).contains(&hue)
        || !(0.0..=100.0).contains(&saturation)
        || !(0.0..=100.0).contains(&value)
    {
        return Err(ColorInputError::InvalidHsv);
    }
    let h = hue.rem_euclid(360.0) / 60.0;
    let v = value / 100.0;
    let c = v * saturation / 100.0;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u8 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let channel = |n: f64| ((n + v - c) * 255.0).round() as u8;
    Ok(Color::rgba(channel(r), channel(g), channel(b), alpha))
}

/// Strict CSS-style hex: #RGB, #RGBA, #RRGGBB, or #RRGGBBAA.
pub fn parse_hex(input: &str) -> Result<Color, ColorInputError> {
    let digits = input.strip_prefix('#').ok_or(ColorInputError::InvalidHex)?;
    if !matches!(digits.len(), 3 | 4 | 6 | 8) || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ColorInputError::InvalidHex);
    }
    let mut channels = [255; 4];
    let short = digits.len() <= 4;
    let count = if short {
        digits.len()
    } else {
        digits.len() / 2
    };
    for (i, channel) in channels.iter_mut().enumerate().take(count) {
        let start = if short { i } else { i * 2 };
        let end = start + if short { 1 } else { 2 };
        *channel =
            u8::from_str_radix(&digits[start..end], 16).map_err(|_| ColorInputError::InvalidHex)?;
        if short {
            *channel *= 17;
        }
    }
    Ok(Color::rgba(
        channels[0],
        channels[1],
        channels[2],
        channels[3],
    ))
}

/// Canonical uppercase #RRGGBBAA, including alpha even when opaque.
pub fn format_hex(color: Color) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color.r, color.g, color.b, color.a
    )
}
