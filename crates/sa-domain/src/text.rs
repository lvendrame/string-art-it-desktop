//! Library-neutral glyph outline commands and their flattening into contours
//! (Text Pin Path, spec 29). Port of web `src/domain/text/glyphOutline.ts`.

use crate::paths::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GlyphCommand {
    MoveTo(Point),
    LineTo(Point),
    CubicTo(Point, Point, Point),
    QuadTo(Point, Point),
    Close,
}

fn cubic_point(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let mt = 1.0 - t;
    let (a, b, c, d) = (mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t);
    Point::new(
        a * p0.x + b * p1.x + c * p2.x + d * p3.x,
        a * p0.y + b * p1.y + c * p2.y + d * p3.y,
    )
}

fn quadratic_point(p0: Point, p1: Point, p2: Point, t: f64) -> Point {
    let mt = 1.0 - t;
    let (a, b, c) = (mt * mt, 2.0 * mt * t, t * t);
    Point::new(
        a * p0.x + b * p1.x + c * p2.x,
        a * p0.y + b * p1.y + c * p2.y,
    )
}

pub const DEFAULT_SEGMENTS_PER_CURVE: usize = 12;

/// Flattens drawing commands into per-contour point lists (a letter with a hole
/// produces several contours). Curves are subdivided into a fixed number of
/// straight segments.
pub fn commands_to_contours(
    commands: &[GlyphCommand],
    segments_per_curve: usize,
) -> Vec<Vec<Point>> {
    let mut contours = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let mut cursor = Point::ORIGIN;
    let close = |current: &mut Vec<Point>, contours: &mut Vec<Vec<Point>>| {
        if !current.is_empty() {
            contours.push(std::mem::take(current));
        }
    };
    for cmd in commands {
        match *cmd {
            GlyphCommand::MoveTo(p) => {
                close(&mut current, &mut contours);
                cursor = p;
                current.push(p);
            }
            GlyphCommand::LineTo(p) => {
                cursor = p;
                current.push(p);
            }
            GlyphCommand::CubicTo(p1, p2, p3) => {
                for i in 1..=segments_per_curve {
                    current.push(cubic_point(
                        cursor,
                        p1,
                        p2,
                        p3,
                        i as f64 / segments_per_curve as f64,
                    ));
                }
                cursor = p3;
            }
            GlyphCommand::QuadTo(p1, p2) => {
                for i in 1..=segments_per_curve {
                    current.push(quadratic_point(
                        cursor,
                        p1,
                        p2,
                        i as f64 / segments_per_curve as f64,
                    ));
                }
                cursor = p2;
            }
            GlyphCommand::Close => close(&mut current, &mut contours),
        }
    }
    close(&mut current, &mut contours);
    contours
}

#[cfg(test)]
mod tests {
    use super::GlyphCommand::*;
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn square_one_contour() {
        let c = commands_to_contours(
            &[
                MoveTo(p(0.0, 0.0)),
                LineTo(p(10.0, 0.0)),
                LineTo(p(10.0, 10.0)),
                LineTo(p(0.0, 10.0)),
                Close,
            ],
            DEFAULT_SEGMENTS_PER_CURVE,
        );
        assert_eq!(
            c,
            vec![vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)]]
        );
    }

    #[test]
    fn ring_with_hole_two_contours() {
        let c = commands_to_contours(
            &[
                MoveTo(p(0.0, 0.0)),
                LineTo(p(10.0, 0.0)),
                LineTo(p(10.0, 10.0)),
                Close,
                MoveTo(p(2.0, 2.0)),
                LineTo(p(8.0, 2.0)),
                LineTo(p(8.0, 8.0)),
                Close,
            ],
            DEFAULT_SEGMENTS_PER_CURVE,
        );
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].len(), 3);
        assert_eq!(c[1].len(), 3);
    }

    #[test]
    fn curves_subdivide() {
        let c = commands_to_contours(
            &[
                MoveTo(p(0.0, 0.0)),
                CubicTo(p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)),
                Close,
            ],
            4,
        );
        assert_eq!(c[0].len(), 5);
        assert!(c[0].iter().any(|q| q.y > 0.1));
        assert_eq!(*c[0].last().unwrap(), p(10.0, 0.0));
        let c = commands_to_contours(
            &[
                MoveTo(p(0.0, 0.0)),
                QuadTo(p(5.0, 10.0), p(10.0, 0.0)),
                Close,
            ],
            4,
        );
        assert_eq!(c[0].len(), 5);
        assert!(c[0].iter().any(|q| q.y > 0.1));
    }

    #[test]
    fn missing_close_and_empty() {
        let c = commands_to_contours(
            &[
                MoveTo(p(0.0, 0.0)),
                LineTo(p(1.0, 0.0)),
                LineTo(p(1.0, 1.0)),
            ],
            12,
        );
        assert_eq!(c.len(), 1);
        assert!(commands_to_contours(&[], 12).is_empty());
    }
}
