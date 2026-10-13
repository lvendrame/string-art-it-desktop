//! Backend-neutral scene: the one place document state becomes drawable
//! primitives (spec 01 — editor, print and export consume the same geometry).
//! Units are centimetres, y down.

use std::sync::Arc;

use sa_app::document::{
    BoardAppearance, Document, GeometryKind, PinPath, PinPathGeometry, layers::pin_index,
};
use sa_app::print::PrintElements;
use sa_domain::transforms::{BoundingBox, bounding_box_of};
use sa_domain::{Path, Point};

use crate::color::Rgba;
use crate::image::{DecodedImage, decode_data_url};
use crate::path::PathData;
use crate::wood::{WoodBand, paint_colours, wood_colours, wood_grain};

#[derive(Clone, Debug, PartialEq)]
pub struct Dash {
    pub array: Vec<f64>,
    pub offset: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub color: Rgba,
    pub width: f64,
    pub dash: Option<Dash>,
}

impl Stroke {
    pub fn solid(color: Rgba, width: f64) -> Self {
        Self {
            color,
            width,
            dash: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stop {
    /// 0..=1
    pub offset: f64,
    pub color: Rgba,
}

/// Board background paints, in the board's bounding-box space (SVG
/// `objectBoundingBox` semantics, as the web app uses).
#[derive(Clone, Debug)]
pub enum BoardFill {
    Solid(Rgba),
    /// Default SVG linear gradient (left→right) rotated by `direction_deg` about
    /// the bbox origin.
    Linear {
        stops: Vec<Stop>,
        direction_deg: f64,
    },
    /// `centre` in bbox fractions, radius 0.5.
    Radial {
        stops: Vec<Stop>,
        centre: Point,
    },
    Wood {
        bands: Arc<Vec<WoodBand>>,
        tint: Option<Rgba>,
    },
    /// Image stretched to cover the bbox (`xMidYMid slice`).
    Image(Arc<DecodedImage>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAnchor {
    Middle,
}

#[derive(Clone, Debug)]
pub enum Item {
    Board {
        outline: PathData,
        bbox: BoundingBox,
        fill: Option<BoardFill>,
        stroke: Option<Stroke>,
    },
    Path {
        data: PathData,
        fill: Option<Rgba>,
        stroke: Option<Stroke>,
    },
    Circle {
        center: Point,
        radius: f64,
        fill: Rgba,
        stroke: Option<Stroke>,
    },
    Polyline {
        points: Vec<Point>,
        stroke: Stroke,
    },
    Text {
        pos: Point,
        text: String,
        size: f64,
        color: Rgba,
        anchor: TextAnchor,
    },
}

#[derive(Clone, Debug)]
pub struct Scene {
    /// Visible extent in cm (board bbox + margin for exports).
    pub bounds: BoundingBox,
    pub items: Vec<Item>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    /// Live editor look (spec 18 canvas): sized pins, threads under pins.
    Editor,
    /// Export look (spec 15): fixed 0.06 cm pin dots, threads on top.
    Export,
    /// Print template (spec 14): black pin dots, labels and lines sized for
    /// the paper. `scale` is the board-cm → paper-cm multiplier.
    Print { scale: f64 },
}

#[derive(Clone, Copy, Debug)]
pub struct SceneOptions {
    pub elements: PrintElements,
    pub style: Style,
    /// Margin around the board bbox (export: 2 cm).
    pub margin: f64,
    /// Grid line spacing (cm) when `elements.grid` is on.
    pub grid_gap: (f64, f64),
}

impl SceneOptions {
    pub fn export(elements: PrintElements) -> Self {
        Self {
            elements,
            style: Style::Export,
            margin: EXPORT_MARGIN_CM,
            grid_gap: (1.0, 1.0),
        }
    }

    pub fn print(elements: PrintElements, scale: f64, grid_gap: (f64, f64)) -> Self {
        Self {
            elements,
            style: Style::Print { scale },
            margin: 0.0,
            grid_gap,
        }
    }

    pub fn editor() -> Self {
        Self {
            elements: PrintElements {
                pin_numbers: false,
                grid: false,
                ..PrintElements::default()
            },
            style: Style::Editor,
            margin: 0.0,
            grid_gap: (1.0, 1.0),
        }
    }
}

pub const EXPORT_MARGIN_CM: f64 = 2.0;
pub const PIN_DOT_RADIUS_CM: f64 = 0.06;
const PIN_NUMBER_FONT_CM: f64 = 0.2;
const PIN_NUMBER_GAP_CM: f64 = 0.08;
const PIN_NUMBER_OFFSET_CM: f64 = PIN_DOT_RADIUS_CM + PIN_NUMBER_FONT_CM / 2.0 + PIN_NUMBER_GAP_CM;
pub const PIN_STROKE: Rgba = Rgba::rgb(0x1b, 0x1b, 0x1b);

pub fn board_fill(appearance: &BoardAppearance) -> Option<BoardFill> {
    let stops = |s: &[sa_app::document::GradientStop]| -> Vec<Stop> {
        s.iter()
            .map(|s| Stop {
                offset: (s.offset / 100.0).clamp(0.0, 1.0),
                color: Rgba::parse(&s.colour),
            })
            .collect()
    };
    Some(match appearance {
        BoardAppearance::Solid { colour } => BoardFill::Solid(Rgba::parse(colour)),
        BoardAppearance::LinearGradient {
            stops: s,
            direction,
        } => BoardFill::Linear {
            stops: stops(s),
            direction_deg: *direction,
        },
        BoardAppearance::RadialGradient { stops: s, centre } => BoardFill::Radial {
            stops: stops(s),
            centre: Point::new(centre.x / 100.0, centre.y / 100.0),
        },
        BoardAppearance::WoodTexture { preset_id } => BoardFill::Wood {
            bands: wood_grain(preset_id, wood_colours(preset_id)),
            tint: None,
        },
        BoardAppearance::PaintedWood { preset_id } => {
            let c = paint_colours(preset_id);
            BoardFill::Wood {
                bands: wood_grain(preset_id, c),
                tint: Some(c[1].with_opacity(0.45)),
            }
        }
        BoardAppearance::CustomTexture { image_data_url } => {
            BoardFill::Image(decode_data_url(image_data_url)?)
        }
    })
}

/// Board bbox (cm), without margin.
pub fn board_bbox(doc: &Document) -> BoundingBox {
    bounding_box_of(&doc.board.path().bounding_box_points())
}

/// Spec 15 local-normal label placement: perpendicular to the path's tangent at
/// each pin, oriented away from the shape's centre.
pub fn pin_label_positions(pins: &[Point], closed: bool, center: Point, offset: f64) -> Vec<Point> {
    let n = pins.len();
    (0..n)
        .map(|i| {
            let pin = pins[i];
            let prev = if closed {
                Some(pins[(i + n - 1) % n])
            } else {
                i.checked_sub(1).map(|j| pins[j])
            };
            let next = if closed {
                Some(pins[(i + 1) % n])
            } else {
                pins.get(i + 1).copied()
            };
            let tx = next.unwrap_or(pin).x - prev.unwrap_or(pin).x;
            let ty = next.unwrap_or(pin).y - prev.unwrap_or(pin).y;
            let tlen = tx.hypot(ty);
            let (mut nx, mut ny);
            if tlen > 1e-6 {
                nx = -ty / tlen;
                ny = tx / tlen;
                if nx * (pin.x - center.x) + ny * (pin.y - center.y) < 0.0 {
                    nx = -nx;
                    ny = -ny;
                }
            } else {
                let (dx, dy) = (pin.x - center.x, pin.y - center.y);
                let d = dx.hypot(dy);
                (nx, ny) = if d > 1e-6 {
                    (dx / d, dy / d)
                } else {
                    (0.0, -1.0)
                };
            }
            Point::new(pin.x + nx * offset, pin.y + ny * offset)
        })
        .collect()
}

pub fn guide_path(geometry: &PinPathGeometry) -> PathData {
    let paths: Vec<Path> = geometry.contour_paths();
    PathData::from_paths(&paths)
}

/// CSS reference pixels per paper cm (web print constants).
const PRINT_PX_PER_CM: f64 = 96.0 / 2.54;

fn pin_radius(path: &PinPath, style: Style) -> f64 {
    match style {
        Style::Editor => (path.diameter / 20.0).max(PIN_DOT_RADIUS_CM),
        Style::Export => PIN_DOT_RADIUS_CM,
        Style::Print { scale } => (path.diameter / 20.0).max(1.5 / (scale * PRINT_PX_PER_CM)),
    }
}

/// Twisted multi-colour thread strands (spec 12): the same polyline once per
/// colour, strands ≥ 1 dashed with offset so they spiral around strand 0.
pub fn thread_strands(
    points: &[Point],
    colours: &[String],
    width: f64,
    twist_pitch: f64,
) -> Vec<(Vec<Point>, Stroke)> {
    let strand_width = width * 0.1;
    let pitch = strand_width * twist_pitch;
    let n = colours.len().max(1) as f64;
    colours
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let color = Rgba::parse(c).with_opacity(if i == 0 { 0.95 } else { 0.9 });
            let dash = (i > 0).then(|| Dash {
                array: vec![pitch, pitch * n],
                offset: i as f64 * pitch,
            });
            (
                points.to_vec(),
                Stroke {
                    color,
                    width: strand_width,
                    dash,
                },
            )
        })
        .collect()
}

pub fn build_scene(doc: &Document, opts: &SceneOptions) -> Scene {
    let el = &opts.elements;
    let bbox = board_bbox(doc);
    let mut items = Vec::new();

    if el.board_outline || el.background {
        let fill = if el.background {
            board_fill(&doc.board.appearance)
        } else {
            None
        };
        let stroke = match opts.style {
            _ if !el.board_outline => None,
            Style::Export | Style::Print { .. } => Some(Stroke::solid(Rgba::BLACK, 0.05)),
            // web BoardLayer: stroke="#00000055" strokeWidth=0.1
            Style::Editor => Some(Stroke::solid(
                Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0x55,
                },
                0.1,
            )),
        };
        items.push(Item::Board {
            outline: PathData::from_paths([&doc.board.path()]),
            bbox,
            fill,
            stroke,
        });
    }

    let threads = |items: &mut Vec<Item>| {
        if !el.threads {
            return;
        }
        let index = pin_index(&doc.pin_layers);
        for layer in doc.thread_layers.iter().filter(|l| l.visible) {
            for t in &layer.thread_paths {
                let pts: Vec<Point> = t
                    .pin_ids
                    .iter()
                    .filter_map(|id| index.get(id))
                    .map(|p| p.point())
                    .collect();
                if pts.len() < 2 {
                    continue;
                }
                if let Style::Print { .. } = opts.style {
                    let c = Rgba::parse(t.colours.first().map(String::as_str).unwrap_or("#000"));
                    items.push(Item::Polyline {
                        points: pts,
                        stroke: Stroke::solid(c, 0.03),
                    });
                    continue;
                }
                for (points, stroke) in thread_strands(&pts, &t.colours, t.width, t.twist_pitch) {
                    items.push(Item::Polyline { points, stroke });
                }
            }
        }
    };

    if el.grid && opts.style != Style::Editor {
        let (gx, gy) = (opts.grid_gap.0.max(0.1), opts.grid_gap.1.max(0.1));
        let w = match opts.style {
            Style::Print { scale } => (1.0 / (scale * PRINT_PX_PER_CM)).max(0.02),
            _ => 0.02,
        };
        let stroke = Stroke::solid(Rgba::BLACK.with_opacity(0.4), w);
        let b = bbox.expand(opts.margin.max(1.0));
        let mut x = (b.min_x / gx).floor() * gx;
        while x <= b.max_x {
            items.push(Item::Polyline {
                points: vec![Point::new(x, b.min_y), Point::new(x, b.max_y)],
                stroke: stroke.clone(),
            });
            x += gx;
        }
        let mut y = (b.min_y / gy).floor() * gy;
        while y <= b.max_y {
            items.push(Item::Polyline {
                points: vec![Point::new(b.min_x, y), Point::new(b.max_x, y)],
                stroke: stroke.clone(),
            });
            y += gy;
        }
    }

    if opts.style == Style::Editor {
        threads(&mut items);
    }

    for layer in doc.pin_layers.iter().filter(|l| l.visible) {
        for p in &layer.pin_paths {
            if el.pin_guides && (opts.style != Style::Editor || p.guide_visible) {
                let (width, dash) = match opts.style {
                    Style::Editor => (0.05, vec![0.2, 0.15]),
                    Style::Export | Style::Print { .. } => (0.03, vec![0.15, 0.1]),
                };
                items.push(Item::Path {
                    data: guide_path(&p.geometry),
                    fill: None,
                    stroke: Some(Stroke {
                        color: Rgba::BLACK.with_opacity(0.8),
                        width,
                        dash: Some(Dash {
                            array: dash,
                            offset: 0.0,
                        }),
                    }),
                });
            }
            if !el.pins {
                continue;
            }
            let radius = pin_radius(p, opts.style);
            let fill = Rgba::parse(&p.colour);
            let stroke_w = if opts.style == Style::Export {
                0.015
            } else {
                0.02
            };
            // Mirrored copies are real physical pins: drawn in editor and export.
            for pin in p.mirrored_pin_groups().iter().flatten().chain(&p.pins) {
                items.push(Item::Circle {
                    center: pin.point(),
                    radius,
                    fill,
                    stroke: Some(Stroke::solid(PIN_STROKE, stroke_w)),
                });
            }
            if el.pin_numbers {
                let pts: Vec<Point> = p.pins.iter().map(|q| q.point()).collect();
                let closed = p.geometry.kind() == GeometryKind::Text
                    || p.geometry.contour_paths().first().is_some_and(|c| c.closed);
                let labels =
                    pin_label_positions(&pts, closed, p.geometry.center(), PIN_NUMBER_OFFSET_CM);
                for (i, pos) in labels.into_iter().enumerate() {
                    items.push(Item::Text {
                        pos,
                        text: (i + 1).to_string(),
                        size: PIN_NUMBER_FONT_CM,
                        color: Rgba::BLACK,
                        anchor: TextAnchor::Middle,
                    });
                }
            }
        }
    }

    if opts.style != Style::Editor {
        threads(&mut items);
    }

    Scene {
        bounds: bbox.expand(opts.margin),
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_app::document::{PinStyle, SymmetryConfig, ThreadPath};

    fn doc_with_mirrored_thread() -> Document {
        let mut doc = Document::empty();
        let path = PinPath::new(
            PinPathGeometry::Line {
                start: Point::new(2.0, 0.0),
                end: Point::new(2.0, 0.0),
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
        );
        let id = path.pins[0].id.clone();
        doc.pin_layers[0].pin_paths.push(path);
        doc.thread_layers[0].thread_paths.push(ThreadPath::new(
            vec![id.clone(), format!("{id}~mirror-0")],
            vec!["#ff0000".into(), "#ffffff".into(), "#0000ff".into()],
            1.0,
            6.0,
        ));
        doc
    }

    #[test]
    fn export_includes_threads_on_mirrored_pins_with_every_colour() {
        let scene = build_scene(
            &doc_with_mirrored_thread(),
            &SceneOptions::export(PrintElements::default()),
        );
        let strands: Vec<&Stroke> = scene
            .items
            .iter()
            .filter_map(|i| {
                if let Item::Polyline { stroke, .. } = i {
                    Some(stroke)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(strands.len(), 3);
        assert!(strands[0].dash.is_none());
        let d = strands[2].dash.as_ref().unwrap();
        assert!(
            (d.array[0] - 0.6).abs() < 1e-12
                && (d.array[1] - 1.8).abs() < 1e-12
                && (d.offset - 1.2).abs() < 1e-12
        );
        // threads drawn last in export
        assert!(matches!(scene.items.last(), Some(Item::Polyline { .. })));
        // margin applied
        assert!((scene.bounds.min_x + 32.0).abs() < 1e-6);
    }

    #[test]
    fn editor_draws_threads_under_pins_and_skips_numbers() {
        let scene = build_scene(&doc_with_mirrored_thread(), &SceneOptions::editor());
        let first_thread = scene
            .items
            .iter()
            .position(|i| matches!(i, Item::Polyline { .. }))
            .unwrap();
        let first_pin = scene
            .items
            .iter()
            .position(|i| matches!(i, Item::Circle { .. }))
            .unwrap();
        assert!(first_thread < first_pin);
        assert!(!scene.items.iter().any(|i| matches!(i, Item::Text { .. })));
        assert_eq!(
            scene
                .items
                .iter()
                .filter(|i| matches!(i, Item::Circle { .. }))
                .count(),
            4
        );
    }

    #[test]
    fn labels_use_local_normal() {
        // collinear pins on a horizontal line centred at origin: labels go below/above, not along
        let pins = [
            Point::new(-1.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
        ];
        let l = pin_label_positions(&pins, false, Point::new(0.0, -1.0), 0.5);
        assert!((l[1].x).abs() < 1e-12 && (l[1].y - 0.5).abs() < 1e-12);
    }
}
