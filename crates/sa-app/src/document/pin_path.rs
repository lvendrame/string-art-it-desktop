//! Pin Paths: parametric geometry + derived pins (specs 07, 08, 29, 33).
//! Port of web `pinPath.ts`.

use serde::{Deserialize, Serialize};

use sa_domain::paths::{distribute_closed_path, distribute_open_path, distribute_path_per_vertex};
use sa_domain::shapes::{
    arc_shape, circle_shape, closed_polyline_shape, ellipse_shape, freehand_shape, line_shape,
    polygram_shape, rectangle_shape, regular_polygon_shape, square_shape, star_shape,
};
use sa_domain::transforms::{rotate_point, scale_point, translate_point};
use sa_domain::{Path, Point};

use super::symmetry::SymmetryConfig;
use crate::ids::next_id;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    #[default]
    Regular,
    Bold,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PinPathGeometry {
    Line {
        start: Point,
        end: Point,
    },
    Arc {
        start: Point,
        end: Point,
        curvature: f64,
    },
    #[serde(rename_all = "camelCase")]
    Ellipse {
        center: Point,
        radius_x: f64,
        radius_y: f64,
        rotation: f64,
    },
    Circle {
        center: Point,
        radius: f64,
    },
    Rectangle {
        position: Point,
        width: f64,
        height: f64,
        rotation: f64,
    },
    Square {
        position: Point,
        side: f64,
        rotation: f64,
    },
    RegularPolygon {
        center: Point,
        radius: f64,
        sides: usize,
        rotation: f64,
    },
    #[serde(rename_all = "camelCase")]
    Star {
        center: Point,
        outer_radius: f64,
        inner_radius: f64,
        points: usize,
        rotation: f64,
    },
    Polygram {
        center: Point,
        radius: f64,
        points: usize,
        skip: usize,
        rotation: f64,
    },
    Freehand {
        points: Vec<Point>,
    },
    /// Spec 33 Path tool: deliberately clicked corners, always closed.
    Polygon {
        points: Vec<Point>,
    },
    /// Spec 29. `contours` is a derived cache of the flattened, positioned glyph
    /// outlines, produced by the infra font layer whenever text fields change.
    #[serde(rename_all = "camelCase")]
    Text {
        origin: Point,
        text: String,
        font_id: String,
        weight: FontWeight,
        italic: bool,
        size: f64,
        letter_spacing: f64,
        rotation: f64,
        contours: Vec<Vec<Point>>,
    },
}

/// Discriminant without payload — used for tool/vertex-anchoring checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeometryKind {
    Line,
    Arc,
    Ellipse,
    Circle,
    Rectangle,
    Square,
    RegularPolygon,
    Star,
    Polygram,
    Freehand,
    Polygon,
    Text,
}

impl GeometryKind {
    /// Straight-edged shapes whose vertices get pins (spec 07 §Vertex-Anchored).
    pub fn is_vertex_anchored(self) -> bool {
        matches!(
            self,
            GeometryKind::Line
                | GeometryKind::Rectangle
                | GeometryKind::Square
                | GeometryKind::RegularPolygon
                | GeometryKind::Star
                | GeometryKind::Polygram
                | GeometryKind::Polygon
        )
    }
}

impl PinPathGeometry {
    pub fn kind(&self) -> GeometryKind {
        use PinPathGeometry as G;
        match self {
            G::Line { .. } => GeometryKind::Line,
            G::Arc { .. } => GeometryKind::Arc,
            G::Ellipse { .. } => GeometryKind::Ellipse,
            G::Circle { .. } => GeometryKind::Circle,
            G::Rectangle { .. } => GeometryKind::Rectangle,
            G::Square { .. } => GeometryKind::Square,
            G::RegularPolygon { .. } => GeometryKind::RegularPolygon,
            G::Star { .. } => GeometryKind::Star,
            G::Polygram { .. } => GeometryKind::Polygram,
            G::Freehand { .. } => GeometryKind::Freehand,
            G::Polygon { .. } => GeometryKind::Polygon,
            G::Text { .. } => GeometryKind::Text,
        }
    }

