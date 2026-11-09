//! Pixel coverage and premultiplied sampling, shared by preview and commit.
use super::{FillPattern, LineStyle, Shape, ShapeKind, ShapeStyle};
use crate::{
    Color,
    geometry::{Frame, Point, PreparedTransform},
};
const ARC_STEPS: usize = 128;

/// Prepared once per rendering operation, shared by preview and committed pixels.
/// Only ellipse dash strokes need the cumulative arc-length lookup table.
pub(crate) struct RasterShape {
    shape: Shape,
    transform: PreparedTransform,
    arc_lengths: [f64; ARC_STEPS + 1],
}
impl RasterShape {
    pub(crate) fn new(shape: Shape) -> Self {
        let mut arc_lengths = [0.; ARC_STEPS + 1];
        if shape.kind == ShapeKind::Ellipse && shape.rendering.line_style != LineStyle::Solid {
            let (rx, ry) = (f64::from(shape.width) / 2., f64::from(shape.height) / 2.);
            let mut previous = Point::new(0., -ry);
            for (i, length) in arc_lengths.iter_mut().enumerate().skip(1) {
                let angle = i as f64 * std::f64::consts::TAU / ARC_STEPS as f64;
                let point = Point::new(rx * angle.sin(), -ry * angle.cos());
                *length = previous.distance(point);
                previous = point;
            }
            for i in 1..=ARC_STEPS {
                arc_lengths[i] += arc_lengths[i - 1];
            }
        }
        Self {
            shape,
            transform: Frame::from_shape(shape).prepare(),
            arc_lengths,
        }
    }

    fn contains(&self, p: Point, inset: f64) -> bool {
        self.shape.contains(
            p.x + f64::from(self.shape.x) + f64::from(self.shape.width) / 2.,
            p.y + f64::from(self.shape.y) + f64::from(self.shape.height) / 2.,
            inset,
        )
    }

    // Conservative distance bounds let flat interiors avoid supersampling.
    // The ellipse expression is a lower bound from the normalized radial norm's
    // Lipschitz constant; it never skips a boundary that can cross this pixel.
    fn near_boundary(&self, p: Point, inset: f64) -> bool {
        let rx = f64::from(self.shape.width) / 2. - inset;
        let ry = f64::from(self.shape.height) / 2. - inset;
        if rx <= 0. || ry <= 0. {
            return false;
        }
        let distance = match self.shape.kind {
            ShapeKind::Rectangle => (p.x.abs() - rx).abs().min((p.y.abs() - ry).abs()),
            ShapeKind::Ellipse => ((p.x / rx).hypot(p.y / ry) - 1.).abs() * rx.min(ry),
        };
        distance <= std::f64::consts::FRAC_1_SQRT_2 + 1e-9
    }

    fn outline_width(&self) -> Option<f64> {
        match self.shape.style {
            ShapeStyle::Fill(_) => None,
            ShapeStyle::Outline { width, .. } | ShapeStyle::Both { width, .. } => Some(width),
        }
    }

    fn perimeter_distance(&self, p: Point, width: f64) -> f64 {
        match self.shape.kind {
            ShapeKind::Rectangle => {
                // Project onto the closest centerline edge, running clockwise
                // from the top-left. Width is inset, preserving existing bounds.
                let (rx, ry) = (
                    (f64::from(self.shape.width) - width).max(0.) / 2.,
                    (f64::from(self.shape.height) - width).max(0.) / 2.,
                );
                let x = p.x.clamp(-rx, rx) + rx;
                let y = p.y.clamp(-ry, ry) + ry;
                let edges = [
                    (p.y + ry).abs(),
                    (p.x - rx).abs(),
                    (p.y - ry).abs(),
                    (p.x + rx).abs(),
                ];
                let edge = edges
                    .iter()
                    .enumerate()
                    .min_by(|a, b| a.1.total_cmp(b.1))
                    .unwrap()
                    .0;
                match edge {
                    0 => x,
                    1 => 2. * rx + y,
                    2 => 4. * rx + 2. * ry - x,
                    _ => 4. * rx + 4. * ry - y,
                }
            }
            ShapeKind::Ellipse => {
                let rx = f64::from(self.shape.width) / 2.;
                let ry = f64::from(self.shape.height) / 2.;
                let angle = (p.x / rx)
                    .atan2(-p.y / ry)
                    .rem_euclid(std::f64::consts::TAU);
                let position = angle * ARC_STEPS as f64 / std::f64::consts::TAU;
                let index = (position.floor() as usize).min(ARC_STEPS - 1);
                self.arc_lengths[index]
                    + (self.arc_lengths[index + 1] - self.arc_lengths[index])
                        * (position - index as f64)
            }
        }
    }

