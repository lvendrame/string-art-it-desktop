//! Point transforms and the document↔screen viewport (spec 05).
//! Port of web `src/domain/transforms/*`.

use crate::paths::Point;

pub fn rotate_point(point: Point, pivot: Point, radians: f64) -> Point {
    let (sin, cos) = radians.sin_cos();
    let (dx, dy) = (point.x - pivot.x, point.y - pivot.y);
    Point::new(pivot.x + dx * cos - dy * sin, pivot.y + dx * sin + dy * cos)
}

pub fn translate_point(point: Point, delta: Point) -> Point {
    Point::new(point.x + delta.x, point.y + delta.y)
}

pub fn scale_point(point: Point, pivot: Point, factor: f64) -> Point {
    Point::new(
        pivot.x + (point.x - pivot.x) * factor,
        pivot.y + (point.y - pivot.y) * factor,
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BoundingBox {
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }
    pub fn center(&self) -> Point {
        Point::new(
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }
    pub fn expand(&self, margin: f64) -> BoundingBox {
        BoundingBox {
            min_x: self.min_x - margin,
            min_y: self.min_y - margin,
            max_x: self.max_x + margin,
            max_y: self.max_y + margin,
        }
    }
}

pub fn bounding_box_of(points: &[Point]) -> BoundingBox {
    let Some(first) = points.first() else {
        return BoundingBox::default();
    };
    points.iter().fold(
        BoundingBox {
            min_x: first.x,
            min_y: first.y,
            max_x: first.x,
            max_y: first.y,
        },
        |b, p| BoundingBox {
            min_x: b.min_x.min(p.x),
            min_y: b.min_y.min(p.y),
            max_x: b.max_x.max(p.x),
            max_y: b.max_y.max(p.y),
        },
    )
}

/// `zoom` is screen pixels per document unit (cm); `pan_origin` is the document
/// point sitting at screen (0,0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub zoom: f64,
    pub pan_origin: Point,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            zoom: CSS_PIXELS_PER_CM,
            pan_origin: Point::ORIGIN,
        }
    }
}

pub fn to_screen(point: Point, v: &Viewport) -> Point {
    Point::new(
        (point.x - v.pan_origin.x) * v.zoom,
        (point.y - v.pan_origin.y) * v.zoom,
    )
}

pub fn to_document(point: Point, v: &Viewport) -> Point {
    Point::new(
        point.x / v.zoom + v.pan_origin.x,
        point.y / v.zoom + v.pan_origin.y,
    )
}

/// Screen-space distances (snap radius) convert by dividing by zoom so
/// interaction stays visually constant at any zoom.
pub fn screen_distance_to_document(distance: f64, v: &Viewport) -> f64 {
    distance / v.zoom
}

/// CSS reference pixel = 1/96 inch: "100%" means one physical centimetre.
pub const CSS_PIXELS_PER_CM: f64 = 96.0 / 2.54;

pub fn zoom_to_percent(zoom: f64) -> f64 {
    zoom / CSS_PIXELS_PER_CM * 100.0
}

pub fn percent_to_zoom(percent: f64) -> f64 {
    percent / 100.0 * CSS_PIXELS_PER_CM
}

/// Changes zoom keeping the document point under `screen_anchor` fixed.
pub fn zoom_at_point(v: &Viewport, new_zoom: f64, screen_anchor: Point) -> Viewport {
    let anchor_doc = to_document(screen_anchor, v);
    Viewport {
        zoom: new_zoom,
        pan_origin: Point::new(
            anchor_doc.x - screen_anchor.x / new_zoom,
            anchor_doc.y - screen_anchor.y / new_zoom,
        ),
    }
}

