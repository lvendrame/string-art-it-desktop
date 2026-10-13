//! Per-Pin-Path symmetry (spec 06). Port of web `symmetryConfig.ts`.
//! Mirrored/radial copies are DERIVED on every read, never stored.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use sa_domain::Point;
use sa_domain::symmetry::{MirrorMode, generate_radial_copies, mirror_copies};

use super::pin_path::{Pin, PinPath};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SymmetryConfig {
    #[default]
    None,
    Vertical {
        axis: Point,
    },
    Horizontal {
        axis: Point,
    },
    Both {
        axis: Point,
    },
    #[serde(rename_all = "camelCase")]
    Radial {
        centre: Point,
        interval_degrees: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymmetryType {
    None,
    Horizontal,
    Vertical,
    Both,
    Radial,
}

/// Spec 34 Shift+S cycle order.
pub const SYMMETRY_TYPE_CYCLE: [SymmetryType; 5] = [
    SymmetryType::None,
    SymmetryType::Horizontal,
    SymmetryType::Vertical,
    SymmetryType::Both,
    SymmetryType::Radial,
];

impl SymmetryConfig {
    pub fn kind(&self) -> SymmetryType {
        match self {
            SymmetryConfig::None => SymmetryType::None,
            SymmetryConfig::Vertical { .. } => SymmetryType::Vertical,
            SymmetryConfig::Horizontal { .. } => SymmetryType::Horizontal,
            SymmetryConfig::Both { .. } => SymmetryType::Both,
            SymmetryConfig::Radial { .. } => SymmetryType::Radial,
        }
    }

    /// Out-of-the-box config for a type (axis/centre at origin, radial 45°).
    pub fn for_type(kind: SymmetryType) -> Self {
        let axis = Point::ORIGIN;
        match kind {
            SymmetryType::None => SymmetryConfig::None,
            SymmetryType::Vertical => SymmetryConfig::Vertical { axis },
            SymmetryType::Horizontal => SymmetryConfig::Horizontal { axis },
            SymmetryType::Both => SymmetryConfig::Both { axis },
            SymmetryType::Radial => SymmetryConfig::Radial {
                centre: axis,
                interval_degrees: 45.0,
            },
        }
    }

    pub fn translated(&self, delta: Point) -> Self {
        let t = |p: Point| Point::new(p.x + delta.x, p.y + delta.y);
        match self {
            SymmetryConfig::None => SymmetryConfig::None,
            SymmetryConfig::Vertical { axis } => SymmetryConfig::Vertical { axis: t(*axis) },
            SymmetryConfig::Horizontal { axis } => SymmetryConfig::Horizontal { axis: t(*axis) },
            SymmetryConfig::Both { axis } => SymmetryConfig::Both { axis: t(*axis) },
            SymmetryConfig::Radial {
                centre,
                interval_degrees,
            } => SymmetryConfig::Radial {
                centre: t(*centre),
                interval_degrees: *interval_degrees,
            },
        }
    }

    /// Copies of `points`, one list per derived instance (source excluded).
    pub fn copies(&self, points: &[Point]) -> Vec<Vec<Point>> {
        match self {
            SymmetryConfig::None => Vec::new(),
            SymmetryConfig::Vertical { axis } => mirror_copies(points, MirrorMode::Vertical, *axis),
            SymmetryConfig::Horizontal { axis } => {
                mirror_copies(points, MirrorMode::Horizontal, *axis)
            }
            SymmetryConfig::Both { axis } => mirror_copies(points, MirrorMode::Both, *axis),
            SymmetryConfig::Radial {
                centre,
                interval_degrees,
            } => generate_radial_copies(points, *centre, *interval_degrees).unwrap_or_default(),
        }
    }
}

pub fn mirrored_pin_id(source_pin_id: &str, group_index: usize) -> String {
    format!("{source_pin_id}~mirror-{group_index}")
}

impl PinPath {
    /// Derived mirrored/radial pin groups with stable ids usable as thread endpoints.
    pub fn mirrored_pin_groups(&self) -> Vec<Vec<Pin>> {
        let points: Vec<Point> = self.pins.iter().map(Pin::point).collect();
        self.symmetry
            .copies(&points)
            .into_iter()
            .enumerate()
            .map(|(g, group)| {
                group
                    .into_iter()
                    .zip(&self.pins)
                    .map(|(pt, src)| Pin {
                        id: mirrored_pin_id(&src.id, g),
                        x: pt.x,
                        y: pt.y,
                    })
                    .collect()
            })
            .collect()
    }

    /// Every real + mirrored pin on this path.
    pub fn all_pins_with_mirrors(&self) -> Vec<Pin> {
        let mut all = self.pins.clone();
        all.extend(self.mirrored_pin_groups().into_iter().flatten());
        all
    }
}

/// Spec 21 Nearest-Pin Reattachment: maps every old pin (real or mirrored) to the
/// nearest new pin (real or mirrored).
pub fn build_nearest_pin_remap(old_path: &PinPath, new_path: &PinPath) -> HashMap<String, String> {
    let new_pins = new_path.all_pins_with_mirrors();
    let mut map = HashMap::new();
    for old in old_path.all_pins_with_mirrors() {
        let nearest = new_pins.iter().min_by(|a, b| {
            let da = (a.x - old.x).powi(2) + (a.y - old.y).powi(2);
            let db = (b.x - old.x).powi(2) + (b.y - old.y).powi(2);
            da.total_cmp(&db)
        });
        if let Some(n) = nearest {
            map.insert(old.id, n.id.clone());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::pin_path::{PinPathGeometry, PinStyle};

    fn style() -> PinStyle {
        PinStyle {
            colour: "#fff".into(),
            diameter: 2.0,
            guide_visible: true,
        }
    }

    fn point_path(x: f64, symmetry: SymmetryConfig) -> PinPath {
        PinPath::new(
            PinPathGeometry::Line {
                start: Point::new(x, 0.0),
                end: Point::new(x, 0.0),
            },
            4.0,
            &style(),
            symmetry,
        )
    }

    #[test]
    fn groups() {
        assert!(
            point_path(1.0, SymmetryConfig::None)
                .mirrored_pin_groups()
                .is_empty()
        );
        let path = point_path(
            1.0,
            SymmetryConfig::Vertical {
                axis: Point::ORIGIN,
            },
        );
        let groups = path.mirrored_pin_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0][0].point(), Point::new(-1.0, 0.0));
        assert_ne!(groups[0][0].id, path.pins[0].id);
        assert!(groups[0][0].id.contains(&path.pins[0].id));
        let radial = point_path(
            10.0,
            SymmetryConfig::Radial {
                centre: Point::ORIGIN,
                interval_degrees: 45.0,
            },
        );
        assert_eq!(radial.mirrored_pin_groups().len(), 7);
    }

    #[test]
    fn json_tags() {
        let v = serde_json::to_value(SymmetryConfig::Radial {
            centre: Point::ORIGIN,
            interval_degrees: 45.0,
        })
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({"type":"radial","centre":{"x":0.0,"y":0.0},"intervalDegrees":45.0})
        );
        let n: SymmetryConfig = serde_json::from_str(r#"{"type":"none"}"#).unwrap();
        assert_eq!(n, SymmetryConfig::None);
    }

    fn manual(id: &str, pins: &[(&str, f64)], symmetry: SymmetryConfig) -> PinPath {
        PinPath {
            id: id.into(),
            geometry: PinPathGeometry::Circle {
                center: Point::ORIGIN,
                radius: 5.0,
            },
            requested_spacing: 1.0,
            actual_spacing: 1.0,
            pins: pins
                .iter()
                .map(|(i, x)| Pin {
                    id: (*i).into(),
                    x: *x,
                    y: 0.0,
                })
                .collect(),
            guide_visible: true,
            colour: "#fff".into(),
            diameter: 2.0,
            symmetry,
        }
    }

    #[test]
    fn nearest_remap() {
        let m = build_nearest_pin_remap(
            &manual("p", &[("p1", 0.0), ("p2", 10.0)], SymmetryConfig::None),
            &manual("p", &[("n1", 1.0), ("n2", 9.0)], SymmetryConfig::None),
        );
        assert_eq!(m["p1"], "n1");
        assert_eq!(m["p2"], "n2");
        let m = build_nearest_pin_remap(
            &manual("p", &[("p1", 0.0), ("p2", 1.0)], SymmetryConfig::None),
            &manual("p", &[("n1", 0.5)], SymmetryConfig::None),
        );
        assert_eq!((m["p1"].as_str(), m["p2"].as_str()), ("n1", "n1"));
        let sym = SymmetryConfig::Vertical {
            axis: Point::ORIGIN,
        };
        let old = manual("p", &[("p1", 3.0)], sym.clone());
        let new = manual("p", &[("n1", 6.0)], sym);
        let m = build_nearest_pin_remap(&old, &new);
        assert_eq!(m["p1"], "n1");
        assert_eq!(
            m[&old.mirrored_pin_groups()[0][0].id],
            new.mirrored_pin_groups()[0][0].id
        );
    }
}
