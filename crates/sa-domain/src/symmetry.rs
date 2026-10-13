//! Mirror and radial symmetry copies (spec 06). Port of web `src/domain/symmetry/*`.

use serde::{Deserialize, Serialize};

use crate::GeometryError;
use crate::paths::Point;

pub fn reflect_vertical(point: Point, axis_x: f64) -> Point {
    Point::new(2.0 * axis_x - point.x, point.y)
}

pub fn reflect_horizontal(point: Point, axis_y: f64) -> Point {
    Point::new(point.x, 2.0 * axis_y - point.y)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MirrorMode {
    None,
    Vertical,
    Horizontal,
    Both,
}

/// Spec 06 §9.1: "both" produces three reflected copies — V, H and VH.
pub fn mirror_copies(points: &[Point], mode: MirrorMode, axis: Point) -> Vec<Vec<Point>> {
    let v = || {
        points
            .iter()
            .map(|&p| reflect_vertical(p, axis.x))
            .collect()
    };
    let h = || {
        points
            .iter()
            .map(|&p| reflect_horizontal(p, axis.y))
            .collect()
    };
    let vh = || {
        points
            .iter()
            .map(|&p| reflect_horizontal(reflect_vertical(p, axis.x), axis.y))
            .collect()
    };
    match mode {
        MirrorMode::None => vec![],
        MirrorMode::Vertical => vec![v()],
        MirrorMode::Horizontal => vec![h()],
        MirrorMode::Both => vec![v(), h(), vh()],
    }
}

pub fn rotate_around(point: Point, center: Point, radians: f64) -> Point {
    crate::transforms::rotate_point(point, center, radians)
}

/// Spec 06 §9.2: one array per copy, excluding the source — interval 45 → 7 copies.
pub fn generate_radial_copies(
    points: &[Point],
    center: Point,
    interval_degrees: f64,
) -> Result<Vec<Vec<Point>>, GeometryError> {
    if interval_degrees <= 0.0 {
        return Err(GeometryError::NonPositiveRadialInterval);
    }
    let mut copies = Vec::new();
    let mut angle_deg = interval_degrees;
    while angle_deg < 360.0 {
        let radians = angle_deg.to_radians();
        copies.push(
            points
                .iter()
                .map(|&p| rotate_around(p, center, radians))
                .collect(),
        );
        angle_deg += interval_degrees;
    }
    Ok(copies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_close;

    const AXIS: Point = Point::ORIGIN;
    const PTS: [Point; 1] = [Point::new(3.0, 4.0)];

    #[test]
    fn mirror_modes() {
        assert_eq!(
            mirror_copies(&PTS, MirrorMode::Vertical, AXIS),
            vec![vec![Point::new(-3.0, 4.0)]]
        );
        assert_eq!(
            mirror_copies(&PTS, MirrorMode::Horizontal, AXIS),
            vec![vec![Point::new(3.0, -4.0)]]
        );
        let both = mirror_copies(&PTS, MirrorMode::Both, AXIS);
        assert_eq!(both.len(), 3);
        assert!(both.contains(&vec![Point::new(-3.0, -4.0)]));
        assert!(mirror_copies(&PTS, MirrorMode::None, AXIS).is_empty());
    }

    #[test]
    fn radial_copies() {
        let src = [Point::new(10.0, 0.0)];
        assert_eq!(generate_radial_copies(&src, AXIS, 45.0).unwrap().len(), 7);
        let copies = generate_radial_copies(&src, AXIS, 37.0).unwrap();
        assert_eq!(copies.len(), (360.0f64 / 37.0).ceil() as usize - 1);
        for c in &copies {
            assert!(!(c[0].x == 10.0 && c[0].y.abs() < 1e-9));
        }
        assert!(generate_radial_copies(&src, AXIS, 0.0).is_err());
        assert!(generate_radial_copies(&src, AXIS, -10.0).is_err());
    }

    #[test]
    fn rotate_90() {
        let r = rotate_around(Point::new(1.0, 0.0), AXIS, std::f64::consts::FRAC_PI_2);
        assert_close(r.x, 0.0, 1e-6);
        assert_close(r.y, 1.0, 1e-6);
    }
}