    fn fill_at(&self, p: Point, color: Color) -> Color {
        let x = p.x + f64::from(self.shape.width) / 2.;
        let y = p.y + f64::from(self.shape.height) / 2.;
        if self.shape.rendering.fill_pattern.foreground(x, y) {
            color
        } else {
            self.shape.rendering.pattern_color
        }
    }
    fn sample(&self, p: Point) -> Option<Color> {
        if !self.contains(p, 0.) {
            return None;
        }
        let stroke = |width| {
            self.shape.rendering.line_style == LineStyle::Solid
                || self
                    .shape
                    .rendering
                    .line_style
                    .painted(self.perimeter_distance(p, width), width)
        };
        match self.shape.style {
            ShapeStyle::Fill(color) => Some(self.fill_at(p, color)),
            ShapeStyle::Outline { color, width } => {
                (!self.contains(p, width) && stroke(width)).then_some(color)
            }
            ShapeStyle::Both {
                fill,
                outline,
                width,
            } => Some(if !self.contains(p, width) && stroke(width) {
                outline
            } else {
                self.fill_at(p, fill)
            }),
        }
    }

    // Exact polygon/pixel intersection keeps even sub-sample-width rectangle
    // strokes visible. The clipped polygon has at most eight vertices; fixed
    // stack buffers avoid allocating memory for each boundary pixel.
    fn rectangle_coverage(&self, x: u32, y: u32, inset: f64) -> f64 {
        let rx = f64::from(self.shape.width) / 2. - inset;
        let ry = f64::from(self.shape.height) / 2. - inset;
        if rx <= 0. || ry <= 0. {
            return 0.;
        }
        let mut input = [Point::default(); 12];
        for (point, (dx, dy)) in input
            .iter_mut()
            .zip([(0., 0.), (1., 0.), (1., 1.), (0., 1.)])
        {
            *point = self
                .transform
                .to_local(Point::new(f64::from(x) + dx, f64::from(y) + dy));
        }
        let mut count = 4;
        for (axis, sign, limit) in [(0, 1., rx), (0, -1., rx), (1, 1., ry), (1, -1., ry)] {
            if count == 0 {
                return 0.;
            }
            let coordinate = |p: Point| if axis == 0 { p.x * sign } else { p.y * sign };
            let mut output = [Point::default(); 12];
            let mut length = 0;
            let mut previous = input[count - 1];
            for &current in &input[..count] {
                let from = coordinate(previous);
                let to = coordinate(current);
                if (from <= limit) != (to <= limit) {
                    let t = (limit - from) / (to - from);
                    output[length] = Point::new(
                        previous.x + t * (current.x - previous.x),
                        previous.y + t * (current.y - previous.y),
                    );
                    length += 1;
                }
                if to <= limit {
                    output[length] = current;
                    length += 1;
                }
                previous = current;
            }
            input = output;
            count = length;
        }
        if count < 3 {
            return 0.;
        }
        let mut twice_area = 0.;
        // Translate before the shoelace sum to avoid cancellation in huge
        // document coordinates when a pixel covers only a tiny sliver.
        let origin = input[0];
        for i in 1..count - 1 {
            let a = Point::new(input[i].x - origin.x, input[i].y - origin.y);
            let b = Point::new(input[i + 1].x - origin.x, input[i + 1].y - origin.y);
            twice_area += a.x * b.y - a.y * b.x;
        }
        (twice_area.abs() / 2.).clamp(0., 1.)
    }
    fn rectangle_color_at(&self, x: u32, y: u32) -> Option<Color> {
        let outer = self.rectangle_coverage(x, y, 0.);
        let (first, first_coverage, second, second_coverage) = match self.shape.style {
            ShapeStyle::Fill(color) => (color, outer, Color::TRANSPARENT, 0.),
            ShapeStyle::Outline { color, width } => (
                color,
                (outer - self.rectangle_coverage(x, y, width)).max(0.),
                Color::TRANSPARENT,
                0.,
            ),
            ShapeStyle::Both {
                fill,
                outline,
                width,
            } => {
                let inner = self.rectangle_coverage(x, y, width);
                (fill, inner, outline, (outer - inner).max(0.))
            }
        };
        let weights = [
            f64::from(first.a) * first_coverage,
            f64::from(second.a) * second_coverage,
        ];
        let alpha = weights[0] + weights[1];
        if alpha.round() == 0. {
            return None;
        }
        let channel = |a: u8, b: u8| {
            ((f64::from(a) * weights[0] + f64::from(b) * weights[1]) / alpha).round() as u8
        };
        Some(Color::rgba(
            channel(first.r, second.r),
            channel(first.g, second.g),
            channel(first.b, second.b),
            alpha.round() as u8,
        ))
    }

