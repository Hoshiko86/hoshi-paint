//! Private, non-zero polygon geometry for ADR 0001's selection model.
// Selection consumes this API in the next slice; keep its unused status checked.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "ADR 0001 selection integration follows this private geometry slice"
    )
)]

use crate::Error;
use crate::geometry::{Affine, Point};
use i_overlay::core::{
    fill_rule::FillRule, overlay::Overlay, overlay::ShapeType, overlay_rule::OverlayRule,
};
use i_overlay::i_float::{
    adapter::FloatPointAdapter,
    float::{number::FloatNumber, rect::FloatRect},
};
use std::f64::consts::{PI, TAU};

// i_overlay's f64 contract: absolute coordinates must not exceed 2^500.
const MAX_COORDINATE: f64 = <f64 as FloatNumber>::MAX_COORDINATE;
const CHORD_ERROR: f64 = 0.1;

/// Canonical non-zero contours in document pixels, including oppositely wound holes.
#[derive(Debug)]
pub(crate) struct PolygonSet {
    contours: Vec<Vec<(f64, f64)>>,
}

/// Set operations on the filled area of two polygon sets.
#[derive(Clone, Copy, Debug)]
pub(crate) enum BoolOp {
    Union,
    Difference,
    Intersect,
    Xor,
}

impl PolygonSet {
    /// Builds an axis-aligned rectangle; zero extents are empty, negative ones invalid.
    pub(crate) fn rect(x: f64, y: f64, w: f64, h: f64) -> Result<Self, Error> {
        if !w.is_finite() || !h.is_finite() || w < 0.0 || h < 0.0 {
            return Err(Error::InvalidShape);
        }
        Self::polygon(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)])
    }

    /// Flattens a rotated ellipse with at most 0.1px chord error; zero radii are empty.
    pub(crate) fn ellipse(
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        rotation_deg: f64,
    ) -> Result<Self, Error> {
        if ![cx, cy, rx, ry, rotation_deg]
            .into_iter()
            .all(f64::is_finite)
            || rx < 0.0
            || ry < 0.0
        {
            return Err(Error::InvalidShape);
        }
        validate_points(&[(cx, cy)])?;
        if rx == 0.0 || ry == 0.0 {
            return Ok(Self {
                contours: Vec::new(),
            });
        }
        // Linear mapping of a unit circle multiplies chord error by at most max(rx, ry).
        // The asin form avoids cancellation in acos(1 - error/r) for large radii.
        let half_step = 2.0 * (CHORD_ERROR / (2.0 * rx.max(ry))).min(1.0).sqrt().asin();
        let count = ((PI / half_step).ceil().max(4.0) as usize)
            .checked_next_multiple_of(4)
            .ok_or(Error::MemoryLimit)?;
        let bytes = count
            .checked_mul(std::mem::size_of::<(f64, f64)>())
            .ok_or(Error::MemoryLimit)?;
        if bytes as u128 > u128::from(crate::memory_budget()) {
            return Err(Error::MemoryLimit);
        }
        let mut points = Vec::new();
        points
            .try_reserve_exact(count)
            .map_err(|_| Error::MemoryLimit)?;
        let (sin, cos) = rotation_deg.rem_euclid(360.0).to_radians().sin_cos();
        for i in 0..count {
            let (s, c) = (TAU * i as f64 / count as f64).sin_cos();
            points.push((
                cx + rx * c * cos - ry * s * sin,
                cy + rx * c * sin + ry * s * cos,
            ));
        }
        Self::polygon(&points)
    }

    /// Resolves a closed lasso's self-intersections using the non-zero fill rule.
    pub(crate) fn polygon(points: &[(f64, f64)]) -> Result<Self, Error> {
        validate_points(points)?;
        Ok(Self::overlay(&[points], &[], OverlayRule::Subject))
    }

    /// Combines filled geometry without cloning either input's point arrays.
    pub(crate) fn boolean(&self, other: &Self, op: BoolOp) -> Self {
        let rule = match op {
            BoolOp::Union => OverlayRule::Union,
            BoolOp::Difference => OverlayRule::Difference,
            BoolOp::Intersect => OverlayRule::Intersect,
            BoolOp::Xor => OverlayRule::Xor,
        };
        let subject: Vec<_> = self.contours().collect();
        let clip: Vec<_> = other.contours().collect();
        Self::overlay(&subject, &clip, rule)
    }

    /// Maps each contour once, preserving winding and rejecting collapsed or unsafe geometry.
    pub(crate) fn transformed(&self, t: &Affine) -> Result<Self, Error> {
        let det = t.determinant();
        if !t.is_finite() || !det.is_finite() || det == 0.0 {
            return Err(Error::InvalidShape);
        }
        let mut contours = Vec::new();
        contours
            .try_reserve_exact(self.contours.len())
            .map_err(|_| Error::MemoryLimit)?;
        for contour in &self.contours {
            let mut points = Vec::new();
            points
                .try_reserve_exact(contour.len())
                .map_err(|_| Error::MemoryLimit)?;
            let map = |&(x, y): &(f64, f64)| {
                let p = t.apply(Point::new(x, y));
                (p.x, p.y)
            };
            if det < 0.0 {
                points.extend(contour.iter().rev().map(map));
            } else {
                points.extend(contour.iter().map(map));
            }
            validate_points(&points)?;
            contours.push(points);
        }
        Ok(Self { contours })
    }

    /// Whether no filled contours remain after normalization.
    pub(crate) fn is_empty(&self) -> bool {
        self.contours.is_empty()
    }

    /// Returns (minimum x, minimum y, maximum x, maximum y), or None for empty sets.
    pub(crate) fn bounds(&self) -> Option<(f64, f64, f64, f64)> {
        point_bounds(self.contours.iter().flatten())
    }

    /// Borrows closed contours; the closing vertex need not be repeated.
    pub(crate) fn contours(&self) -> impl Iterator<Item = &[(f64, f64)]> {
        self.contours.iter().map(Vec::as_slice)
    }

    /// Non-zero winding membership, including boundaries; non-finite queries are outside.
    pub(crate) fn contains(&self, x: f64, y: f64) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        let Some((left, top, right, bottom)) = self.bounds() else {
            return false;
        };
        if x < left || x > right || y < top || y > bottom {
            return false;
        }
        let mut winding = 0_i64;
        for contour in &self.contours {
            for (&a, &b) in contour
                .iter()
                .zip(contour.iter().cycle().skip(1))
                .take(contour.len())
            {
                let cross = (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
                if cross == 0.0
                    && x >= a.0.min(b.0)
                    && x <= a.0.max(b.0)
                    && y >= a.1.min(b.1)
                    && y <= a.1.max(b.1)
                {
                    return true;
                }
                if a.1 <= y && b.1 > y && cross > 0.0 {
                    winding += 1;
                } else if a.1 > y && b.1 <= y && cross < 0.0 {
                    winding -= 1;
                }
            }
        }
        winding != 0
    }

    fn overlay(subject: &[&[(f64, f64)]], clip: &[&[(f64, f64)]], rule: OverlayRule) -> Self {
        let all = || subject.iter().chain(clip).flat_map(|path| path.iter());
        let Some((min_x, min_y, max_x, max_y)) = point_bounds(all()) else {
            return Self {
                contours: Vec::new(),
            };
        };
        // Constructors enforce the dependency's coordinate contract. i64 preserves
        // subpixel detail on large documents; input tuples are streamed, not cloned.
        let adapter = FloatPointAdapter::<[f64; 2], i64>::new_conservative(FloatRect {
            min_x,
            max_x,
            min_y,
            max_y,
        });
        let mut overlay = Overlay::<i64>::new(all().count());
        for (paths, kind) in [(subject, ShapeType::Subject), (clip, ShapeType::Clip)] {
            for path in paths {
                overlay.add_path_iter(
                    path.iter().map(|&(x, y)| adapter.float_to_int(&[x, y])),
                    kind,
                );
            }
        }
        let contours = overlay
            .overlay(rule, FillRule::NonZero)
            .into_iter()
            .flatten()
            .map(|path| {
                path.into_iter()
                    .map(|point| {
                        let [x, y] = adapter.int_to_float(&point);
                        (x, y)
                    })
                    .collect()
            })
            .collect();
        Self { contours }
    }
}

