//! Editable swatches and Paint.NET palette text interchange.
//!
//! The built-in swatches are Hoshi Paint's own selection, not a reproduction of
//! Paint.NET's default RGB values. Text interchange follows Paint.NET's 96-entry
//! AARRGGBB format: <https://www.getpaint.net/doc/latest/WorkingWithPalettes.html>.
use crate::Color;

pub const PALETTE_SIZE: usize = 96;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    colors: [Color; PALETTE_SIZE],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteError {
    /// One-based source line containing an invalid retained color.
    InvalidColor {
        line: usize,
    },
    InvalidIndex {
        index: usize,
    },
}

impl std::fmt::Display for PaletteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidColor { line } => write!(
                formatter,
                "Palette line {line} must contain eight hexadecimal ARGB digits."
            ),
            Self::InvalidIndex { index } => write!(
                formatter,
                "Palette index {index} is out of range (expected 0–{}).",
                PALETTE_SIZE - 1
            ),
        }
    }
}
impl std::error::Error for PaletteError {}

impl Default for Palette {
    fn default() -> Self {
        let common = [
            (0, 0, 0),
            (255, 255, 255),
            (128, 128, 128),
            (192, 192, 192),
            (128, 0, 0),
            (255, 0, 0),
            (128, 128, 0),
            (255, 255, 0),
            (0, 128, 0),
            (0, 255, 0),
            (0, 128, 128),
            (0, 255, 255),
            (0, 0, 128),
            (0, 0, 255),
            (128, 0, 128),
            (255, 0, 255),
            (255, 128, 0),
            (255, 192, 128),
            (165, 42, 42),
            (210, 180, 140),
            (255, 192, 203),
            (220, 20, 60),
            (255, 127, 80),
            (255, 215, 0),
            (128, 128, 32),
            (54, 167, 140),
            (64, 224, 208),
            (72, 136, 240),
            (135, 206, 235),
            (75, 0, 130),
            (148, 0, 211),
            (230, 230, 250),
        ];
        let mut colors = [Color::rgba(255, 255, 255, 255); PALETTE_SIZE];
        for (slot, (r, g, b)) in colors.iter_mut().zip(common) {
            *slot = Color::rgba(r, g, b, 255);
        }
        // Each hue has four shades followed by four progressively lighter tints.
        let hues = [
            (232, 72, 72),
            (240, 144, 48),
            (224, 192, 32),
            (72, 176, 80),
            (40, 176, 176),
            (64, 112, 224),
            (136, 80, 208),
            (208, 72, 160),
        ];
        for (hue, (r, g, b)) in hues.into_iter().enumerate() {
            for step in 0..8 {
                let channel = |value: u16| -> u8 {
                    if step < 4 {
                        (value * (step as u16 + 1) / 4) as u8
                    } else {
                        (value + (255 - value) * (step as u16 - 3) / 5) as u8
                    }
                };
                colors[32 + hue * 8 + step] = Color::rgba(channel(r), channel(g), channel(b), 255);
            }
        }
        Self { colors }
    }
}

impl Palette {
    pub fn colors(&self) -> &[Color; PALETTE_SIZE] {
        &self.colors
    }

    pub fn replace(&mut self, index: usize, color: Color) -> Result<(), PaletteError> {
        let slot = self
            .colors
            .get_mut(index)
            .ok_or(PaletteError::InvalidIndex { index })?;
        *slot = color;
        Ok(())
    }

    pub fn parse_paint_net(text: &str) -> Result<Self, PaletteError> {
        parse_paint_net(text)
    }

    /// Serialize every swatch, including alpha, as uppercase AARRGGBB.
    pub fn to_paint_net(&self) -> String {
        use std::fmt::Write;
        let mut text = String::from("; Hoshi Paint palette — Paint.NET compatible\n");
        for color in &self.colors {
            writeln!(
                text,
                "{:02X}{:02X}{:02X}{:02X}",
                color.a, color.r, color.g, color.b
            )
            .expect("writing to a String cannot fail");
        }
        text
    }
}

/// Parse into a new palette, so a failed import cannot partially change swatches.
/// Missing colors are opaque white. Entries after the first 96 are ignored.
pub fn parse_paint_net(text: &str) -> Result<Palette, PaletteError> {
    let mut colors = [Color::rgba(255, 255, 255, 255); PALETTE_SIZE];
    let mut next = 0;
    for (line_index, line) in text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .lines()
        .enumerate()
    {
        if next == PALETTE_SIZE {
            break;
        }
        let entry = line.split(';').next().unwrap_or("").trim();
        if entry.is_empty() {
            continue;
        }
        if entry.len() != 8 || !entry.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(PaletteError::InvalidColor {
                line: line_index + 1,
            });
        }
        // ASCII validation above makes these byte ranges valid UTF-8 boundaries.
        let channel =
            |start| u8::from_str_radix(&entry[start..start + 2], 16).expect("validated hex");
        colors[next] = Color::rgba(channel(2), channel(4), channel(6), channel(0));
        next += 1;
    }
    Ok(Palette { colors })
}