pub fn fit_to_viewport(box_: &BoundingBox, width: f64, height: f64, padding_px: f64) -> Viewport {
    let content_w = box_.width().max(1e-6);
    let content_h = box_.height().max(1e-6);
    let avail_w = (width - padding_px * 2.0).max(1.0);
    let avail_h = (height - padding_px * 2.0).max(1.0);
    let zoom = (avail_w / content_w).min(avail_h / content_h);
    let c = box_.center();
    Viewport {
        zoom,
        pan_origin: Point::new(c.x - width / 2.0 / zoom, c.y - height / 2.0 / zoom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_close;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn rotate_translate_scale() {
        let r = rotate_point(p(1.0, 0.0), Point::ORIGIN, std::f64::consts::FRAC_PI_2);
        assert_close(r.x, 0.0, 1e-6);
        assert_close(r.y, 1.0, 1e-6);
        assert_eq!(translate_point(p(1.0, 2.0), p(3.0, -4.0)), p(4.0, -2.0));
        assert_eq!(scale_point(p(10.0, 0.0), Point::ORIGIN, 2.0), p(20.0, 0.0));
        assert_eq!(scale_point(p(5.0, 5.0), p(5.0, 5.0), 3.0), p(5.0, 5.0));
        assert_eq!(scale_point(p(8.0, 8.0), p(4.0, 4.0), 0.5), p(6.0, 6.0));
    }

    #[test]
    fn bounding_box() {
        assert_eq!(
            bounding_box_of(&[p(-2.0, 5.0), p(4.0, -1.0)]),
            BoundingBox {
                min_x: -2.0,
                min_y: -1.0,
                max_x: 4.0,
                max_y: 5.0
            }
        );
        assert_eq!(bounding_box_of(&[]), BoundingBox::default());
    }

    #[test]
    fn viewport_round_trip_and_zoom() {
        let v = Viewport {
            zoom: 2.0,
            pan_origin: p(10.0, 10.0),
        };
        let doc = p(15.0, 20.0);
        assert_eq!(to_document(to_screen(doc, &v), &v), doc);
        let v1 = Viewport {
            zoom: 1.0,
            pan_origin: Point::ORIGIN,
        };
        let v4 = Viewport {
            zoom: 4.0,
            pan_origin: Point::ORIGIN,
        };
        let (a, b) = (to_screen(doc, &v1), to_screen(doc, &v4));
        assert_eq!(b, p(a.x * 4.0, a.y * 4.0));
        assert_eq!(
            screen_distance_to_document(20.0, &Viewport { zoom: 2.0, ..v1 }),
            10.0
        );
        assert_eq!(
            screen_distance_to_document(20.0, &Viewport { zoom: 200.0, ..v1 }),
            0.1
        );
    }

    #[test]
    fn zoom_at_point_keeps_anchor() {
        let v = Viewport {
            zoom: 2.0,
            pan_origin: p(5.0, 5.0),
        };
        let anchor = p(360.0, 320.0);
        let before = to_document(anchor, &v);
        let z = zoom_at_point(&v, 4.0, anchor);
        let after = to_document(anchor, &z);
        assert_close(after.x, before.x, 1e-6);
        assert_close(after.y, before.y, 1e-6);
        assert_eq!(z.zoom, 4.0);

        let z = zoom_at_point(
            &Viewport {
                zoom: 1.0,
                pan_origin: Point::ORIGIN,
            },
            2.0,
            p(100.0, 100.0),
        );
        assert_eq!(to_screen(p(100.0, 100.0), &z), p(100.0, 100.0));
    }

    #[test]
    fn percent_conversions() {
        assert_close(zoom_to_percent(CSS_PIXELS_PER_CM), 100.0, 1e-6);
        assert_close(percent_to_zoom(100.0), CSS_PIXELS_PER_CM, 1e-6);
        assert!(zoom_to_percent(1.0) < 3.0);
        assert_close(zoom_to_percent(percent_to_zoom(250.0)), 250.0, 1e-6);
    }

    #[test]
    fn fit() {
        let b = BoundingBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 100.0,
            max_y: 50.0,
        };
        assert_close(fit_to_viewport(&b, 200.0, 200.0, 0.0).zoom, 2.0, 1e-6);
    }
}
