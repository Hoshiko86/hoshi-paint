//! Canonical, canvas-clipped selection geometry and scanline coverage.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "ADR 0001 slice 4 wires Selection into State")
)]

use crate::geometry::{Affine, Frame};
use crate::mask::MaskStore;
use crate::polygon::{BoolOp, PolygonSet};
use crate::{Error, MAX_DIMENSION, MemoryReservation};

/// Boolean operation applied to the previous selection and a new shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CombineMode {
    Replace,
    Union,
    Exclude,
    Intersect,
    Xor,
}

/// Pixel-center or sixteen-sample coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipQuality {
    Aliased,
    Antialiased,
}

/// Borrowed selection input in document pixels; lassos close implicitly.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SelectionShape<'a> {
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Ellipse(Frame),
    Lasso(&'a [(f64, f64)]),
}

impl SelectionShape<'_> {
    fn geometry(self) -> Result<PolygonSet, Error> {
        match self {
            Self::Rect {
                x,
                y,
                width,
                height,
            } => PolygonSet::rect(x, y, width, height),
            Self::Ellipse(frame) => PolygonSet::ellipse(
                frame.center.x,
                frame.center.y,
                frame.width / 2.0,
                frame.height / 2.0,
                frame.angle_degrees,
            ),
            Self::Lasso(points) => PolygonSet::polygon(points),
        }
    }
}

/// Nonempty, budgeted geometry with exclusive integer bounds inside its canvas.
#[derive(Debug)]
pub(crate) struct Selection {
    geometry: PolygonSet,
    bounds: (u32, u32, u32, u32),
    canvas: (u32, u32),
    _reservation: MemoryReservation,
}

/// An oriented non-horizontal contour edge and its active vertical interval.
struct EdgeEntry {
    a: (f64, f64),
    b: (f64, f64),
    min_y: f64,
    max_y: f64,
}

type ActiveEntry<'a> = &'a EdgeEntry;

impl Selection {
    /// Returns a new selection without mutating the borrowed previous value on failure.
    pub(crate) fn combine(
        prev: Option<&Selection>,
        shape: SelectionShape<'_>,
        mode: CombineMode,
        canvas: (u32, u32),
    ) -> Result<Option<Self>, Error> {
        if canvas.0 == 0 || canvas.1 == 0 || canvas.0 > MAX_DIMENSION || canvas.1 > MAX_DIMENSION {
            return Err(Error::InvalidShape);
        }
        if prev.is_some_and(|prev| prev.canvas != canvas) {
            return Err(Error::InvalidShape);
        }
        // i_overlay's transient allocations are not charged to the memory budget.
        let clipped = shape.geometry()?.boolean(
            &PolygonSet::rect(0.0, 0.0, f64::from(canvas.0), f64::from(canvas.1))?,
            BoolOp::Intersect,
        );
        let geometry = match (mode, prev) {
            (CombineMode::Replace, _) => clipped,
            (CombineMode::Union | CombineMode::Xor, None) => clipped,
            // ADR 0001 open question: verify Paint.NET's no-prior-selection behavior.
            (CombineMode::Exclude | CombineMode::Intersect, None) => return Ok(None),
            (CombineMode::Union, Some(prev)) => prev.geometry.boolean(&clipped, BoolOp::Union),
            (CombineMode::Exclude, Some(prev)) => {
                prev.geometry.boolean(&clipped, BoolOp::Difference)
            }
            (CombineMode::Intersect, Some(prev)) => {
                prev.geometry.boolean(&clipped, BoolOp::Intersect)
            }
            (CombineMode::Xor, Some(prev)) => prev.geometry.boolean(&clipped, BoolOp::Xor),
        };
        Self::from_geometry(geometry, canvas)
    }

    /// Transforms borrowed geometry and clips it to the original canvas.
    pub(crate) fn transformed(&self, t: &Affine) -> Result<Option<Self>, Error> {
        let mut geometry = self.geometry.transformed(t)?;
        let (w, h) = (f64::from(self.canvas.0), f64::from(self.canvas.1));
        // ADR 0001 open question: Move Selection may retain off-canvas geometry in
        // Paint.NET. Floating always derives from its origin, so clipping is safe here.
        if geometry
            .bounds()
            .is_some_and(|(l, t, r, b)| l < 0.0 || t < 0.0 || r > w || b > h)
        {
            geometry = geometry.boolean(&PolygonSet::rect(0.0, 0.0, w, h)?, BoolOp::Intersect);
        }
        Self::from_geometry(geometry, self.canvas)
    }