    pub(crate) fn color_at(&self, x: u32, y: u32) -> Option<Color> {
        let point = Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
        let local = self.transform.to_local(point);
        if !self.shape.rendering.antialias {
            return self.sample(local);
        }
        let outer_edge = self.near_boundary(local, 0.);
        let inner_edge = self
            .outline_width()
            .is_some_and(|w| self.near_boundary(local, w));
        let inside = self.contains(local, 0.);
        if !outer_edge && !inside {
            return None;
        }
        let in_stroke = self
            .outline_width()
            .is_some_and(|w| !self.contains(local, w));
        let texture = if in_stroke {
            self.shape.rendering.line_style != LineStyle::Solid
                || (matches!(self.shape.style, ShapeStyle::Both { .. })
                    && self.shape.rendering.fill_pattern != FillPattern::Solid)
        } else {
            self.shape.rendering.fill_pattern != FillPattern::Solid
        };
        if !outer_edge && !inner_edge && !texture {
            return self.sample(local);
        }

        if self.shape.kind == ShapeKind::Rectangle
            && self.shape.rendering.line_style == LineStyle::Solid
            && self.shape.rendering.fill_pattern == FillPattern::Solid
        {
            return self.rectangle_color_at(x, y);
        }

        // A 4×4 grid, averaged in premultiplied encoded RGBA. Unpremultiplication
        // happens once, avoiding dark fringes around translucent colored edges.
        let mut alpha = 0u32;
        let mut channels = [0u32; 3];
        for sy in 0..4 {
            for sx in 0..4 {
                let sample = Point::new(
                    f64::from(x) + (f64::from(sx) + 0.5) / 4.,
                    f64::from(y) + (f64::from(sy) + 0.5) / 4.,
                );
                if let Some(color) = self.sample(self.transform.to_local(sample)) {
                    let a = u32::from(color.a);
                    alpha += a;
                    for (sum, channel) in channels.iter_mut().zip([color.r, color.g, color.b]) {
                        *sum += u32::from(channel) * a;
                    }
                }
            }
        }
        let a = (alpha + 8) / 16;
        if a == 0 {
            return None;
        }
        let [r, g, b] = channels.map(|sum| ((sum + alpha / 2) / alpha) as u8);
        Some(Color::rgba(r, g, b, a as u8))
    }
}