    /// Every shape as a list of contour paths: one for single-contour shapes, one
    /// closed polyline per flattened contour for text. Invalid parameters (e.g. a
    /// polygram with gcd ≠ 1 loaded from a file) yield no contours.
    pub fn contour_paths(&self) -> Vec<Path> {
        use PinPathGeometry as G;
        let single = |p: Path| vec![p];
        match self {
            G::Line { start, end } => single(line_shape(*start, *end)),
            G::Arc {
                start,
                end,
                curvature,
            } => single(arc_shape(*start, *end, *curvature)),
            G::Ellipse {
                center,
                radius_x,
                radius_y,
                rotation,
            } => single(ellipse_shape(*center, *radius_x, *radius_y, *rotation)),
            G::Circle { center, radius } => single(circle_shape(*center, *radius)),
            G::Rectangle {
                position,
                width,
                height,
                rotation,
            } => single(rectangle_shape(*position, *width, *height, *rotation)),
            G::Square {
                position,
                side,
                rotation,
            } => single(square_shape(*position, *side, *rotation)),
            G::RegularPolygon {
                center,
                radius,
                sides,
                rotation,
            } => regular_polygon_shape(*center, *radius, *sides, *rotation)
                .into_iter()
                .collect(),
            G::Star {
                center,
                outer_radius,
                inner_radius,
                points,
                rotation,
            } => star_shape(*center, *outer_radius, *inner_radius, *points, *rotation)
                .into_iter()
                .collect(),
            G::Polygram {
                center,
                radius,
                points,
                skip,
                rotation,
            } => polygram_shape(*center, *radius, *points, *skip, *rotation)
                .into_iter()
                .collect(),
            G::Freehand { points } => single(freehand_shape(points)),
            G::Polygon { points } => single(closed_polyline_shape(points)),
            G::Text { contours, .. } => contours.iter().map(|c| closed_polyline_shape(c)).collect(),
        }
    }

    /// Move tool: shifts every point field; rotation fields are untouched.
    pub fn translated(&self, delta: Point) -> Self {
        let t = |p: Point| translate_point(p, delta);
        self.map_points(t, |g| g)
    }

    /// Rotation tool: rotates by `theta` about an arbitrary external pivot.
    pub fn rotated(&self, pivot: Point, theta: f64) -> Self {
        use PinPathGeometry as G;
        let r = |p: Point| rotate_point(p, pivot, theta);
        match self.clone() {
            G::Line { start, end } => G::Line {
                start: r(start),
                end: r(end),
            },
            G::Arc {
                start,
                end,
                curvature,
            } => G::Arc {
                start: r(start),
                end: r(end),
                curvature,
            },
            G::Circle { center, radius } => G::Circle {
                center: r(center),
                radius,
            },
            G::Ellipse {
                center,
                radius_x,
                radius_y,
                rotation,
            } => G::Ellipse {
                center: r(center),
                radius_x,
                radius_y,
                rotation: rotation + theta,
            },
            G::RegularPolygon {
                center,
                radius,
                sides,
                rotation,
            } => G::RegularPolygon {
                center: r(center),
                radius,
                sides,
                rotation: rotation + theta,
            },
            G::Star {
                center,
                outer_radius,
                inner_radius,
                points,
                rotation,
            } => G::Star {
                center: r(center),
                outer_radius,
                inner_radius,
                points,
                rotation: rotation + theta,
            },
            G::Polygram {
                center,
                radius,
                points,
                skip,
                rotation,
            } => G::Polygram {
                center: r(center),
                radius,
                points,
                skip,
                rotation: rotation + theta,
            },
            // `position` is centre − halfSize in the unrotated frame: rotate the
            // derived centre, then re-derive position.
            G::Rectangle {
                position,
                width,
                height,
                rotation,
            } => {
                let c = r(Point::new(
                    position.x + width / 2.0,
                    position.y + height / 2.0,
                ));
                G::Rectangle {
                    position: Point::new(c.x - width / 2.0, c.y - height / 2.0),
                    width,
                    height,
                    rotation: rotation + theta,
                }
            }
            G::Square {
                position,
                side,
                rotation,
            } => {
                let c = r(Point::new(position.x + side / 2.0, position.y + side / 2.0));
                G::Square {
                    position: Point::new(c.x - side / 2.0, c.y - side / 2.0),
                    side,
                    rotation: rotation + theta,
                }
            }
            G::Freehand { points } => G::Freehand {
                points: points.into_iter().map(r).collect(),
            },
            G::Polygon { points } => G::Polygon {
                points: points.into_iter().map(r).collect(),
            },
            G::Text {
                origin,
                text,
                font_id,
                weight,
                italic,
                size,
                letter_spacing,
                rotation,
                contours,
            } => G::Text {
                origin: r(origin),
                text,
                font_id,
                weight,
                italic,
                size,
                letter_spacing,
                rotation: rotation + theta,
                contours: contours
                    .into_iter()
                    .map(|c| c.into_iter().map(r).collect())
                    .collect(),
            },
        }
    }