    fn from_geometry(geometry: PolygonSet, canvas: (u32, u32)) -> Result<Option<Self>, Error> {
        if geometry.is_empty() {
            return Ok(None);
        }
        let Some((left, top, right, bottom)) = geometry.bounds() else {
            return Ok(None);
        };
        let bytes = geometry.contours().try_fold(0_usize, |bytes, contour| {
            contour
                .len()
                .checked_mul(std::mem::size_of::<(f64, f64)>())
                .and_then(|count| bytes.checked_add(count))
                .ok_or(Error::MemoryLimit)
        })?;
        let reservation =
            MemoryReservation::new(u64::try_from(bytes).map_err(|_| Error::MemoryLimit)?)?;
        Ok(Some(Self {
            geometry,
            bounds: (
                left.floor().clamp(0.0, f64::from(canvas.0)) as u32,
                top.floor().clamp(0.0, f64::from(canvas.1)) as u32,
                right.ceil().clamp(0.0, f64::from(canvas.0)) as u32,
                bottom.ceil().clamp(0.0, f64::from(canvas.1)) as u32,
            ),
            canvas,
            _reservation: reservation,
        }))
    }

    /// Returns canvas-clipped bounds with exclusive right and bottom edges.
    pub(crate) fn bounds(&self) -> (u32, u32, u32, u32) {
        self.bounds
    }

    /// Tests non-zero membership, including the geometry's boundary.
    pub(crate) fn contains(&self, x: f64, y: f64) -> bool {
        self.geometry.contains(x, y)
    }

    /// Borrows normalized contours, including oppositely wound holes.
    pub(crate) fn contours(&self) -> impl Iterator<Item = &[(f64, f64)]> {
        self.geometry.contours()
    }

    /// Builds fresh sparse coverage; failures discard the partially constructed mask.
    pub(crate) fn mask(&self, q: ClipQuality) -> Result<MaskStore, Error> {
        let mut mask = MaskStore::new(self.canvas.0, self.canvas.1, 0);
        let (left, top, right, bottom) = self.bounds;
        let samples = match q {
            ClipQuality::Aliased => 1,
            ClipQuality::Antialiased => 4,
        };
        let edges = self.edges().filter(|(a, b)| a.1 != b.1).count();
        let width = (right - left) as usize;
        let bytes = [
            std::mem::size_of::<EdgeEntry>(),
            std::mem::size_of::<ActiveEntry<'_>>(),
            std::mem::size_of::<(f64, i64)>(),
        ]
        .into_iter()
        .try_fold(width, |bytes, size| {
            edges
                .checked_mul(size)
                .and_then(|size| bytes.checked_add(size))
                .ok_or(Error::MemoryLimit)
        })?;
        let _scratch =
            MemoryReservation::new(u64::try_from(bytes).map_err(|_| Error::MemoryLimit)?)?;
        let table = self.edge_table(edges)?;
        let mut pending = table.as_slice();
        let mut active = scratch_vec(edges)?;
        let mut crossings = scratch_vec(edges)?;
        let mut counts = scratch_vec(width)?;
        counts.resize(width, 0_u8);
        for y in top..bottom {
            counts.fill(0);
            for sy in 0..samples {
                Self::crossings(
                    f64::from(y) + (f64::from(sy) + 0.5) / f64::from(samples),
                    &mut pending,
                    &mut active,
                    &mut crossings,
                );
                count_spans(&crossings, samples, left, right, &mut counts);
            }
            for (x, &count) in (left..right).zip(&counts) {
                if count != 0 {
                    let total = samples * samples;
                    mask.set_max(x, y, ((u32::from(count) * 255 + total / 2) / total) as u8)?;
                }
            }
        }
        Ok(mask)
    }

    /// Iterates every closed contour edge in its original traversal direction.
    fn edges(&self) -> impl Iterator<Item = ((f64, f64), (f64, f64))> {
        self.contours().flat_map(|contour| {
            contour
                .iter()
                .copied()
                .zip(contour.iter().copied().cycle().skip(1))
                .take(contour.len())
        })
    }

