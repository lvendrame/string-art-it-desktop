//! Shape constructors producing [`Path`]s. Port of web `src/domain/shapes/*`.

use std::f64::consts::PI;

use crate::GeometryError;
use crate::paths::{Path, Point, Segment};
use crate::transforms::rotate_point;

pub fn line_shape(start: Point, end: Point) -> Path {
    Path::new(vec![Segment::line(start, end)], false)
}

/// A hand-drawn shape: short line segments chained end-to-end. A lone point
/// becomes one zero-length segment so its single pin lands on that point.
pub fn freehand_shape(points: &[Point]) -> Path {
    if points.len() == 1 {
        return Path::new(vec![Segment::line(points[0], points[0])], false);
    }
    let segments = points
        .windows(2)
        .map(|w| Segment::line(w[0], w[1]))
        .collect();
    Path::new(segments, false)
}

/// A closed contour (one loop of a flattened glyph outline, a Path-tool polygon).
pub fn closed_polyline_shape(points: &[Point]) -> Path {
    if points.len() < 2 {
        return Path::new(Vec::new(), true);
    }
    let segments = (0..points.len())
        .map(|i| Segment::line(points[i], points[(i + 1) % points.len()]))
        .collect();
    Path::new(segments, true)
}

fn normalize_angle_diff(angle: f64) -> f64 {
    let mut d = angle % (2.0 * PI);
    if d <= -PI {
        d += 2.0 * PI;
    }
    if d > PI {
        d -= 2.0 * PI;
    }
    d
}

/// Start + End + Curvature, where curvature is the signed sagitta (perpendicular
/// bulge distance from the chord midpoint). Curvature 0 degenerates to a line.
pub fn arc_shape(start: Point, end: Point, curvature: f64) -> Path {
    if curvature == 0.0 {
        return line_shape(start, end);
    }
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let chord = dx.hypot(dy);
    let mid = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    let (nx, ny) = (-dy / chord, dx / chord);
    let s = curvature;
    let r = (4.0 * s * s + chord * chord) / (8.0 * s);

    let apex = Point::new(mid.x + nx * s, mid.y + ny * s);
    let center = Point::new(mid.x - nx * (r - s), mid.y - ny * (r - s));
    let radius = r.abs();

    let start_angle = (start.y - center.y).atan2(start.x - center.x);
    let end_angle = (end.y - center.y).atan2(end.x - center.x);

    let short_sweep = normalize_angle_diff(end_angle - start_angle);
    let sign = if short_sweep == 0.0 {
        1.0
    } else {
        short_sweep.signum()
    };
    let long_sweep = short_sweep - sign * 2.0 * PI;

    let mid_of = |sweep: f64| {
        let a = start_angle + sweep / 2.0;
        Point::new(center.x + radius * a.cos(), center.y + radius * a.sin())
    };
    let sweep = if mid_of(short_sweep).distance(apex) <= mid_of(long_sweep).distance(apex) {
        short_sweep
    } else {
        long_sweep
    };
    Path::new(
        vec![Segment::circular_arc(center, radius, start_angle, sweep)],
        false,
    )
}

pub fn circle_shape(center: Point, radius: f64) -> Path {
    Path::new(
        vec![Segment::circular_arc(center, radius, 0.0, 2.0 * PI)],
        true,
    )
}

/// A circle is an ellipse with equal radii.
pub fn ellipse_shape(center: Point, radius_x: f64, radius_y: f64, rotation: f64) -> Path {
    if radius_x == radius_y {
        return circle_shape(center, radius_x);
    }
    Path::new(
        vec![Segment::elliptical_arc(
            center,
            radius_x,
            radius_y,
            rotation,
            0.0,
            2.0 * PI,
        )],
        true,
    )
}

/// `position` is the top-left corner before rotation; rotation (radians) is
/// about the rectangle's own centre.
pub fn rectangle_shape(position: Point, width: f64, height: f64, rotation: f64) -> Path {
    let center = Point::new(position.x + width / 2.0, position.y + height / 2.0);
    let corners: Vec<Point> = [
        Point::new(position.x, position.y),
        Point::new(position.x + width, position.y),
        Point::new(position.x + width, position.y + height),
        Point::new(position.x, position.y + height),
    ]
    .into_iter()
    .map(|p| rotate_point(p, center, rotation))
    .collect();
    closed_from_vertices(&corners)
}

