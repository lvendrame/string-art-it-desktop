//! Drag-to-create geometry for the Pin tools (spec 08). Port of web
//! `pinToolGeometry.ts`.

use sa_domain::Point;
use sa_domain::shapes::{HEPTAGRAM, OCTAGRAM, PENTAGRAM};

use crate::document::PinPathGeometry;
use crate::editor::PinTool;

const MIN_SIZE: f64 = 0.01;

/// Tools drawn with one press→release drag (everything but Arc/Freehand/Path/Text).
pub fn is_drag_tool(tool: PinTool) -> bool {
    use PinTool::*;
    matches!(
        tool,
        Line | Ellipse
            | Circle
            | Rectangle
            | Square
            | Pentagon
            | Hexagon
            | Octagon
            | Star5
            | Star6
            | Star8
            | Pentagram
            | Heptagram
            | Octagram
    )
}

/// Polygon/star family shown in the "Polygon / Star" picker.
pub const POLYGON_FAMILY: [PinTool; 9] = [
    PinTool::Pentagon,
    PinTool::Hexagon,
    PinTool::Octagon,
    PinTool::Star5,
    PinTool::Star6,
    PinTool::Star8,
    PinTool::Pentagram,
    PinTool::Heptagram,
    PinTool::Octagram,
];

fn radius_from(p0: Point, p1: Point) -> f64 {
    p0.distance(p1).max(MIN_SIZE)
}

fn normalize_box(p0: Point, p1: Point) -> (Point, f64, f64) {
    (
        Point::new(p0.x.min(p1.x), p0.y.min(p1.y)),
        (p1.x - p0.x).abs(),
        (p1.y - p0.y).abs(),
    )
}

/// Bounding-box tools drag corner→corner; centre-radius tools drag centre→out.
/// `alt_held` is the shape constraint modifier (Alt+Ellipse → circle,
/// Alt+Rectangle → square).
pub fn geometry_from_drag(
    tool: PinTool,
    p0: Point,
    p1: Point,
    alt_held: bool,
) -> Option<PinPathGeometry> {
    use PinPathGeometry as G;
    let rot = 0.0;
    Some(match tool {
        PinTool::Line => {
            if p0 == p1 {
                return None;
            }
            G::Line { start: p0, end: p1 }
        }
        PinTool::Ellipse => {
            let (pos, w, h) = normalize_box(p0, p1);
            let mut rx = (w / 2.0).max(MIN_SIZE);
            let mut ry = (h / 2.0).max(MIN_SIZE);
            if alt_held {
                rx = rx.min(ry);
                ry = rx;
            }
            G::Ellipse {
                center: Point::new(pos.x + w / 2.0, pos.y + h / 2.0),
                radius_x: rx,
                radius_y: ry,
                rotation: rot,
            }
        }
        PinTool::Circle => G::Circle {
            center: p0,
            radius: radius_from(p0, p1),
        },
        PinTool::Rectangle => {
            let (pos, mut w, mut h) = normalize_box(p0, p1);
            if alt_held {
                w = w.min(h);
                h = w;
            }
            G::Rectangle {
                position: pos,
                width: w.max(MIN_SIZE),
                height: h.max(MIN_SIZE),
                rotation: rot,
            }
        }
        PinTool::Square => {
            let (pos, w, h) = normalize_box(p0, p1);
            G::Square {
                position: pos,
                side: w.max(h).max(MIN_SIZE),
                rotation: rot,
            }
        }
        PinTool::Pentagon | PinTool::Hexagon | PinTool::Octagon => G::RegularPolygon {
            center: p0,
            radius: radius_from(p0, p1),
            sides: match tool {
                PinTool::Pentagon => 5,
                PinTool::Hexagon => 6,
                _ => 8,
            },
            rotation: rot,
        },
        PinTool::Star5 | PinTool::Star6 | PinTool::Star8 => {
            let outer = radius_from(p0, p1);
            G::Star {
                center: p0,
                outer_radius: outer,
                inner_radius: outer * 0.5,
                points: match tool {
                    PinTool::Star5 => 5,
                    PinTool::Star6 => 6,
                    _ => 8,
                },
                rotation: rot,
            }
        }
        PinTool::Pentagram | PinTool::Heptagram | PinTool::Octagram => {
            let g = match tool {
                PinTool::Pentagram => PENTAGRAM,
                PinTool::Heptagram => HEPTAGRAM,
                _ => OCTAGRAM,
            };
            G::Polygram {
                center: p0,
                radius: radius_from(p0, p1),
                points: g.points,
                skip: g.skip,
                rotation: rot,
            }
        }
        _ => return None,
    })
}

/// Signed sagitta: perpendicular distance of `cursor` from the chord midpoint.
pub fn curvature_from_cursor(start: Point, end: Point, cursor: Point) -> f64 {
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let chord = dx.hypot(dy);
    if chord == 0.0 {
        return 0.0;
    }
    let (nx, ny) = (-dy / chord, dx / chord);
    let mid = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    (cursor.x - mid.x) * nx + (cursor.y - mid.y) * ny
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn drag_shapes() {
        assert!(geometry_from_drag(PinTool::Line, p(1.0, 1.0), p(1.0, 1.0), false).is_none());
        let e = geometry_from_drag(PinTool::Ellipse, p(4.0, 2.0), p(0.0, 0.0), true).unwrap();
        assert_eq!(
            e,
            PinPathGeometry::Ellipse {
                center: p(2.0, 1.0),
                radius_x: 1.0,
                radius_y: 1.0,
                rotation: 0.0
            }
        );
        let r = geometry_from_drag(PinTool::Rectangle, p(0.0, 0.0), p(4.0, 2.0), true).unwrap();
        assert_eq!(
            r,
            PinPathGeometry::Rectangle {
                position: p(0.0, 0.0),
                width: 2.0,
                height: 2.0,
                rotation: 0.0
            }
        );
        let s = geometry_from_drag(PinTool::Square, p(0.0, 0.0), p(4.0, 2.0), false).unwrap();
        assert_eq!(
            s,
            PinPathGeometry::Square {
                position: p(0.0, 0.0),
                side: 4.0,
                rotation: 0.0
            }
        );
        let star = geometry_from_drag(PinTool::Star6, p(0.0, 0.0), p(3.0, 4.0), false).unwrap();
        assert_eq!(
            star,
            PinPathGeometry::Star {
                center: p(0.0, 0.0),
                outer_radius: 5.0,
                inner_radius: 2.5,
                points: 6,
                rotation: 0.0
            }
        );
        let c = geometry_from_drag(PinTool::Circle, p(1.0, 1.0), p(1.0, 1.0), false).unwrap();
        assert_eq!(
            c,
            PinPathGeometry::Circle {
                center: p(1.0, 1.0),
                radius: 0.01
            }
        );
        assert!(geometry_from_drag(PinTool::Arc, p(0.0, 0.0), p(1.0, 0.0), false).is_none());
    }

    #[test]
    fn curvature_sign() {
        assert_eq!(
            curvature_from_cursor(p(0.0, 0.0), p(10.0, 0.0), p(5.0, 3.0)),
            3.0
        );
        assert_eq!(
            curvature_from_cursor(p(0.0, 0.0), p(10.0, 0.0), p(5.0, -2.0)),
            -2.0
        );
        assert_eq!(
            curvature_from_cursor(p(1.0, 1.0), p(1.0, 1.0), p(5.0, -2.0)),
            0.0
        );
    }
}