    /// Reserves non-horizontal edges once and sorts them by their minimum height.
    fn edge_table(&self, capacity: usize) -> Result<Vec<EdgeEntry>, Error> {
        let mut table = scratch_vec(capacity)?;
        for (a, b) in self.edges().filter(|(a, b)| a.1 != b.1) {
            table.push(EdgeEntry {
                a,
                b,
                min_y: a.1.min(b.1),
                max_y: a.1.max(b.1),
            });
        }
        table.sort_unstable_by(|l, r| l.min_y.total_cmp(&r.min_y));
        Ok(table)
    }

    fn crossings<'a>(
        y: f64,
        pending: &mut &'a [EdgeEntry],
        active: &mut Vec<ActiveEntry<'a>>,
        crossings: &mut Vec<(f64, i64)>,
    ) {
        while let Some((edge, rest)) = pending.split_first() {
            if edge.min_y > y {
                break;
            }
            active.push(edge);
            *pending = rest;
        }
        active.retain(|edge| edge.max_y > y);
        crossings.clear();
        for edge in active {
            let (a, b) = (edge.a, edge.b);
            crossings.push((
                a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1),
                if b.1 > a.1 { 1 } else { -1 },
            ));
        }
        crossings.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    }
}

/// Allocates an empty scratch buffer with a fallible exact reservation.
fn scratch_vec<T>(capacity: usize) -> Result<Vec<T>, Error> {
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(capacity)
        .map_err(|_| Error::MemoryLimit)?;
    Ok(buffer)
}

