//! Core geometry vocabulary shared by every shape (docs/specs/07-pin-geometry-engine.md
//! §Geometry Engine Responsibilities). A [`Path`] is a sequence of [`Segment`]s walked
//! as one continuous distance axis — this is what makes corner-crossing spacing
//! trivial: callers never need to know where one segment ends and the next begins.
//!
//! Port of web `src/domain/paths/*`.

use serde::{Deserialize, Serialize};

use crate::GeometryError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ORIGIN: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Point) -> f64 {
        (other.x - self.x).hypot(other.y - self.y)
    }
}

/// Ellipse arc length has no closed form (spec 07 §Geometry Precision requires
/// "accurate numerical path length" within a defined tolerance). Approach: densely
/// sample the parametric ellipse, build a cumulative chord-length table, and
/// interpolate — accurate to well under 0.1% at this sample count.
const ELLIPSE_SAMPLES: usize = 1440;

#[derive(Clone, Debug, PartialEq)]
pub struct EllipticalArc {
    pub center: Point,
    pub radius_x: f64,
    pub radius_y: f64,
    pub rotation: f64,
    pub start_angle: f64,
    pub sweep: f64,
    cumulative_length: Vec<f64>,
    angles: Vec<f64>,
}

impl EllipticalArc {
    pub fn new(
        center: Point,
        radius_x: f64,
        radius_y: f64,
        rotation: f64,
        start_angle: f64,
        sweep: f64,
    ) -> Self {
        let mut arc = Self {
            center,
            radius_x,
            radius_y,
            rotation,
            start_angle,
            sweep,
            cumulative_length: Vec::with_capacity(ELLIPSE_SAMPLES + 1),
            angles: Vec::with_capacity(ELLIPSE_SAMPLES + 1),
        };
        let mut acc = 0.0;
        let mut prev = arc.point_at_angle(start_angle);
        arc.cumulative_length.push(0.0);
        arc.angles.push(start_angle);
        for i in 1..=ELLIPSE_SAMPLES {
            let angle = start_angle + (sweep * i as f64) / ELLIPSE_SAMPLES as f64;
            let point = arc.point_at_angle(angle);
            acc += prev.distance(point);
            arc.cumulative_length.push(acc);
            arc.angles.push(angle);
            prev = point;
        }
        arc
    }

    pub fn point_at_angle(&self, angle: f64) -> Point {
        let ex = self.radius_x * angle.cos();
        let ey = self.radius_y * angle.sin();
        let (sin, cos) = self.rotation.sin_cos();
        Point::new(
            self.center.x + ex * cos - ey * sin,
            self.center.y + ex * sin + ey * cos,
        )
    }

    fn length(&self) -> f64 {
        *self.cumulative_length.last().expect("table is never empty")
    }