    /// The shape's own centroid — pivot for Scale and keyboard Rotate/Scale.
    pub fn center(&self) -> Point {
        use PinPathGeometry as G;
        let mean = |pts: &mut dyn Iterator<Item = &Point>| {
            let (mut sx, mut sy, mut n) = (0.0, 0.0, 0usize);
            for p in pts {
                sx += p.x;
                sy += p.y;
                n += 1;
            }
            (n > 0).then(|| Point::new(sx / n as f64, sy / n as f64))
        };
        match self {
            G::Line { start, end } | G::Arc { start, end, .. } => {
                Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0)
            }
            G::Ellipse { center, .. }
            | G::Circle { center, .. }
            | G::RegularPolygon { center, .. }
            | G::Star { center, .. }
            | G::Polygram { center, .. } => *center,
            G::Rectangle {
                position,
                width,
                height,
                ..
            } => Point::new(position.x + width / 2.0, position.y + height / 2.0),
            G::Square { position, side, .. } => {
                Point::new(position.x + side / 2.0, position.y + side / 2.0)
            }
            // Web divides by length unguarded (NaN for an empty list); fall back to origin.
            G::Freehand { points } | G::Polygon { points } => {
                mean(&mut points.iter()).unwrap_or(Point::ORIGIN)
            }
            G::Text {
                origin, contours, ..
            } => mean(&mut contours.iter().flatten()).unwrap_or(*origin),
        }
    }

    /// Scale tool: resizes about the shape's own centroid.
    pub fn scaled(&self, factor: f64) -> Self {
        self.scaled_about(self.center(), factor)
    }

    /// General Scale about an arbitrary pivot (multi-select, spec 26).
    pub fn scaled_about(&self, pivot: Point, factor: f64) -> Self {
        use PinPathGeometry as G;
        let s = |p: Point| scale_point(p, pivot, factor);
        match self.clone() {
            G::Line { start, end } => G::Line {
                start: s(start),
                end: s(end),
            },
            G::Arc {
                start,
                end,
                curvature,
            } => G::Arc {
                start: s(start),
                end: s(end),
                curvature: curvature * factor,
            },
            G::Circle { center, radius } => G::Circle {
                center: s(center),
                radius: radius * factor,
            },
            G::Ellipse {
                center,
                radius_x,
                radius_y,
                rotation,
            } => G::Ellipse {
                center: s(center),
                radius_x: radius_x * factor,
                radius_y: radius_y * factor,
                rotation,
            },
            G::Rectangle {
                position,
                width,
                height,
                rotation,
            } => {
                let (w, h) = (width * factor, height * factor);
                let c = s(Point::new(
                    position.x + width / 2.0,
                    position.y + height / 2.0,
                ));
                G::Rectangle {
                    position: Point::new(c.x - w / 2.0, c.y - h / 2.0),
                    width: w,
                    height: h,
                    rotation,
                }
            }
            G::Square {
                position,
                side,
                rotation,
            } => {
                let ns = side * factor;
                let c = s(Point::new(position.x + side / 2.0, position.y + side / 2.0));
                G::Square {
                    position: Point::new(c.x - ns / 2.0, c.y - ns / 2.0),
                    side: ns,
                    rotation,
                }
            }
            G::RegularPolygon {
                center,
                radius,
                sides,
                rotation,
            } => G::RegularPolygon {
                center: s(center),
                radius: radius * factor,
                sides,
                rotation,
            },
            G::Star {
                center,
                outer_radius,
                inner_radius,
                points,
                rotation,
            } => G::Star {
                center: s(center),
                outer_radius: outer_radius * factor,
                inner_radius: inner_radius * factor,
                points,
                rotation,
            },
            G::Polygram {
                center,
                radius,
                points,
                skip,
                rotation,
            } => G::Polygram {
                center: s(center),
                radius: radius * factor,
                points,
                skip,
                rotation,
            },
            other => other.map_points(s, |g| g),
        }
    }

    /// Applies `f` to every point field (rotations/sizes untouched), then `post`.
    fn map_points(&self, f: impl Fn(Point) -> Point, post: impl FnOnce(Self) -> Self) -> Self {
        use PinPathGeometry as G;
        let g = match self.clone() {
            G::Line { start, end } => G::Line {
                start: f(start),
                end: f(end),
            },
            G::Arc {
                start,
                end,
                curvature,
            } => G::Arc {
                start: f(start),
                end: f(end),
                curvature,
            },
            G::Ellipse {
                center,
                radius_x,
                radius_y,
                rotation,
            } => G::Ellipse {
                center: f(center),
                radius_x,
                radius_y,
                rotation,
            },
            G::Circle { center, radius } => G::Circle {
                center: f(center),
                radius,
            },
            G::Rectangle {
                position,
                width,
                height,
                rotation,
            } => G::Rectangle {
                position: f(position),
                width,
                height,
                rotation,
            },
            G::Square {
                position,
                side,
                rotation,
            } => G::Square {
                position: f(position),
                side,
                rotation,
            },
            G::RegularPolygon {
                center,
                radius,
                sides,
                rotation,
            } => G::RegularPolygon {
                center: f(center),
                radius,
                sides,
                rotation,
            },
            G::Star {
                center,
                outer_radius,
                inner_radius,
                points,
                rotation,
            } => G::Star {
                center: f(center),
                outer_radius,
                inner_radius,
                points,
                rotation,
            },
            G::Polygram {
                center,
                radius,
                points,
                skip,
                rotation,
            } => G::Polygram {
                center: f(center),
                radius,
                points,
                skip,
                rotation,
            },
            G::Freehand { points } => G::Freehand {
                points: points.into_iter().map(&f).collect(),
            },
            G::Polygon { points } => G::Polygon {
                points: points.into_iter().map(&f).collect(),
            },
            G::Text {
                origin,
                text,
                font_id,
                weight,
                italic,
                size,
                letter_spacing,
                rotation,
                contours,
            } => G::Text {
                origin: f(origin),
                text,
                font_id,
                weight,
                italic,
                size,
                letter_spacing,
                rotation,
                contours: contours
                    .into_iter()
                    .map(|c| c.into_iter().map(&f).collect())
                    .collect(),
            },
        };
        post(g)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    pub id: String,
    pub x: f64,
    pub y: f64,
}