fn count_spans(crossings: &[(f64, i64)], samples: u32, left: u32, right: u32, counts: &mut [u8]) {
    let mut winding = 0;
    let mut start = 0.0;
    for &(end, sign) in crossings {
        if winding != 0 {
            let first = (start * f64::from(samples) - 0.5)
                .ceil()
                .max(f64::from(left) * f64::from(samples)) as u64;
            let last = (end * f64::from(samples) - 0.5)
                .ceil()
                .min(f64::from(right) * f64::from(samples)) as u64;
            for sample in first..last {
                if let Some(count) =
                    counts.get_mut((sample / u64::from(samples) - u64::from(left)) as usize)
                {
                    *count += 1;
                }
            }
        }
        winding += sign;
        start = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    const CANVAS: (u32, u32) = (16, 16);
    const MODES: [CombineMode; 5] = [
        CombineMode::Replace,
        CombineMode::Union,
        CombineMode::Exclude,
        CombineMode::Intersect,
        CombineMode::Xor,
    ];

    fn rect(x: f64, y: f64, width: f64, height: f64) -> SelectionShape<'static> {
        SelectionShape::Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn select(shape: SelectionShape<'_>) -> Selection {
        Selection::combine(None, shape, CombineMode::Replace, CANVAS)
            .unwrap()
            .unwrap()
    }

    fn ellipse() -> Frame {
        Frame {
            center: Point::new(8.0, 8.0),
            width: 11.0,
            height: 5.0,
            angle_degrees: 30.0,
            flip_horizontal: false,
            flip_vertical: false,
        }
    }

    fn on_edge(selection: &Selection, x: f64, y: f64) -> bool {
        selection.contours().any(|contour| {
            contour
                .iter()
                .zip(contour.iter().cycle().skip(1))
                .take(contour.len())
                .any(|(&a, &b)| {
                    let cross = (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
                    cross.abs() < 1e-8
                        && x >= a.0.min(b.0)
                        && x <= a.0.max(b.0)
                        && y >= a.1.min(b.1)
                        && y <= a.1.max(b.1)
                })
        })
    }

    #[test]
    fn combine_modes_match_pointwise_oracle() {
        let prev = select(rect(2.0, 2.0, 7.0, 7.0));
        let shape = rect(6.0, 5.0, 6.0, 6.0);
        let other = select(shape);
        for mode in MODES {
            let result = Selection::combine(Some(&prev), shape, mode, CANVAS)
                .unwrap()
                .unwrap();
            for ix in 0..64 {
                for iy in 0..64 {
                    let (x, y) = (f64::from(ix) * 0.25, f64::from(iy) * 0.25);
                    if on_edge(&prev, x, y) || on_edge(&other, x, y) {
                        continue;
                    }
                    let (a, b) = (prev.contains(x, y), other.contains(x, y));
                    let expected = match mode {
                        CombineMode::Replace => b,
                        CombineMode::Union => a || b,
                        CombineMode::Exclude => a && !b,
                        CombineMode::Intersect => a && b,
                        CombineMode::Xor => a ^ b,
                    };
                    assert_eq!(result.contains(x, y), expected, "{mode:?} at {x},{y}");
                }
            }
        }
        assert_eq!(prev.bounds(), (2, 2, 9, 9));
    }

    #[test]
    fn clipping_empty_results_and_no_previous_selection() {
        let clipped = select(rect(-3.5, -2.0, 7.0, 20.0));
        assert_eq!(clipped.bounds(), (0, 0, 4, 16));
        assert!(!clipped.contains(-1.0, 1.0));
        assert!(clipped.contains(1.0, 1.0));
        let shape = rect(2.0, 2.0, 4.0, 4.0);
        let prev = select(shape);
        assert!(
            Selection::combine(Some(&prev), shape, CombineMode::Exclude, CANVAS)
                .unwrap()
                .is_none()
        );
        assert!(
            Selection::combine(
                Some(&prev),
                rect(10.0, 10.0, 2.0, 2.0),
                CombineMode::Intersect,
                CANVAS
            )
            .unwrap()
            .is_none()
        );
        assert!(
            Selection::combine(None, rect(0.0, 0.0, 0.0, 2.0), CombineMode::Replace, CANVAS)
                .unwrap()
                .is_none()
        );
        for mode in MODES {
            let result = Selection::combine(None, shape, mode, CANVAS).unwrap();
            assert_eq!(
                result.is_some(),
                matches!(
                    mode,
                    CombineMode::Replace | CombineMode::Union | CombineMode::Xor
                )
            );
            if let Some(result) = result {
                assert!(result.contains(3.0, 3.0));
            }
        }
    }

    #[test]
    fn lasso_lobes_and_rotated_ellipse_membership() {
        let lasso = select(SelectionShape::Lasso(&[
            (1.0, 1.0),
            (9.0, 9.0),
            (1.0, 9.0),
            (9.0, 1.0),
        ]));
        assert!(lasso.contains(5.0, 2.0));
        assert!(lasso.contains(5.0, 8.0));
        assert!(!lasso.contains(2.0, 5.0));
        let frame = ellipse();
        let selection = select(SelectionShape::Ellipse(frame));
        let oracle = PolygonSet::ellipse(
            frame.center.x,
            frame.center.y,
            frame.width / 2.0,
            frame.height / 2.0,
            frame.angle_degrees,
        )
        .unwrap();
        for ix in 0..64 {
            for iy in 0..64 {
                let (x, y) = (f64::from(ix) * 0.25, f64::from(iy) * 0.25);
                if !on_edge(&selection, x, y) {
                    assert_eq!(selection.contains(x, y), oracle.contains(x, y));
                }
            }
        }
    }

    #[test]
    fn invalid_geometry_and_canvas_mismatch_leave_previous_untouched() {
        let prev = select(rect(1.0, 1.0, 3.0, 3.0));
        for shape in [
            rect(f64::NAN, 1.0, 2.0, 2.0),
            rect(0.0, 0.0, f64::INFINITY, 2.0),
            SelectionShape::Lasso(&[(0.0, 0.0), (f64::MAX, 1.0)]),
        ] {
            assert!(matches!(
                Selection::combine(Some(&prev), shape, CombineMode::Union, CANVAS),
                Err(Error::InvalidShape)
            ));
        }
        let mut frame = ellipse();
        frame.angle_degrees = f64::INFINITY;
        assert!(matches!(
            Selection::combine(
                None,
                SelectionShape::Ellipse(frame),
                CombineMode::Replace,
                CANVAS
            ),
            Err(Error::InvalidShape)
        ));
        assert!(matches!(
            Selection::combine(
                Some(&prev),
                rect(1.0, 1.0, 2.0, 2.0),
                CombineMode::Replace,
                (20, 20)
            ),
            Err(Error::InvalidShape)
        ));
        assert_eq!(prev.bounds(), (1, 1, 4, 4));
        assert!(prev.contains(2.0, 2.0));
    }

    fn check_mask(selection: &Selection) {
        for quality in [ClipQuality::Aliased, ClipQuality::Antialiased] {
            let mask = selection.mask(quality).unwrap();
            let samples = if quality == ClipQuality::Aliased {
                1
            } else {
                4
            };
            for y in 0..CANVAS.1 {
                for x in 0..CANVAS.0 {
                    let mut count = 0;
                    let mut boundary = false;
                    for sy in 0..samples {
                        for sx in 0..samples {
                            let (px, py) = (
                                f64::from(x) + (f64::from(sx) + 0.5) / f64::from(samples),
                                f64::from(y) + (f64::from(sy) + 0.5) / f64::from(samples),
                            );
                            boundary |= on_edge(selection, px, py);
                            count += u32::from(selection.contains(px, py));
                        }
                    }
                    if !boundary {
                        let total = samples * samples;
                        assert_eq!(
                            mask.get(x, y),
                            ((count * 255 + total / 2) / total) as u8,
                            "{quality:?} at {x},{y}"
                        );
                    }
                }
            }
            if let Some((l, t, r, b)) = mask.covered_bounds() {
                let bounds = selection.bounds();
                assert!(l >= bounds.0 && t >= bounds.1 && r <= bounds.2 && b <= bounds.3);
            }
        }
    }

    #[test]
    fn scanline_masks_match_point_membership_oracles() {
        check_mask(&select(SelectionShape::Ellipse(ellipse())));
        check_mask(&select(rect(1.3, 2.2, 8.4, 7.7)));
        let outer = select(SelectionShape::Lasso(&[
            (1.1, 1.3),
            (13.2, 2.1),
            (12.4, 14.2),
            (2.2, 12.7),
        ]));
        let hole = Selection::combine(
            Some(&outer),
            rect(4.1, 4.3, 4.7, 4.4),
            CombineMode::Exclude,
            CANVAS,
        )
        .unwrap()
        .unwrap();
        check_mask(&hole);
    }

    #[test]
    fn antialiased_sample_indices_support_wide_canvases() {
        let selection = Selection::combine(
            None,
            rect(f64::from(MAX_DIMENSION) - 1024.0, 2.0, 128.0, 8.0),
            CombineMode::Replace,
            (MAX_DIMENSION, 16),
        )
        .unwrap()
        .unwrap();
        let mask = selection.mask(ClipQuality::Antialiased).unwrap();
        assert!(mask.covered_bounds().is_some());
    }

    #[test]
    fn integer_rectangles_have_exact_coverage() {
        let selection = select(rect(2.0, 3.0, 6.0, 7.0));
        for quality in [ClipQuality::Aliased, ClipQuality::Antialiased] {
            let mask = selection.mask(quality).unwrap();
            assert_eq!(mask.covered_bounds(), Some(selection.bounds()));
            for y in 0..16 {
                for x in 0..16 {
                    assert_eq!(
                        mask.get(x, y),
                        if (2..8).contains(&x) && (3..10).contains(&y) {
                            255
                        } else {
                            0
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn memory_failures_preserve_selection_and_release_reservations() {
        const CHILD: &str = "HOSHI_SELECTION_BUDGET_TEST";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "selection::tests::memory_failures_preserve_selection_and_release_reservations",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "{stdout}\n{stderr}");
            assert!(
                stdout.lines().any(|line| line.trim() == "running 1 test"),
                "{stdout}"
            );
            return;
        }
        let baseline = crate::memory_usage();
        crate::configure_memory_budget(1).unwrap();
        assert!(matches!(
            Selection::combine(None, rect(1.0, 1.0, 3.0, 3.0), CombineMode::Replace, CANVAS),
            Err(Error::MemoryLimit)
        ));
        assert_eq!(crate::memory_usage(), baseline);
        crate::configure_memory_budget(64).unwrap();
        let selection = select(rect(1.0, 1.0, 3.0, 3.0));
        assert_eq!(crate::memory_usage(), baseline + 64);
        assert!(matches!(
            selection.transformed(&Affine::IDENTITY),
            Err(Error::MemoryLimit)
        ));
        assert_eq!(crate::memory_usage(), baseline + 64);
        for quality in [ClipQuality::Aliased, ClipQuality::Antialiased] {
            assert!(matches!(selection.mask(quality), Err(Error::MemoryLimit)));
            assert_eq!(crate::memory_usage(), baseline + 64);
        }
        assert!(matches!(
            Selection::combine(
                Some(&selection),
                rect(2.0, 2.0, 4.0, 4.0),
                CombineMode::Union,
                CANVAS
            ),
            Err(Error::MemoryLimit)
        ));
        assert_eq!(selection.bounds(), (1, 1, 4, 4));
        assert!(selection.contains(2.0, 2.0));
        drop(selection);
        assert_eq!(crate::memory_usage(), baseline);
    }
    #[test]
    fn transformed_selection_preserves_exact_geometry_and_rotation_membership() {
        let original = select(rect(5., 6., 6., 4.));
        let identity = original.transformed(&Affine::IDENTITY).unwrap().unwrap();
        assert_eq!(identity.bounds(), original.bounds());
        assert!(identity.contours().eq(original.contours()));
        let transform = Affine::rotate_about(Point::new(8., 8.), 30.);
        let rotated = original.transformed(&transform).unwrap().unwrap();
        let area = |selection: &Selection| {
            selection
                .contours()
                .map(|path| {
                    path.iter()
                        .zip(path.iter().cycle().skip(1))
                        .take(path.len())
                        .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
                        .sum::<f64>()
                })
                .sum::<f64>()
                .abs()
                * 0.5
        };
        assert!((area(&rotated) - area(&original)).abs() < 1e-9 * area(&original));
        let inverse = transform.inverse().unwrap();
        for x in 0..64 {
            for y in 0..64 {
                let (x, y) = (f64::from(x) * 0.25, f64::from(y) * 0.25);
                let p = inverse.apply(Point::new(x, y));
                if !on_edge(&rotated, x, y) && !on_edge(&original, p.x, p.y) {
                    assert_eq!(rotated.contains(x, y), original.contains(p.x, p.y));
                }
            }
        }
        let clipped = original
            .transformed(&Affine::translate(-7., 0.))
            .unwrap()
            .unwrap();
        assert_eq!(clipped.bounds(), (0, 6, 4, 10));
        assert!(
            original
                .transformed(&Affine::translate(20., 0.))
                .unwrap()
                .is_none()
        );
        for invalid in [Affine::scale(0., 1.), Affine::translate(f64::NAN, 0.)] {
            assert!(matches!(
                original.transformed(&invalid),
                Err(Error::InvalidShape)
            ));
            assert_eq!(original.bounds(), identity.bounds());
            assert!(original.contours().eq(identity.contours()));
        }
    }

    fn reference_mask(selection: &Selection, q: ClipQuality) -> MaskStore {
        fn build(selection: &Selection, q: ClipQuality) -> Result<MaskStore, Error> {
            let mut mask = MaskStore::new(selection.canvas.0, selection.canvas.1, 0);
            let (left, top, right, bottom) = selection.bounds;
            let samples = match q {
                ClipQuality::Aliased => 1,
                ClipQuality::Antialiased => 4,
            };
            let mut crossings = Vec::new();
            let edges = selection.contours().try_fold(0_usize, |count, contour| {
                count.checked_add(contour.len()).ok_or(Error::MemoryLimit)
            })?;
            crossings
                .try_reserve(edges)
                .map_err(|_| Error::MemoryLimit)?;
            let mut counts = Vec::new();
            counts
                .try_reserve_exact((right - left) as usize)
                .map_err(|_| Error::MemoryLimit)?;
            counts.resize((right - left) as usize, 0_u8);
            for y in top..bottom {
                counts.fill(0);
                for sy in 0..samples {
                    reference_crossings(
                        selection,
                        f64::from(y) + (f64::from(sy) + 0.5) / f64::from(samples),
                        &mut crossings,
                    );
                    count_spans(&crossings, samples, left, right, &mut counts);
                }
                for (x, &count) in (left..right).zip(&counts) {
                    if count != 0 {
                        let total = samples * samples;
                        mask.set_max(x, y, ((u32::from(count) * 255 + total / 2) / total) as u8)?;
                    }
                }
            }
            Ok(mask)
        }

        build(selection, q).unwrap()
    }

    fn reference_crossings(selection: &Selection, y: f64, crossings: &mut Vec<(f64, i64)>) {
        crossings.clear();
        for contour in selection.contours() {
            for (&a, &b) in contour
                .iter()
                .zip(contour.iter().cycle().skip(1))
                .take(contour.len())
            {
                if (a.1 <= y && b.1 > y) || (b.1 <= y && a.1 > y) {
                    crossings.push((
                        a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1),
                        if b.1 > a.1 { 1 } else { -1 },
                    ));
                }
            }
        }
        crossings.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    }

    #[test]
    fn active_edge_masks_match_the_original_scanline_for_every_pixel() {
        let outer = select(SelectionShape::Lasso(&[
            (1.1, 1.3),
            (13.2, 2.1),
            (12.4, 14.2),
            (2.2, 12.7),
        ]));
        let hole = Selection::combine(
            Some(&outer),
            rect(4.1, 4.3, 4.7, 4.4),
            CombineMode::Exclude,
            CANVAS,
        )
        .unwrap()
        .unwrap();
        let bowtie = select(SelectionShape::Lasso(&[
            (1.0, 1.0),
            (9.0, 9.0),
            (1.0, 9.0),
            (9.0, 1.0),
        ]));
        for selection in [
            select(SelectionShape::Ellipse(ellipse())),
            select(rect(1.3, 2.2, 8.4, 7.7)),
            hole,
            bowtie,
        ] {
            for quality in [ClipQuality::Aliased, ClipQuality::Antialiased] {
                let mask = selection.mask(quality).unwrap();
                let reference = reference_mask(&selection, quality);
                for y in 0..CANVAS.1 {
                    for x in 0..CANVAS.0 {
                        assert_eq!(
                            mask.get(x, y),
                            reference.get(x, y),
                            "{quality:?} at {x},{y}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_canvas_sizes_fail_without_changing_the_previous_selection() {
        let previous = select(rect(1.0, 1.0, 3.0, 3.0));
        let contours: Vec<_> = previous.contours().map(<[_]>::to_vec).collect();
        for canvas in [
            (0, 16),
            (16, 0),
            (crate::MAX_DIMENSION + 1, 16),
            (16, crate::MAX_DIMENSION + 1),
        ] {
            for prev in [None, Some(&previous)] {
                assert!(matches!(
                    Selection::combine(
                        prev,
                        rect(1.0, 1.0, 2.0, 2.0),
                        CombineMode::Replace,
                        canvas
                    ),
                    Err(Error::InvalidShape)
                ));
            }
            assert_eq!(previous.bounds(), (1, 1, 4, 4));
            assert!(previous.contours().eq(contours.iter().map(Vec::as_slice)));
        }
        let maximum = Selection::combine(
            None,
            rect(1.0, 1.0, 3.0, 3.0),
            CombineMode::Replace,
            (crate::MAX_DIMENSION, crate::MAX_DIMENSION),
        )
        .unwrap()
        .unwrap();
        assert_eq!(maximum.bounds(), (1, 1, 4, 4));
        assert_eq!(maximum.mask(ClipQuality::Aliased).unwrap().get(2, 2), 255);
    }

    #[test]
    fn mask_scratch_is_budgeted_and_released() {
        const CHILD: &str = "HOSHI_SELECTION_SCRATCH_BUDGET_TEST";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "selection::tests::mask_scratch_is_budgeted_and_released",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "{stdout}\n{stderr}");
            assert!(
                stdout.lines().any(|line| line.trim() == "running 1 test"),
                "{stdout}"
            );
            return;
        }
        let mut points: Vec<_> = (0..=2048)
            .map(|i| {
                (
                    1.0 + f64::from(i) * 0.05,
                    if i % 2 == 0 { 1.0 } else { 1.05 },
                )
            })
            .collect();
        points.extend([(103.4, 100.0), (1.0, 100.0)]);
        let selection = Selection::combine(
            None,
            SelectionShape::Lasso(&points),
            CombineMode::Replace,
            (128, 128),
        )
        .unwrap()
        .unwrap();
        assert!(selection.contours().map(<[_]>::len).sum::<usize>() >= 2048);
        let small = Selection::combine(
            None,
            rect(1.0, 1.0, 3.0, 3.0),
            CombineMode::Replace,
            (128, 128),
        )
        .unwrap()
        .unwrap();
        let baseline = crate::memory_usage();
        crate::configure_memory_budget(baseline + 16_384 + 1_024).unwrap();
        assert!(matches!(
            selection.mask(ClipQuality::Aliased),
            Err(Error::MemoryLimit)
        ));
        assert_eq!(crate::memory_usage(), baseline);
        let mask = small.mask(ClipQuality::Aliased).unwrap();
        assert_eq!(mask.get(2, 2), 255);
        assert_eq!(crate::memory_usage(), baseline + 16_384);
        drop(mask);
        assert_eq!(crate::memory_usage(), baseline);
    }
}