fn validate_points(points: &[(f64, f64)]) -> Result<(), Error> {
    if points.iter().all(|&(x, y)| {
        x.is_finite() && y.is_finite() && x.abs() <= MAX_COORDINATE && y.abs() <= MAX_COORDINATE
    }) {
        Ok(())
    } else {
        Err(Error::InvalidShape)
    }
}

fn point_bounds<'a>(
    mut points: impl Iterator<Item = &'a (f64, f64)>,
) -> Option<(f64, f64, f64, f64)> {
    let &(x, y) = points.next()?;
    Some(
        points.fold((x, y, x, y), |(left, top, right, bottom), &(x, y)| {
            (left.min(x), top.min(y), right.max(x), bottom.max(y))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(set: &PolygonSet) -> f64 {
        (set.contours()
            .map(|path| {
                path.iter()
                    .zip(path.iter().cycle().skip(1))
                    .take(path.len())
                    .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
                    .sum::<f64>()
            })
            .sum::<f64>()
            * 0.5)
            .abs()
    }

    #[test]
    fn rectangle_union_area_subtracts_overlap() {
        let a = PolygonSet::rect(0.0, 0.0, 4.0, 3.0).unwrap();
        let b = PolygonSet::rect(2.0, 1.0, 4.0, 3.0).unwrap();
        let union = a.boolean(&b, BoolOp::Union);
        assert!((area(&union) - (12.0 + 12.0 - 4.0)).abs() < 1e-9);
        assert_eq!(union.bounds(), Some((0.0, 0.0, 6.0, 4.0)));
    }

    #[test]
    fn difference_preserves_a_hole_and_non_zero_membership() {
        let outer = PolygonSet::rect(0.0, 0.0, 6.0, 6.0).unwrap();
        let inner = PolygonSet::rect(2.0, 2.0, 2.0, 2.0).unwrap();
        let result = outer.boolean(&inner, BoolOp::Difference);
        assert_eq!(result.contours().count(), 2);
        assert!(result.contains(0.5, 0.5));
        assert!(!result.contains(3.0, 3.0));
        assert!((area(&result) - 32.0).abs() < 1e-9);
    }

    #[test]
    fn disjoint_intersection_and_identical_xor_are_empty() {
        let a = PolygonSet::rect(0.0, 0.0, 2.0, 2.0).unwrap();
        let b = PolygonSet::rect(5.0, 5.0, 2.0, 2.0).unwrap();
        let empty = a.boolean(&b, BoolOp::Intersect);
        assert!(empty.is_empty());
        assert_eq!(empty.bounds(), None);
        assert!(a.boolean(&a, BoolOp::Xor).is_empty());
        assert!(!empty.contains(0.0, 0.0));
    }

    #[test]
    fn self_intersecting_lasso_retains_both_lobes() {
        let bowtie =
            PolygonSet::polygon(&[(0.0, 0.0), (4.0, 4.0), (0.0, 4.0), (4.0, 0.0)]).unwrap();
        assert!(bowtie.contains(2.0, 1.0));
        assert!(bowtie.contains(2.0, 3.0));
        assert!(!bowtie.contains(0.5, 2.0));
    }

    #[test]
    fn twice_wound_lasso_uses_non_zero_not_even_odd_fill() {
        let twice = PolygonSet::polygon(&[
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (0.0, 4.0),
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (0.0, 4.0),
        ])
        .unwrap();
        assert!(twice.contains(2.0, 2.0));
        assert!((area(&twice) - 16.0).abs() < 1e-9);
    }

    fn ellipse_point(cx: f64, cy: f64, rx: f64, ry: f64, rotation: f64, angle: f64) -> (f64, f64) {
        let (s, c) = angle.sin_cos();
        let (sin, cos) = rotation.to_radians().sin_cos();
        (
            cx + rx * c * cos - ry * s * sin,
            cy + rx * c * sin + ry * s * cos,
        )
    }

    fn edge_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = dx * dx + dy * dy;
        let t = if length == 0.0 {
            0.0
        } else {
            (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0)
        };
        (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
    }

    #[test]
    fn rotated_eccentric_and_large_ellipses_meet_chord_error_bound() {
        for (cx, cy, rx, ry, rotation) in [
            (10.0, 15.0, 20.0, 7.0, 31.0),
            (0.0, 0.0, 100.0, 100.0, 0.0),
            (200.0, 500.0, 262_144.0, 2.0, 73.0),
        ] {
            let ellipse = PolygonSet::ellipse(cx, cy, rx, ry, rotation).unwrap();
            assert!(ellipse.contains(cx, cy));
            // Includes the required eight angles, plus dense samples between them.
            for i in 0..1024 {
                let p = ellipse_point(cx, cy, rx, ry, rotation, TAU * i as f64 / 1024.0);
                let nearest = ellipse
                    .contours()
                    .flat_map(|path| {
                        path.iter()
                            .zip(path.iter().cycle().skip(1))
                            .take(path.len())
                    })
                    .map(|(&a, &b)| edge_distance(p, a, b))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    nearest <= CHORD_ERROR + 1e-9,
                    "error {nearest} at angle sample {i}"
                );
            }
        }
    }

    #[test]
    fn boolean_membership_matches_a_pointwise_oracle() {
        let a = PolygonSet::rect(0.0, 0.0, 5.0, 5.0).unwrap().boolean(
            &PolygonSet::rect(1.0, 1.0, 2.0, 2.0).unwrap(),
            BoolOp::Difference,
        );
        let b = PolygonSet::rect(2.0, -1.0, 5.0, 5.0).unwrap();
        for op in [
            BoolOp::Union,
            BoolOp::Difference,
            BoolOp::Intersect,
            BoolOp::Xor,
        ] {
            let result = a.boolean(&b, op);
            for x in -2..9 {
                for y in -2..7 {
                    let (x, y) = (f64::from(x) + 0.37, f64::from(y) + 0.37);
                    let (inside_a, inside_b) = (a.contains(x, y), b.contains(x, y));
                    let expected = match op {
                        BoolOp::Union => inside_a || inside_b,
                        BoolOp::Difference => inside_a && !inside_b,
                        BoolOp::Intersect => inside_a && inside_b,
                        BoolOp::Xor => inside_a ^ inside_b,
                    };
                    assert_eq!(result.contains(x, y), expected, "{op:?} at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn invalid_inputs_and_queries_are_rejected() {
        assert!(matches!(
            PolygonSet::polygon(&[(0.0, 0.0), (f64::NAN, 1.0)]),
            Err(Error::InvalidShape)
        ));
        assert!(matches!(
            PolygonSet::rect(f64::INFINITY, 0.0, 1.0, 1.0),
            Err(Error::InvalidShape)
        ));
        assert!(matches!(
            PolygonSet::rect(0.0, 0.0, -1.0, 1.0),
            Err(Error::InvalidShape)
        ));
        assert!(matches!(
            PolygonSet::ellipse(0.0, 0.0, 1.0, 1.0, f64::NAN),
            Err(Error::InvalidShape)
        ));
        assert!(matches!(
            PolygonSet::polygon(&[(f64::MAX, 0.0)]),
            Err(Error::InvalidShape)
        ));
        let rect = PolygonSet::rect(0.0, 0.0, 2.0, 2.0).unwrap();
        assert!(!rect.contains(f64::NAN, 0.0));
        assert!(!rect.contains(f64::INFINITY, 0.0));
        assert!(rect.contains(0.0, 1.0));
    }

    #[test]
    fn empty_degenerate_and_unallocatable_shapes_are_safe() {
        assert!(PolygonSet::polygon(&[]).unwrap().is_empty());
        assert!(
            PolygonSet::polygon(&[(0.0, 0.0), (1.0, 1.0)])
                .unwrap()
                .is_empty()
        );
        assert!(PolygonSet::rect(0.0, 0.0, 0.0, 2.0).unwrap().is_empty());
        assert!(
            PolygonSet::ellipse(0.0, 0.0, 0.0, 2.0, 0.0)
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            PolygonSet::ellipse(0.0, 0.0, 1e100, 1.0, 0.0),
            Err(Error::MemoryLimit)
        ));
        let empty = PolygonSet::polygon(&[]).unwrap();
        let rect = PolygonSet::rect(0.0, 0.0, 2.0, 2.0).unwrap();
        assert!(empty.boolean(&rect, BoolOp::Intersect).is_empty());
        assert!((area(&empty.boolean(&rect, BoolOp::Union)) - 4.0).abs() < 1e-9);
        assert!(empty.boolean(&empty, BoolOp::Union).is_empty());
    }
    #[test]
    fn affine_maps_preserve_contours_area_and_hole_winding() {
        let rect = PolygonSet::rect(0., 0., 40., 20.).unwrap();
        assert_eq!(
            rect.transformed(&Affine::IDENTITY).unwrap().contours,
            rect.contours
        );
        let rotated = rect
            .transformed(&Affine::rotate_about(Point::new(0., 0.), 37.))
            .unwrap();
        assert!((area(&rotated) - area(&rect)).abs() < 1e-9 * area(&rect));
        let hole = PolygonSet::rect(0., 0., 6., 6.).unwrap().boolean(
            &PolygonSet::rect(2., 2., 2., 2.).unwrap(),
            BoolOp::Difference,
        );
        let mirrored = hole.transformed(&Affine::scale(-1., 1.)).unwrap();
        let signed_area = |path: &[(f64, f64)]| {
            path.iter()
                .zip(path.iter().cycle().skip(1))
                .take(path.len())
                .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
                .sum::<f64>()
        };
        for (a, b) in hole.contours().zip(mirrored.contours()) {
            assert_eq!(signed_area(a).signum(), signed_area(b).signum());
        }
        for x in -2..9 {
            for y in -2..9 {
                let (x, y) = (f64::from(x) + 0.37, f64::from(y) + 0.37);
                assert_eq!(hole.contains(x, y), mirrored.contains(-x, y));
            }
        }
        assert!(
            PolygonSet::polygon(&[])
                .unwrap()
                .transformed(&Affine::IDENTITY)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn affine_maps_reject_invalid_coefficients_and_unsafe_outputs() {
        let rect = PolygonSet::rect(0., 0., 2., 2.).unwrap();
        for transform in [
            Affine::scale(f64::NAN, 1.),
            Affine::translate(0., f64::INFINITY),
            Affine::scale(0., 1.),
            Affine::scale(1e300, 1e300),
        ] {
            assert!(matches!(
                rect.transformed(&transform),
                Err(Error::InvalidShape)
            ));
        }
        // Direct fixtures exercise output validation beyond constructor-safe coordinates.
        let huge = PolygonSet {
            contours: vec![vec![(1e300, 1.)]],
        };
        assert!(matches!(
            huge.transformed(&Affine::scale(1e300, 1.)),
            Err(Error::InvalidShape)
        ));
        let large = PolygonSet {
            contours: vec![vec![(2_f64.powi(300), 1.)]],
        };
        assert!(matches!(
            large.transformed(&Affine::scale(2_f64.powi(300), 1.)),
            Err(Error::InvalidShape)
        ));
    }
}
