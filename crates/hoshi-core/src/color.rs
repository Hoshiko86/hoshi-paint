//! RGBA8 colors and straight-alpha compositing.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Straight-alpha source-over in encoded RGBA8 space, rounded to nearest.
    pub(crate) fn over(self, destination: Self, opacity: u8) -> Self {
        // Keep effective source alpha at full precision until the final channels
        // are rounded. Quantizing it to u8 first loses low-opacity colors.
        let alpha = u64::from(self.a) * u64::from(opacity);
        if alpha == 0 {
            // Preserve canonical transparent output even when the destination
            // carries RGB under zero alpha (including rounded low opacity).
            return if destination.a == 0 {
                Self::TRANSPARENT
            } else {
                destination
            };
        }
        if alpha == 65_025 {
            return self;
        }
        let back = u64::from(destination.a) * (65_025 - alpha);
        let denominator = alpha * 255 + back;
        if denominator == 0 {
            return Self::TRANSPARENT;
        }
        let channel = |source: u8, target: u8| {
            ((u64::from(source) * alpha * 255 + u64::from(target) * back + denominator / 2)
                / denominator) as u8
        };
        Self::rgba(
            channel(self.r, destination.r),
            channel(self.g, destination.g),
            channel(self.b, destination.b),
            ((denominator + 32_512) / 65_025) as u8,
        )
    }
}