impl Pin {
    pub fn point(&self) -> Point {
        Point::new(self.x, self.y)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinStyle {
    pub colour: String,
    pub diameter: f64,
    pub guide_visible: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinPath {
    pub id: String,
    pub geometry: PinPathGeometry,
    pub requested_spacing: f64,
    pub actual_spacing: f64,
    pub pins: Vec<Pin>,
    pub guide_visible: bool,
    pub colour: String,
    pub diameter: f64,
    pub symmetry: SymmetryConfig,
}

pub fn next_pin_id() -> String {
    next_id("pin")
}

pub fn next_path_id() -> String {
    next_id("pinpath")
}

/// Distributes pins along `geometry`. Non-positive spacing yields no pins (the
/// UI clamps spacing; the web version throws).
pub fn distribute_pins(geometry: &PinPathGeometry, requested_spacing: f64) -> (Vec<Pin>, f64) {
    let to_pins = |pts: Vec<Point>| -> Vec<Pin> {
        pts.into_iter()
            .map(|p| Pin {
                id: next_pin_id(),
                x: p.x,
                y: p.y,
            })
            .collect()
    };
    // Spec 29: each glyph contour is a closed curved path, spaced independently.
    if let PinPathGeometry::Text { contours, .. } = geometry {
        let mut pins = Vec::new();
        let mut actual = requested_spacing;
        for (i, contour) in contours.iter().enumerate() {
            if contour.len() < 2 {
                continue;
            }
            if let Ok(d) =
                distribute_closed_path(&closed_polyline_shape(contour), requested_spacing)
            {
                if i == 0 {
                    actual = d.actual_spacing;
                }
                pins.extend(to_pins(d.points));
            }
        }
        return (pins, actual);
    }
    let Some(path) = geometry.contour_paths().into_iter().next() else {
        return (Vec::new(), requested_spacing);
    };
    if geometry.kind().is_vertex_anchored() {
        return match distribute_path_per_vertex(&path, requested_spacing) {
            Ok(d) => (to_pins(d.points), d.actual_spacing),
            Err(_) => (Vec::new(), requested_spacing),
        };
    }
    if path.closed {
        return match distribute_closed_path(&path, requested_spacing) {
            Ok(d) => (to_pins(d.points), d.actual_spacing),
            Err(_) => (Vec::new(), requested_spacing),
        };
    }
    match distribute_open_path(&path, requested_spacing) {
        Ok(pts) => (to_pins(pts), requested_spacing),
        Err(_) => (Vec::new(), requested_spacing),
    }
}

impl PinPath {
    pub fn new(
        geometry: PinPathGeometry,
        requested_spacing: f64,
        style: &PinStyle,
        symmetry: SymmetryConfig,
    ) -> Self {
        let (pins, actual_spacing) = distribute_pins(&geometry, requested_spacing);
        Self {
            id: next_path_id(),
            geometry,
            requested_spacing,
            actual_spacing,
            pins,
            guide_visible: style.guide_visible,
            colour: style.colour.clone(),
            diameter: style.diameter,
            symmetry,
        }
    }

    /// Any geometry or spacing edit recalculates pins (spec 09).
    pub fn recomputed(&self) -> Self {
        let (pins, actual_spacing) = distribute_pins(&self.geometry, self.requested_spacing);
        Self {
            pins,
            actual_spacing,
            ..self.clone()
        }
    }

    /// Duplicate with fresh path and pin ids (spec 13: duplicates never inherit
    /// thread connections).
    pub fn cloned_fresh(&self) -> Self {
        Self {
            id: next_path_id(),
            pins: self
                .pins
                .iter()
                .map(|p| Pin {
                    id: next_pin_id(),
                    ..p.clone()
                })
                .collect(),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    pub(crate) fn style() -> PinStyle {
        PinStyle {
            colour: "#fff".into(),
            diameter: 2.0,
            guide_visible: true,
        }
    }

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn line_is_vertex_anchored_with_both_endpoints() {
        let path = PinPath::new(
            PinPathGeometry::Line {
                start: p(0.0, 0.0),
                end: p(7.5, 0.0),
            },
            1.0,
            &style(),
            SymmetryConfig::None,
        );
        assert_eq!(path.pins.len(), 9);
        assert_eq!(path.pins.last().unwrap().point(), p(7.5, 0.0));
    }

    #[test]
    fn circle_closed_distribution() {
        let path = PinPath::new(
            PinPathGeometry::Circle {
                center: p(0.0, 0.0),
                radius: 31.0 / (2.0 * PI),
            },
            2.0,
            &style(),
            SymmetryConfig::None,
        );
        assert_eq!(path.pins.len(), 16);
        assert!((path.actual_spacing - 1.9375).abs() < 1e-4);
    }

    #[test]
    fn arc_open_distribution_keeps_requested_spacing() {
        let path = PinPath::new(
            PinPathGeometry::Arc {
                start: p(-5.0, 0.0),
                end: p(5.0, 0.0),
                curvature: 5.0,
            },
            1.0,
            &style(),
            SymmetryConfig::None,
        );
        assert_eq!(path.pins.len(), (5.0 * PI).floor() as usize + 1);
        assert_eq!(path.actual_spacing, 1.0);
    }

    #[test]
    fn text_contours_distribute_independently() {
        let square = |o: f64, s: f64| vec![p(o, o), p(o + s, o), p(o + s, o + s), p(o, o + s)];
        let g = PinPathGeometry::Text {
            origin: p(0.0, 0.0),
            text: "o".into(),
            font_id: "pt-sans".into(),
            weight: FontWeight::Regular,
            italic: false,
            size: 10.0,
            letter_spacing: 0.0,
            rotation: 0.0,
            contours: vec![square(0.0, 4.0), square(1.0, 2.0), vec![p(9.0, 9.0)]],
        };
        let (pins, actual) = distribute_pins(&g, 1.0);
        assert_eq!(pins.len(), 16 + 8);
        assert_eq!(actual, 1.0);
    }

    #[test]
    fn ids_are_unique() {
        let a = PinPath::new(
            PinPathGeometry::Circle {
                center: p(0.0, 0.0),
                radius: 5.0,
            },
            1.0,
            &style(),
            SymmetryConfig::None,
        );
        let b = a.cloned_fresh();
        assert_ne!(a.id, b.id);
        assert!(
            a.pins
                .iter()
                .zip(&b.pins)
                .all(|(x, y)| x.id != y.id && x.x == y.x)
        );
    }

    #[test]
    fn rotate_rectangle_about_external_pivot_keeps_centre_correct() {
        let g = PinPathGeometry::Rectangle {
            position: p(0.0, 0.0),
            width: 4.0,
            height: 2.0,
            rotation: 0.0,
        };
        let r = g.rotated(p(10.0, 0.0), PI);
        let c = r.center();
        assert!(
            (c.x - 18.0).abs() < 1e-9 && (c.y + 1.0).abs() < 1e-9,
            "{c:?}"
        );
        let PinPathGeometry::Rectangle { rotation, .. } = r else {
            unreachable!()
        };
        assert!((rotation - PI).abs() < 1e-12);
    }

    #[test]
    fn scale_keeps_centroid_and_scales_curvature() {
        let g = PinPathGeometry::Arc {
            start: p(0.0, 0.0),
            end: p(10.0, 0.0),
            curvature: 2.0,
        };
        let s = g.scaled(2.0);
        assert_eq!(s.center(), g.center());
        assert_eq!(
            s,
            PinPathGeometry::Arc {
                start: p(-5.0, 0.0),
                end: p(15.0, 0.0),
                curvature: 4.0
            }
        );
        let sq = PinPathGeometry::Square {
            position: p(0.0, 0.0),
            side: 2.0,
            rotation: 0.3,
        }
        .scaled(3.0);
        assert_eq!(
            sq,
            PinPathGeometry::Square {
                position: p(-2.0, -2.0),
                side: 6.0,
                rotation: 0.3
            }
        );
    }

    #[test]
    fn translate_moves_points_only() {
        let g = PinPathGeometry::Star {
            center: p(1.0, 1.0),
            outer_radius: 5.0,
            inner_radius: 2.0,
            points: 5,
            rotation: 0.5,
        };
        assert_eq!(
            g.translated(p(2.0, -1.0)),
            PinPathGeometry::Star {
                center: p(3.0, 0.0),
                outer_radius: 5.0,
                inner_radius: 2.0,
                points: 5,
                rotation: 0.5
            }
        );
    }

    #[test]
    fn json_shape_matches_web() {
        let g: PinPathGeometry = serde_json::from_str(
            r#"{"type":"regular-polygon","center":{"x":0,"y":0},"radius":5,"sides":6,"rotation":0}"#,
        )
        .unwrap();
        assert_eq!(g.kind(), GeometryKind::RegularPolygon);
        let e = serde_json::to_value(PinPathGeometry::Ellipse {
            center: p(0.0, 0.0),
            radius_x: 2.0,
            radius_y: 1.0,
            rotation: 0.0,
        })
        .unwrap();
        assert_eq!(e["type"], "ellipse");
        assert_eq!(e["radiusX"], 2.0);
    }
}
