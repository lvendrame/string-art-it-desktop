//! Canvas hit testing (pure). Port of web `ui/canvas/hitTesting.ts`.

use sa_domain::Point;

use crate::document::layers::pin_index;
use crate::document::{Pin, PinLayer, PinPath, ThreadLayer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinHit {
    pub layer_id: String,
    pub path_id: String,
    pub pin_id: String,
}

fn nearest_among<'a>(
    layers: impl IntoIterator<Item = &'a PinLayer>,
    point: Point,
    max: f64,
    pins_of: impl Fn(&PinPath) -> Vec<Pin>,
) -> Option<PinHit> {
    let mut best = None;
    let mut best_d = f64::INFINITY;
    for l in layers {
        for p in &l.pin_paths {
            for pin in pins_of(p) {
                let d = pin.point().distance(point);
                if d <= max && d < best_d {
                    best_d = d;
                    best = Some(PinHit {
                        layer_id: l.id.clone(),
                        path_id: p.id.clone(),
                        pin_id: pin.id,
                    });
                }
            }
        }
    }
    best
}

/// Real (stored) pins only — the Pin Eraser's target set.
pub fn nearest_pin_owner(layers: &[PinLayer], point: Point, max: f64) -> Option<PinHit> {
    nearest_among(layers, point, max, |p| p.pins.clone())
}

pub fn nearest_pin_or_mirror_owner<'a>(
    layers: impl IntoIterator<Item = &'a PinLayer>,
    point: Point,
    max: f64,
) -> Option<PinHit> {
    nearest_among(layers, point, max, PinPath::all_pins_with_mirrors)
}

fn prioritized<'a>(layers: &'a [PinLayer], active: &str) -> (Vec<&'a PinLayer>, Vec<&'a PinLayer>) {
    let visible = layers.iter().filter(|l| l.visible);
    let (a, o): (Vec<_>, Vec<_>) = visible.partition(|l| l.id == active);
    (a, o)
}

/// Thread insertion / Edit-mode path select: visible layers only, active layer
/// wins outright, mirrored pins included.
pub fn nearest_thread_insertion_pin(
    layers: &[PinLayer],
    active: &str,
    point: Point,
    max: f64,
) -> Option<PinHit> {
    let (a, o) = prioritized(layers, active);
    nearest_pin_or_mirror_owner(a, point, max)
        .or_else(|| nearest_pin_or_mirror_owner(o, point, max))
}

pub fn nearest_select_path_hit(
    layers: &[PinLayer],
    active: &str,
    point: Point,
    max: f64,
) -> Option<PinHit> {
    nearest_thread_insertion_pin(layers, active, point, max)
}

pub fn nearest_select_pin_hit(
    layers: &[PinLayer],
    active: &str,
    point: Point,
    max: f64,
) -> Option<PinHit> {
    let (a, o) = prioritized(layers, active);
    nearest_among(a, point, max, |p| p.pins.clone())
        .or_else(|| nearest_among(o, point, max, |p| p.pins.clone()))
}

pub fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len_sq = dx * dx + dy * dy;
    let t = if len_sq == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0)
    };
    p.distance(Point::new(a.x + t * dx, a.y + t * dy))
}

/// Normalized document-space rectangle (rubber band).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DocRect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl DocRect {
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x0.min(self.x1)
            && p.x <= self.x0.max(self.x1)
            && p.y >= self.y0.min(self.y1)
            && p.y <= self.y0.max(self.y1)
    }
}

pub fn selectable_pin_paths_touching_rect(
    layers: &[PinLayer],
    active: &str,
    rect: DocRect,
) -> Vec<(String, String)> {
    let touched = |ls: &[&PinLayer]| -> Vec<(String, String)> {
        ls.iter()
            .flat_map(|l| {
                l.pin_paths
                    .iter()
                    .filter(|p| {
                        p.all_pins_with_mirrors()
                            .iter()
                            .any(|q| rect.contains(q.point()))
                    })
                    .map(|p| (l.id.clone(), p.id.clone()))
            })
            .collect()
    };
    let (a, o) = prioritized(layers, active);
    let at = touched(&a);
    if at.is_empty() { touched(&o) } else { at }
}

