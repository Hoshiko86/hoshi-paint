//! Procedural line and fill paints in shape-local coordinates.
/// Stroke patterns use lengths measured in multiples of the outline width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineStyle {
    #[default]
    Solid,
    Dash,
    Dot,
    DashDot,
    DashDotDot,
}
impl LineStyle {
    fn segments(self) -> &'static [f64] {
        match self {
            Self::Solid => &[],
            Self::Dash => &[3., 1.],
            Self::Dot => &[1., 1.],
            Self::DashDot => &[3., 1., 1., 1.],
            Self::DashDotDot => &[3., 1., 1., 1., 1., 1.],
        }
    }
    pub(super) fn painted(self, distance: f64, width: f64) -> bool {
        let segments = self.segments();
        if segments.is_empty() {
            return true;
        }
        let period: f64 = segments.iter().sum();
        let mut phase = (distance / width).rem_euclid(period);
        for (index, &length) in segments.iter().enumerate() {
            if phase < length {
                return index % 2 == 0;
            }
            phase -= length;
        }
        true
    }
}

/// A small procedural palette of two-color hatches, sampled in shape-local space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillPattern {
    #[default]
    Solid,
    Horizontal,
    Vertical,
    ForwardDiagonal,
    BackwardDiagonal,
    Cross,
    DiagonalCross,
    Checkerboard,
}
impl FillPattern {
    pub(super) fn foreground(self, x: f64, y: f64) -> bool {
        let x = x.floor() as i64;
        let y = y.floor() as i64;
        let horizontal = y.rem_euclid(8) == 0;
        let vertical = x.rem_euclid(8) == 0;
        let forward = (x + y).rem_euclid(8) == 0;
        let backward = (x - y).rem_euclid(8) == 0;
        match self {
            Self::Solid => true,
            Self::Horizontal => horizontal,
            Self::Vertical => vertical,
            Self::ForwardDiagonal => forward,
            Self::BackwardDiagonal => backward,
            Self::Cross => horizontal || vertical,
            Self::DiagonalCross => forward || backward,
            Self::Checkerboard => (x.div_euclid(4) + y.div_euclid(4)).rem_euclid(2) == 0,
        }
    }
}
