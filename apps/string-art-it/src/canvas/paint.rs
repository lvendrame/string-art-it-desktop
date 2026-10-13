//! Paints a [`Scene`] with egui's painter (vector), sharing the dash splitter
//! with every other backend so strands/guides look identical to exports.

use egui::{Color32, Painter, Pos2, Shape, Stroke};
use sa_domain::Point;
use sa_domain::transforms::{Viewport, to_screen};
use sa_render::dash::dash_polyline;
use sa_render::scene::{Item, Scene, Stroke as SceneStroke};
use sa_render::{Rgba, path::PathData};

pub fn c32(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

/// Document → screen mapping for one canvas rect.
#[derive(Clone, Copy)]
pub struct Xf {
    pub vp: Viewport,
    pub origin: Pos2,
}

impl Xf {
    pub fn pos(&self, p: Point) -> Pos2 {
        let s = to_screen(p, &self.vp);
        Pos2::new(self.origin.x + s.x as f32, self.origin.y + s.y as f32)
    }

    pub fn len(&self, cm: f64) -> f32 {
        (cm * self.vp.zoom) as f32
    }

    pub fn doc(&self, p: Pos2) -> Point {
        sa_domain::transforms::to_document(
            Point::new((p.x - self.origin.x) as f64, (p.y - self.origin.y) as f64),
            &self.vp,
        )
    }
}

/// Strokes a polyline in document units, honouring dashes.
pub fn stroke_polyline(painter: &Painter, xf: &Xf, points: &[Point], stroke: &SceneStroke) {
    let s = Stroke::new(xf.len(stroke.width).max(0.5), c32(stroke.color));
    let runs = match &stroke.dash {
        Some(d) => dash_polyline(points, &d.array, d.offset),
        None => vec![points.to_vec()],
    };
    for run in runs {
        if run.len() < 2 {
            continue;
        }
        let pts: Vec<Pos2> = run.iter().map(|p| xf.pos(*p)).collect();
        painter.add(Shape::line(pts, s));
    }
}

pub fn stroke_path(painter: &Painter, xf: &Xf, data: &PathData, stroke: &SceneStroke) {
    let tolerance = 0.3 / xf.vp.zoom;
    for (mut pts, closed) in data.flatten(tolerance) {
        if closed && let Some(first) = pts.first().copied() {
            pts.push(first);
        }
        stroke_polyline(painter, xf, &pts, stroke);
    }
}

/// Draws every non-board item (the board fill is a cached texture).
pub fn paint_items(painter: &Painter, xf: &Xf, scene: &Scene) {
    for item in &scene.items {
        match item {
            Item::Board {
                outline, stroke, ..
            } => {
                if let Some(s) = stroke {
                    stroke_path(painter, xf, outline, s);
                }
            }
            Item::Path {
                data,
                fill: _,
                stroke,
            } => {
                if let Some(s) = stroke {
                    stroke_path(painter, xf, data, s);
                }
            }
            Item::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                let r = xf.len(*radius).max(1.0);
                let st = stroke
                    .as_ref()
                    .map(|s| Stroke::new(xf.len(s.width).max(0.5), c32(s.color)))
                    .unwrap_or(Stroke::NONE);
                painter.circle(xf.pos(*center), r, c32(*fill), st);
            }
            Item::Polyline { points, stroke } => stroke_polyline(painter, xf, points, stroke),
            Item::Text {
                pos,
                text,
                size,
                color,
                ..
            } => {
                painter.text(
                    xf.pos(*pos),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(xf.len(*size).max(6.0)),
                    c32(*color),
                );
            }
        }
    }
}