pub fn selectable_pins_touching_rect(
    layers: &[PinLayer],
    active: &str,
    rect: DocRect,
) -> Vec<PinHit> {
    let touched = |ls: &[&PinLayer]| -> Vec<PinHit> {
        let mut out = Vec::new();
        for l in ls {
            for p in &l.pin_paths {
                for q in p.pins.iter().filter(|q| rect.contains(q.point())) {
                    out.push(PinHit {
                        layer_id: l.id.clone(),
                        path_id: p.id.clone(),
                        pin_id: q.id.clone(),
                    });
                }
            }
        }
        out
    };
    let (a, o) = prioritized(layers, active);
    let at = touched(&a);
    if at.is_empty() { touched(&o) } else { at }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadHit {
    pub layer_id: String,
    pub path_id: String,
    pub segment_index: usize,
}

/// Nearest thread segment (Path Eraser / Segment Eraser / Thread Select).
pub fn nearest_thread_path(
    threads: &[ThreadLayer],
    pins: &[PinLayer],
    point: Point,
    max: f64,
) -> Option<ThreadHit> {
    let index = pin_index(pins);
    let mut best = None;
    let mut best_d = f64::INFINITY;
    for l in threads {
        for t in &l.thread_paths {
            for (i, w) in t.pin_ids.windows(2).enumerate() {
                let (Some(a), Some(b)) = (index.get(&w[0]), index.get(&w[1])) else {
                    continue;
                };
                let d = distance_to_segment(point, a.point(), b.point());
                if d <= max && d < best_d {
                    best_d = d;
                    best = Some(ThreadHit {
                        layer_id: l.id.clone(),
                        path_id: t.id.clone(),
                        segment_index: i,
                    });
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{PinPathGeometry, PinStyle, SymmetryConfig};

    fn layer(name: &str, x: f64, visible: bool) -> PinLayer {
        let mut l = PinLayer::new(name);
        l.visible = visible;
        l.pin_paths.push(PinPath::new(
            PinPathGeometry::Line {
                start: Point::new(x, 0.0),
                end: Point::new(x + 1.0, 0.0),
            },
            1.0,
            &PinStyle {
                colour: "#fff".into(),
                diameter: 1.0,
                guide_visible: true,
            },
            SymmetryConfig::Vertical {
                axis: Point::ORIGIN,
            },
        ));
        l
    }

    #[test]
    fn active_layer_wins_outright() {
        let ls = vec![layer("a", 0.0, true), layer("b", 0.2, true)];
        let hit = nearest_thread_insertion_pin(&ls, &ls[0].id, Point::new(0.25, 0.0), 1.0).unwrap();
        assert_eq!(hit.layer_id, ls[0].id);
        let hit = nearest_pin_or_mirror_owner(&ls, Point::new(-1.0, 0.0), 0.1).unwrap();
        assert!(hit.pin_id.contains("~mirror-"));
        assert!(nearest_pin_owner(&ls, Point::new(-1.0, 0.0), 0.1).is_none());
    }

    #[test]
    fn hidden_layers_ignored() {
        let ls = vec![layer("a", 0.0, false)];
        assert!(nearest_thread_insertion_pin(&ls, &ls[0].id, Point::ORIGIN, 1.0).is_none());
    }

    #[test]
    fn segment_distance() {
        assert_eq!(
            distance_to_segment(Point::new(5.0, 2.0), Point::ORIGIN, Point::new(10.0, 0.0)),
            2.0
        );
        assert_eq!(
            distance_to_segment(Point::new(-3.0, 4.0), Point::ORIGIN, Point::new(10.0, 0.0)),
            5.0
        );
    }
}