    fn point_at_distance(&self, distance: f64) -> Point {
        let table = &self.cumulative_length;
        if distance <= 0.0 {
            return self.point_at_angle(self.angles[0]);
        }
        let total = self.length();
        if distance >= total {
            return self.point_at_angle(*self.angles.last().expect("non-empty"));
        }
        // Binary search for the bracketing sample pair, then interpolate the angle.
        let (mut lo, mut hi) = (0, table.len() - 1);
        while lo < hi - 1 {
            let mid = (lo + hi) / 2;
            if table[mid] <= distance {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let seg_len = table[hi] - table[lo];
        let t = if seg_len == 0.0 {
            0.0
        } else {
            (distance - table[lo]) / seg_len
        };
        let angle = self.angles[lo] + (self.angles[hi] - self.angles[lo]) * t;
        self.point_at_angle(angle)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Segment {
    Line {
        start: Point,
        end: Point,
    },
    /// True circular arc — length is exact (radius × angle). A full circle is an
    /// arc with sweep = 2π. Positive sweep = counter-clockwise.
    CircularArc {
        center: Point,
        radius: f64,
        start_angle: f64,
        sweep: f64,
    },
    EllipticalArc(EllipticalArc),
}

impl Segment {
    pub fn line(start: Point, end: Point) -> Self {
        Segment::Line { start, end }
    }

    pub fn circular_arc(center: Point, radius: f64, start_angle: f64, sweep: f64) -> Self {
        Segment::CircularArc {
            center,
            radius,
            start_angle,
            sweep,
        }
    }

    pub fn elliptical_arc(
        center: Point,
        radius_x: f64,
        radius_y: f64,
        rotation: f64,
        start_angle: f64,
        sweep: f64,
    ) -> Self {
        Segment::EllipticalArc(EllipticalArc::new(
            center,
            radius_x,
            radius_y,
            rotation,
            start_angle,
            sweep,
        ))
    }

    pub fn length(&self) -> f64 {
        match self {
            Segment::Line { start, end } => start.distance(*end),
            Segment::CircularArc { radius, sweep, .. } => (radius * sweep).abs(),
            Segment::EllipticalArc(arc) => arc.length(),
        }
    }

    /// `distance` is measured from the start of this segment, in `[0, length()]`.
    pub fn point_at_distance(&self, distance: f64) -> Point {
        match self {
            Segment::Line { start, end } => {
                let len = self.length();
                if len == 0.0 {
                    return *start;
                }
                let t = distance / len;
                Point::new(
                    start.x + (end.x - start.x) * t,
                    start.y + (end.y - start.y) * t,
                )
            }
            Segment::CircularArc {
                center,
                radius,
                start_angle,
                sweep,
            } => {
                let len = self.length();
                let t = if len == 0.0 { 0.0 } else { distance / len };
                let angle = start_angle + sweep * t;
                Point::new(
                    center.x + radius * angle.cos(),
                    center.y + radius * angle.sin(),
                )
            }
            Segment::EllipticalArc(arc) => arc.point_at_distance(distance),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Path {
    pub segments: Vec<Segment>,
    pub closed: bool,
}

impl Path {
    pub fn new(segments: Vec<Segment>, closed: bool) -> Self {
        Self { segments, closed }
    }

    pub fn length(&self) -> f64 {
        self.segments.iter().map(Segment::length).sum()
    }

    /// Walks the segments as one continuous distance axis — pin spacing continues
    /// across corners instead of resetting per edge (spec 07 §Continuous Pin Spacing).
    pub fn point_at_distance(&self, distance: f64) -> Point {
        let mut remaining = distance;
        let last = self.segments.len().saturating_sub(1);
        for (i, segment) in self.segments.iter().enumerate() {
            let len = segment.length();
            if remaining <= len || i == last {
                return segment.point_at_distance(remaining.min(len).max(0.0));
            }
            remaining -= len;
        }
        // Empty path — degenerate, but never panic on it.
        Point::ORIGIN
    }

    /// Samples each segment's actual extent (32 samples per segment) so curves
    /// contribute their full envelope to viewport-fit and print tiling.
    pub fn bounding_box_points(&self) -> Vec<Point> {
        const SAMPLES: usize = 32;
        let mut points = Vec::with_capacity(self.segments.len() * (SAMPLES + 1));
        for segment in &self.segments {
            let len = segment.length();
            for i in 0..=SAMPLES {
                points.push(segment.point_at_distance(len * i as f64 / SAMPLES as f64));
            }
        }
        if points.is_empty() {
            points.push(Point::ORIGIN);
        }
        points
    }
}

/// Spec 07 §Open-Path Pin Distribution — pins at 0, D, 2D, … floor(L/D)·D.
/// The endpoint is never forced.
pub fn distribute_open_path(path: &Path, spacing: f64) -> Result<Vec<Point>, GeometryError> {
    if spacing <= 0.0 {
        return Err(GeometryError::NonPositiveSpacing);
    }
    let count = (path.length() / spacing).floor() as usize + 1;
    Ok((0..count)
        .map(|i| path.point_at_distance(i as f64 * spacing))
        .collect())
}

/// Spec 07 §Closed-Path Pin Count Algorithm — the integer interval count N that
/// makes `perimeter / N` closest to the requested spacing.
pub fn closest_interval_count(perimeter: f64, requested_spacing: f64) -> usize {
    let approx = perimeter / requested_spacing;
    let lo = approx.floor().max(1.0);
    let hi = approx.ceil().max(1.0);
    if lo == hi {
        return lo as usize;
    }
    let diff_lo = (perimeter / lo - requested_spacing).abs();
    let diff_hi = (perimeter / hi - requested_spacing).abs();
    if diff_hi < diff_lo {
        hi as usize
    } else {
        lo as usize
    }
}

/// Spec 32 — inverse of [`closest_interval_count`]: a spacing that lands on
/// EXACTLY `count`. Biased 1e-9 smaller so `floor` never rounds down to
/// `count - 1` from floating-point error.
pub fn spacing_for_pin_count(length: f64, count: usize) -> Result<f64, GeometryError> {
    if count == 0 {
        return Err(GeometryError::NonPositiveCount);
    }
    Ok((length / count as f64) * (1.0 - 1e-9))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Distribution {
    pub points: Vec<Point>,
    pub actual_spacing: f64,
    pub n: usize,
}

/// Spec 07 §Closed-Path Pin Distribution — uniform spacing around the full
/// perimeter, no duplicate start/end pin.
pub fn distribute_closed_path(
    path: &Path,
    requested_spacing: f64,
) -> Result<Distribution, GeometryError> {
    if requested_spacing <= 0.0 {
        return Err(GeometryError::NonPositiveSpacing);
    }
    let perimeter = path.length();
    let n = closest_interval_count(perimeter, requested_spacing);
    let actual_spacing = perimeter / n as f64;
    let points = (0..n)
        .map(|i| path.point_at_distance(i as f64 * actual_spacing))
        .collect();
    Ok(Distribution {
        points,
        actual_spacing,
        n,
    })
}

/// Vertex-anchored distribution for straight-edged shapes: every corner gets a
/// pin, and each edge's interior pins independently use the closest-N rule.
pub fn distribute_path_per_vertex(
    path: &Path,
    requested_spacing: f64,
) -> Result<Distribution, GeometryError> {
    if requested_spacing <= 0.0 {
        return Err(GeometryError::NonPositiveSpacing);
    }
    let mut points = Vec::new();
    let mut representative_spacing = requested_spacing;
    for (i, segment) in path.segments.iter().enumerate() {
        let seg_len = segment.length();
        // The vertex — shared with the previous segment's end, never duplicated.
        points.push(segment.point_at_distance(0.0));
        let n = closest_interval_count(seg_len, requested_spacing);
        let seg_actual = seg_len / n as f64;
        if i == 0 {
            representative_spacing = seg_actual;
        }
        for k in 1..n {
            points.push(segment.point_at_distance(k as f64 * seg_actual));
        }
    }
    // Open paths have no next segment to own the final vertex.
    if !path.closed
        && let Some(last) = path.segments.last()
    {
        points.push(last.point_at_distance(last.length()));
    }
    let n = points.len();
    Ok(Distribution {
        points,
        actual_spacing: representative_spacing,
        n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_close;
    use std::f64::consts::PI;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn open_line(length: f64) -> Path {
        Path::new(vec![Segment::line(p(0.0, 0.0), p(length, 0.0))], false)
    }

    fn regular_hexagon(side: f64) -> Path {
        let vertices: Vec<Point> = (0..6)
            .map(|i| {
                let a = PI / 3.0 * i as f64;
                p(side * a.cos(), side * a.sin())
            })
            .collect();
        let segments = (0..6)
            .map(|i| Segment::line(vertices[i], vertices[(i + 1) % 6]))
            .collect();
        Path::new(segments, true)
    }

    fn closed_loop_of_perimeter(perimeter: f64) -> Path {
        let radius = perimeter / (2.0 * PI);
        Path::new(
            vec![Segment::circular_arc(p(0.0, 0.0), radius, 0.0, 2.0 * PI)],
            true,
        )
    }

    // --- CircularArcSegment
    #[test]
    fn circular_arc_length() {
        assert_close(
            Segment::circular_arc(p(0.0, 0.0), 10.0, 0.0, PI).length(),
            10.0 * PI,
            1e-5,
        );
    }

    #[test]
    fn circular_arc_interpolates_sweep() {
        let seg = Segment::circular_arc(p(0.0, 0.0), 10.0, 0.0, PI / 2.0);
        let s = seg.point_at_distance(0.0);
        assert_close(s.x, 10.0, 1e-5);
        assert_close(s.y, 0.0, 1e-5);
        let e = seg.point_at_distance(seg.length());
        assert_close(e.x, 0.0, 1e-5);
        assert_close(e.y, 10.0, 1e-5);
    }

    #[test]
    fn circular_arc_zero_sweep_returns_start() {
        let seg = Segment::circular_arc(p(5.0, 5.0), 10.0, 0.0, 0.0);
        assert_eq!(seg.length(), 0.0);
        let q = seg.point_at_distance(0.0);
        assert_close(q.x, 15.0, 1e-5);
        assert_close(q.y, 5.0, 1e-5);
    }

    // --- EllipticalArcSegment
    #[test]
    fn ellipse_circle_circumference() {
        let seg = Segment::elliptical_arc(p(0.0, 0.0), 10.0, 10.0, 0.0, 0.0, 2.0 * PI);
        assert_close(seg.length(), 2.0 * PI * 10.0, 0.05);
    }

    #[test]
    fn ellipse_start_for_non_positive_distance() {
        let seg = Segment::elliptical_arc(p(5.0, 5.0), 10.0, 4.0, 0.0, 0.0, PI);
        assert_eq!(seg.point_at_distance(0.0), p(15.0, 5.0));
        assert_eq!(seg.point_at_distance(-5.0), p(15.0, 5.0));
    }

    #[test]
    fn ellipse_end_for_distance_past_total() {
        let seg = Segment::elliptical_arc(p(0.0, 0.0), 10.0, 4.0, 0.0, 0.0, PI);
        let total = seg.length();
        let end = seg.point_at_distance(total);
        assert_close(end.x, -10.0, 1e-5);
        assert_close(end.y, 0.0, 1e-5);
        assert_eq!(seg.point_at_distance(total + 100.0), end);
    }

    #[test]
    fn ellipse_mid_arc_point() {
        let seg = Segment::elliptical_arc(p(0.0, 0.0), 10.0, 10.0, 0.0, 0.0, PI / 2.0);
        let mid = seg.point_at_distance(seg.length() / 2.0);
        assert_close(mid.x, 10.0 * (PI / 4.0).cos(), 0.05);
        assert_close(mid.y, 10.0 * (PI / 4.0).sin(), 0.05);
    }

    #[test]
    fn ellipse_rotation_applies() {
        let a = Segment::elliptical_arc(p(0.0, 0.0), 10.0, 4.0, 0.0, 0.0, PI);
        let b = Segment::elliptical_arc(p(0.0, 0.0), 10.0, 4.0, PI / 2.0, 0.0, PI);
        let pa = a.point_at_distance(a.length() / 2.0);
        let pb = b.point_at_distance(b.length() / 2.0);
        assert_close(pb.x, -pa.y, 0.05);
        assert_close(pb.y, pa.x, 0.05);
    }

    #[test]
    fn ellipse_negative_sweep() {
        let seg = Segment::elliptical_arc(p(0.0, 0.0), 10.0, 10.0, 0.0, 0.0, -PI / 2.0);
        assert_close(seg.length(), PI / 2.0 * 10.0, 0.05);
        let end = seg.point_at_distance(seg.length());
        assert_close(end.x, 0.0, 0.05);
        assert_close(end.y, -10.0, 0.05);
    }

    // --- Path
    fn l_path() -> Path {
        Path::new(
            vec![
                Segment::line(p(0.0, 0.0), p(10.0, 0.0)),
                Segment::line(p(10.0, 0.0), p(10.0, 10.0)),
            ],
            false,
        )
    }

    #[test]
    fn path_length_sums_segments() {
        assert_eq!(l_path().length(), 20.0);
    }

    #[test]
    fn point_at_distance_walks_into_next_segment() {
        assert_eq!(l_path().point_at_distance(5.0), p(5.0, 0.0));
        assert_eq!(l_path().point_at_distance(12.0), p(10.0, 2.0));
    }

    #[test]
    fn spacing_carries_across_corner() {
        let points = distribute_open_path(&l_path(), 3.0).unwrap();
        assert_eq!(points.len(), 7);
        assert_eq!(points[4], p(10.0, 2.0));
    }

    #[test]
    fn empty_path_returns_origin() {
        assert_eq!(Path::default().point_at_distance(5.0), Point::ORIGIN);
        assert_eq!(Path::default().bounding_box_points(), vec![Point::ORIGIN]);
    }

    // --- distributeOpenPath
    #[test]
    fn open_8_by_1() {
        let xs: Vec<f64> = distribute_open_path(&open_line(8.0), 1.0)
            .unwrap()
            .iter()
            .map(|q| q.x)
            .collect();
        assert_eq!(xs, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn open_7_5_by_1() {
        let pts = distribute_open_path(&open_line(7.5), 1.0).unwrap();
        assert_eq!(pts.len(), 8);
        assert_eq!(pts.last().unwrap().x, 7.0);
    }

    #[test]
    fn open_spacing_larger_than_path() {
        let pts = distribute_open_path(&open_line(0.5), 5.0).unwrap();
        assert_eq!(pts, vec![p(0.0, 0.0)]);
    }

    #[test]
    fn open_tiny_path() {
        assert_eq!(
            distribute_open_path(&open_line(0.01), 0.001).unwrap().len(),
            11
        );
    }

    #[test]
    fn open_rejects_zero_spacing() {
        assert_eq!(
            distribute_open_path(&open_line(8.0), 0.0),
            Err(GeometryError::NonPositiveSpacing)
        );
    }

    // --- closestIntervalCount
    #[test]
    fn closest_interval_counts() {
        assert_eq!(closest_interval_count(31.0, 2.0), 16);
        assert_eq!(closest_interval_count(30.0, 3.0), 10);
        assert_eq!(closest_interval_count(100.0, 7.0), 14);
        assert_eq!(closest_interval_count(5.0, 20.0), 1);
    }

    // --- spacingForPinCount
    #[test]
    fn spacing_for_pin_count_round_trips_closed() {
        #[allow(clippy::approx_constant)] // web test fixture value, not τ
        for perimeter in [1.0, 6.28318, 31.0, 100.0, 314.159] {
            for count in [3, 4, 5, 7, 16, 40, 99, 180, 200, 399] {
                let spacing = spacing_for_pin_count(perimeter, count).unwrap();
                let r =
                    distribute_closed_path(&closed_loop_of_perimeter(perimeter), spacing).unwrap();
                assert_eq!(r.n, count, "perimeter {perimeter} count {count}");
            }
        }
    }

    #[test]
    fn spacing_for_pin_count_round_trips_per_vertex() {
        for nails in [2, 3, 6, 10, 40] {
            let spacing = spacing_for_pin_count(3.5, nails).unwrap();
            let r = distribute_path_per_vertex(&regular_hexagon(3.5), spacing).unwrap();
            assert_eq!(r.n, 6 * nails);
        }
    }

    #[test]
    fn spacing_for_pin_count_rejects_zero() {
        assert!(spacing_for_pin_count(10.0, 0).is_err());
    }

    // --- distributeClosedPath
    #[test]
    fn closed_31_by_2() {
        let r = distribute_closed_path(&closed_loop_of_perimeter(31.0), 2.0).unwrap();
        assert_eq!(r.n, 16);
        assert_close(r.actual_spacing, 1.9375, 1e-4);
        assert_eq!(r.points.len(), 16);
    }

    #[test]
    fn closed_exact_division() {
        let r = distribute_closed_path(&closed_loop_of_perimeter(30.0), 3.0).unwrap();
        assert_eq!(r.n, 10);
        assert_close(r.actual_spacing, 3.0, 1e-6);
    }

    #[test]
    fn closed_spacing_larger_than_perimeter() {
        assert_eq!(
            distribute_closed_path(&closed_loop_of_perimeter(5.0), 20.0)
                .unwrap()
                .n,
            1
        );
    }

    #[test]
    fn closed_rejects_zero_spacing() {
        assert!(distribute_closed_path(&closed_loop_of_perimeter(31.0), 0.0).is_err());
    }

    // --- distributePathPerVertex
    #[test]
    fn per_vertex_hexagon() {
        let r = distribute_path_per_vertex(&regular_hexagon(2.0), 1.0).unwrap();
        assert_eq!(r.n, 12);
        assert_close(r.actual_spacing, 1.0, 1e-6);
    }

    #[test]
    fn per_vertex_open_line_exact() {
        let r = distribute_path_per_vertex(&open_line(8.0), 1.0).unwrap();
        assert_eq!(r.n, 9);
        assert_eq!(r.points[0], p(0.0, 0.0));
        assert_eq!(*r.points.last().unwrap(), p(8.0, 0.0));
    }

    #[test]
    fn per_vertex_open_line_forces_endpoints() {
        let r = distribute_path_per_vertex(&open_line(7.5), 1.0).unwrap();
        assert_eq!(r.n, 9);
        assert_close(r.actual_spacing, 0.9375, 1e-4);
        assert_eq!(*r.points.last().unwrap(), p(7.5, 0.0));
    }

    #[test]
    fn per_vertex_short_edge() {
        let r = distribute_path_per_vertex(&open_line(0.5), 5.0).unwrap();
        assert_eq!(r.points, vec![p(0.0, 0.0), p(0.5, 0.0)]);
    }

    #[test]
    fn per_vertex_symmetric_spacing() {
        let r = distribute_path_per_vertex(&regular_hexagon(3.5), 2.0).unwrap();
        assert_eq!(r.n, 12);
        for i in 0..r.points.len() {
            let next = r.points[(i + 1) % r.points.len()];
            assert_close(r.points[i].distance(next), r.actual_spacing, 1e-4);
        }
    }

    #[test]
    fn per_vertex_rejects_zero_spacing() {
        assert!(distribute_path_per_vertex(&regular_hexagon(2.0), 0.0).is_err());
    }
}