pub fn square_shape(position: Point, side: f64, rotation: f64) -> Path {
    rectangle_shape(position, side, side, rotation)
}

fn vertex_at(center: Point, radius: f64, angle: f64) -> Point {
    Point::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

fn closed_from_vertices(vertices: &[Point]) -> Path {
    let segments = (0..vertices.len())
        .map(|i| Segment::line(vertices[i], vertices[(i + 1) % vertices.len()]))
        .collect();
    Path::new(segments, true)
}

/// Regular polygon inscribed in a circle of `radius`, first vertex at the top.
pub fn regular_polygon_shape(
    center: Point,
    radius: f64,
    sides: usize,
    rotation: f64,
) -> Result<Path, GeometryError> {
    if sides < 3 {
        return Err(GeometryError::TooFewSides);
    }
    let vertices: Vec<Point> = (0..sides)
        .map(|i| {
            vertex_at(
                center,
                radius,
                rotation + 2.0 * PI * i as f64 / sides as f64 - PI / 2.0,
            )
        })
        .collect();
    Ok(closed_from_vertices(&vertices))
}

/// Conventional star: `points` outer tips alternating with inner valleys.
pub fn star_shape(
    center: Point,
    outer_radius: f64,
    inner_radius: f64,
    points: usize,
    rotation: f64,
) -> Result<Path, GeometryError> {
    if points < 3 {
        return Err(GeometryError::TooFewStarPoints);
    }
    let step = PI / points as f64;
    let vertices: Vec<Point> = (0..points * 2)
        .map(|i| {
            let angle = rotation + i as f64 * step - PI / 2.0;
            let r = if i % 2 == 0 {
                outer_radius
            } else {
                inner_radius
            };
            vertex_at(center, r, angle)
        })
        .collect();
    Ok(closed_from_vertices(&vertices))
}

pub fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Star polygon `{points/skip}` — connects every `skip`-th vertex, producing the
/// self-intersecting perimeter.
pub fn polygram_shape(
    center: Point,
    radius: f64,
    points: usize,
    skip: usize,
    rotation: f64,
) -> Result<Path, GeometryError> {
    if gcd(points, skip) != 1 {
        return Err(GeometryError::PolygramNotContinuous { points, skip });
    }
    let anchors: Vec<Point> = (0..points)
        .map(|i| {
            vertex_at(
                center,
                radius,
                rotation + 2.0 * PI * i as f64 / points as f64 - PI / 2.0,
            )
        })
        .collect();
    let order: Vec<Point> = (0..points).map(|i| anchors[(i * skip) % points]).collect();
    Ok(closed_from_vertices(&order))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Polygram {
    pub points: usize,
    pub skip: usize,
}

pub const PENTAGRAM: Polygram = Polygram { points: 5, skip: 2 };
pub const HEPTAGRAM: Polygram = Polygram { points: 7, skip: 2 };
pub const OCTAGRAM: Polygram = Polygram { points: 8, skip: 3 };

/// Equilateral triangle from one side: inscribed radius R = side / √3.
pub fn equilateral_triangle_shape(center: Point, side: f64, rotation: f64) -> Path {
    regular_polygon_shape(center, side / 3f64.sqrt(), 3, rotation).expect("3 sides")
}

/// Right-angled triangle; the right angle sits at `origin`.
pub fn right_triangle_shape(origin: Point, base: f64, height: f64) -> Path {
    let a = origin;
    let b = Point::new(origin.x + base, origin.y);
    let c = Point::new(origin.x, origin.y + height);
    Path::new(
        vec![
            Segment::line(a, b),
            Segment::line(b, c),
            Segment::line(c, a),
        ],
        true,
    )
}

pub fn right_triangle_hypotenuse(base: f64, height: f64) -> f64 {
    base.hypot(height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::distribute_open_path;
    use crate::test_util::assert_close;

    const O: Point = Point::ORIGIN;
    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn freehand_chains_points() {
        let path = freehand_shape(&[p(0.0, 0.0), p(3.0, 0.0), p(3.0, 4.0)]);
        assert!(!path.closed);
        assert_eq!(path.segments.len(), 2);
        assert_close(path.length(), 7.0, 1e-6);
    }

    #[test]
    fn freehand_single_point() {
        let path = freehand_shape(&[p(5.0, 5.0)]);
        assert_eq!(path.length(), 0.0);
        assert_eq!(
            distribute_open_path(&path, 10.0).unwrap(),
            vec![p(5.0, 5.0)]
        );
    }

    #[test]
    fn closed_polyline() {
        let path = closed_polyline_shape(&[p(0.0, 0.0), p(3.0, 0.0), p(3.0, 4.0)]);
        assert!(path.closed);
        assert_eq!(path.segments.len(), 3);
        assert_close(path.length(), 12.0, 1e-6);
        assert!(closed_polyline_shape(&[]).segments.is_empty());
        assert!(closed_polyline_shape(&[p(1.0, 1.0)]).segments.is_empty());
    }

    #[test]
    fn circle_true_circumference() {
        assert_close(circle_shape(O, 5.0).length(), 2.0 * PI * 5.0, 1e-6);
    }

    #[test]
    fn ellipse_ramanujan() {
        let (a, b) = (5.0f64, 3.0f64);
        let h = (a - b).powi(2) / (a + b).powi(2);
        let reference = PI * (a + b) * (1.0 + 3.0 * h / (10.0 + (4.0 - 3.0 * h).sqrt()));
        assert_close(ellipse_shape(O, a, b, 0.0).length(), reference, 0.005);
        assert_close(
            ellipse_shape(O, 4.0, 4.0, 0.0).length(),
            2.0 * PI * 4.0,
            1e-6,
        );
    }

    #[test]
    fn arc_semicircle_and_quarter() {
        let r = 5.0;
        assert_close(arc_shape(p(-r, 0.0), p(r, 0.0), r).length(), PI * r, 1e-6);
        let r = 10.0;
        let sagitta = r * (1.0 - (PI / 4.0).cos());
        assert_close(
            arc_shape(p(r, 0.0), p(0.0, r), sagitta).length(),
            r * PI / 2.0,
            1e-4,
        );
        assert_close(arc_shape(O, p(10.0, 0.0), 0.0).length(), 10.0, 1e-6);
    }

    #[test]
    fn rectangle_and_square_perimeter() {
        assert_close(rectangle_shape(O, 10.0, 6.0, 0.0).length(), 32.0, 1e-6);
        assert_close(square_shape(O, 5.0, 0.0).length(), 20.0, 1e-6);
    }

    #[test]
    fn regular_polygon() {
        let radius = 10.0;
        let side = 2.0 * radius * (PI / 8.0).sin();
        assert_close(
            regular_polygon_shape(O, radius, 8, 0.0).unwrap().length(),
            8.0 * side,
            1e-6,
        );
        assert_eq!(
            regular_polygon_shape(O, 10.0, 2, 0.0),
            Err(GeometryError::TooFewSides)
        );
    }

    #[test]
    fn star() {
        let path = star_shape(O, 10.0, 4.0, 5, 0.0).unwrap();
        assert_eq!(path.segments.len(), 10);
        assert!(star_shape(O, 10.0, 4.0, 2, 0.0).is_err());
    }

    #[test]
    fn polygram() {
        let penta = polygram_shape(O, 10.0, 5, 2, 0.0).unwrap();
        let hull = regular_polygon_shape(O, 10.0, 5, 0.0).unwrap();
        assert!(penta.length() > hull.length() + 0.05);
        assert_eq!(
            polygram_shape(O, 10.0, OCTAGRAM.points, OCTAGRAM.skip, 0.0)
                .unwrap()
                .segments
                .len(),
            8
        );
        assert!(polygram_shape(O, 10.0, 8, 2, 0.0).is_err());
    }

    #[test]
    fn triangles() {
        let path = equilateral_triangle_shape(O, 50.0, 0.0);
        assert_close(path.length(), 150.0, 1e-6);
        for s in &path.segments {
            assert_close(s.length(), 50.0, 1e-6);
        }
        assert_close(right_triangle_hypotenuse(40.0, 30.0), 50.0, 1e-6);
        assert_close(right_triangle_shape(O, 40.0, 30.0).length(), 120.0, 1e-6);
    }
}
