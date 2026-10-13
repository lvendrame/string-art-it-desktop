//! Backend-neutral path data. Arcs are converted to cubic Béziers so every
//! backend (tiny-skia, krilla, SVG, egui) consumes the same commands.

use sa_domain::paths::Segment;
use sa_domain::{Path, Point};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCmd {
    MoveTo(Point),
    LineTo(Point),
    CubicTo(Point, Point, Point),
    Close,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathData {
    pub cmds: Vec<PathCmd>,
}

impl PathData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, p: Point) {
        self.cmds.push(PathCmd::MoveTo(p));
    }

    pub fn line_to(&mut self, p: Point) {
        self.cmds.push(PathCmd::LineTo(p));
    }

    pub fn close(&mut self) {
        self.cmds.push(PathCmd::Close);
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    pub fn polyline(points: &[Point], closed: bool) -> Self {
        let mut d = Self::new();
        for (i, p) in points.iter().enumerate() {
            if i == 0 { d.move_to(*p) } else { d.line_to(*p) }
        }
        if closed && !points.is_empty() {
            d.close();
        }
        d
    }

    /// Appends a domain [`Path`] as a new subpath.
    pub fn append_path(&mut self, path: &Path) {
        let Some(first) = path.segments.first() else {
            return;
        };
        self.move_to(first.point_at_distance(0.0));
        for seg in &path.segments {
            match seg {
                Segment::Line { end, .. } => self.line_to(*end),
                Segment::CircularArc {
                    center,
                    radius,
                    start_angle,
                    sweep,
                } => {
                    append_elliptic_arc(self, *center, *radius, *radius, 0.0, *start_angle, *sweep);
                }
                Segment::EllipticalArc(arc) => append_elliptic_arc(
                    self,
                    arc.center,
                    arc.radius_x,
                    arc.radius_y,
                    arc.rotation,
                    arc.start_angle,
                    arc.sweep,
                ),
            }
        }
        if path.closed {
            self.close();
        }
    }

    pub fn from_paths<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Self {
        let mut d = Self::new();
        for p in paths {
            d.append_path(p);
        }
        d
    }

    /// Flattens to polylines (one per subpath) with `tolerance` cm.
    pub fn flatten(&self, tolerance: f64) -> Vec<(Vec<Point>, bool)> {
        let mut out: Vec<(Vec<Point>, bool)> = Vec::new();
        let mut current: Vec<Point> = Vec::new();
        for cmd in &self.cmds {
            match *cmd {
                PathCmd::MoveTo(p) => {
                    if current.len() > 1 {
                        out.push((std::mem::take(&mut current), false));
                    }
                    current.clear();
                    current.push(p);
                }
                PathCmd::LineTo(p) => current.push(p),
                PathCmd::CubicTo(c1, c2, p) => {
                    let p0 = *current.last().unwrap_or(&p);
                    let len = p0.distance(c1) + c1.distance(c2) + c2.distance(p);
                    let n = ((len / tolerance.max(1e-6)).sqrt().ceil() as usize).clamp(1, 256);
                    for i in 1..=n {
                        current.push(cubic_at(p0, c1, c2, p, i as f64 / n as f64));
                    }
                }
                PathCmd::Close => {
                    if current.len() > 1 {
                        out.push((std::mem::take(&mut current), true));
                    }
                    current.clear();
                }
            }
        }
        if current.len() > 1 {
            out.push((current, false));
        }
        out
    }
}

pub fn cubic_at(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let mt = 1.0 - t;
    let (a, b, c, d) = (mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t);
    Point::new(
        a * p0.x + b * p1.x + c * p2.x + d * p3.x,
        a * p0.y + b * p1.y + c * p2.y + d * p3.y,
    )
}

/// Appends an elliptical arc as cubic segments of at most 90°.
fn append_elliptic_arc(
    d: &mut PathData,
    center: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    start: f64,
    sweep: f64,
) {
    if sweep == 0.0 {
        return;
    }
    let n = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = sweep / n as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let (rs, rc) = rotation.sin_cos();
    let map = |x: f64, y: f64| Point::new(center.x + x * rc - y * rs, center.y + x * rs + y * rc);
    for i in 0..n {
        let a0 = start + step * i as f64;
        let a1 = a0 + step;
        let (s0, c0) = a0.sin_cos();
        let (s1, c1) = a1.sin_cos();
        let p1 = map(rx * (c0 - k * s0), ry * (s0 + k * c0));
        let p2 = map(rx * (c1 + k * s1), ry * (s1 - k * c1));
        let p3 = map(rx * c1, ry * s1);
        d.cmds.push(PathCmd::CubicTo(p1, p2, p3));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_domain::shapes::{circle_shape, rectangle_shape};

    #[test]
    fn circle_becomes_four_cubics_close_to_true_radius() {
        let d = PathData::from_paths([&circle_shape(Point::ORIGIN, 10.0)]);
        assert_eq!(
            d.cmds
                .iter()
                .filter(|c| matches!(c, PathCmd::CubicTo(..)))
                .count(),
            4
        );
        for (pts, closed) in d.flatten(0.001) {
            assert!(closed);
            for p in pts {
                let r = p.x.hypot(p.y);
                assert!((r - 10.0).abs() < 0.01, "r={r}");
            }
        }
    }

    #[test]
    fn rectangle_lines() {
        let d = PathData::from_paths([&rectangle_shape(Point::ORIGIN, 2.0, 1.0, 0.0)]);
        assert_eq!(d.cmds.len(), 6); // M + 4 L + Z
    }
}
