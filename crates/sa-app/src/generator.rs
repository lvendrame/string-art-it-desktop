//! Generator mode patterns (spec 32): 19 parametric designs that produce Pin
//! Paths + Thread Paths. Port of web `generatorPatterns.ts`.

use std::f64::consts::PI;

use sa_domain::Point;
use sa_domain::generator::*;
use sa_domain::lotus;
use sa_domain::paths::spacing_for_pin_count;

use crate::document::pin_path::{next_path_id, next_pin_id};
use crate::document::{
    Board, BoardShape, Pin, PinPath, PinPathGeometry, PinStyle, SymmetryConfig, ThreadPath,
};
use crate::editor::ThreadDefaults;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FreestyleCircle {
    pub enabled: bool,
    pub nails: f64,
    pub radius_ratio: f64,
    pub center_x_ratio: f64,
    pub center_y_ratio: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AsymmetryLayer {
    pub enabled: bool,
    pub start: f64,
    pub end: f64,
    pub reverse: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RingType {
    Circle,
    Polygon,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeneratorParams {
    Mandala {
        n: f64,
        base: f64,
        layers: f64,
    },
    Star {
        side_nails: f64,
        star_points: f64,
        outer_ratio: f64,
        inner_ratio: f64,
        rotation: f64,
    },
    Freestyle {
        circles: Vec<FreestyleCircle>,
    },
    StarOfDavid {
        depth: f64,
        layer_angle: f64,
        rotation: f64,
        mirror_tiling: bool,
    },
    Spirals {
        arms: f64,
        nails_per_spiral: f64,
        total_turns: f64,
        rotation: f64,
    },
    Wave {
        n: f64,
        base: f64,
        layers: f64,
        layer_fill: f64,
        layer_spread: f64,
    },
    HexagonSpades {
        depth: f64,
        layer_angle: f64,
        rotation: f64,
        mirror_tiling: bool,
    },
    DanceOfPlanets {
        outer_type: RingType,
        outer_nails: f64,
        outer_sides: f64,
        inner_type: RingType,
        inner_nails: f64,
        inner_sides: f64,
        inner_size_ratio: f64,
        rounds: f64,
        reverse: bool,
        rotation: f64,
    },
    Sun {
        side_nails: f64,
        star_points: f64,
        outer_ratio: f64,
        inner_ratio: f64,
        rotation: f64,
        layers: f64,
        layer_spread: f64,
    },
    Vortex {
        sides: f64,
        nails_per_side: f64,
        layers: f64,
        layer_angle: f64,
        rotation: f64,
    },
    Polygon {
        sides: f64,
        nails_per_side: f64,
        bezier_step: f64,
        rotation: f64,
    },
    Flower {
        sides: f64,
        nails_per_side: f64,
        layers: f64,
        rotation: f64,
    },
    Assymetry {
        circle_nails: f64,
        layers: Vec<AsymmetryLayer>,
        rotation: f64,
    },
    Spiral {
        n: f64,
        repetition: f64,
        inner_length: f64,
        rotation: f64,
    },
    MaurerRose {
        n: f64,
        max_steps: f64,
        angle_degrees: f64,
        rotation: f64,
    },
    Comet {
        n: f64,
        layers: f64,
        first_layer_size: f64,
        layer_distance: f64,
        cluster_strength: f64,
        distortion: f64,
        rotation: f64,
    },
    FlowerOfLife {
        levels: f64,
        density: f64,
        rotation: f64,
        ring_enabled: bool,
        ring_nails: f64,
        ring_base: f64,
    },
    Crosses {
        nails_per_line: f64,
        vertical: bool,
        gap: f64,
        sides_rotation: f64,
    },
    Lotus {
        sides: f64,
        density: f64,
        rotation: f64,
        remove_sections: f64,
        render_center: bool,
        center_radius: f64,
        radial_color: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatternId {
    Mandala,
    Star,
    Freestyle,
    StarOfDavid,
    Spirals,
    Wave,
    HexagonSpades,
    DanceOfPlanets,
    Sun,
    Vortex,
    Polygon,
    Flower,
    Assymetry,
    Spiral,
    MaurerRose,
    Comet,
    FlowerOfLife,
    Crosses,
    Lotus,
}

impl PatternId {
    pub const ALL: [PatternId; 19] = [
        PatternId::Mandala,
        PatternId::Star,
        PatternId::Freestyle,
        PatternId::StarOfDavid,
        PatternId::Spirals,
        PatternId::Wave,
        PatternId::HexagonSpades,
        PatternId::DanceOfPlanets,
        PatternId::Sun,
        PatternId::Vortex,
        PatternId::Polygon,
        PatternId::Flower,
        PatternId::Assymetry,
        PatternId::Spiral,
        PatternId::MaurerRose,
        PatternId::Comet,
        PatternId::FlowerOfLife,
        PatternId::Crosses,
        PatternId::Lotus,
    ];

    /// English name used for generated layer names (web `GENERATOR_PATTERN_NAMES`).
    pub fn name(self) -> &'static str {
        use PatternId::*;
        match self {
            Mandala => "Mandala",
            Star => "Star",
            Freestyle => "Freestyle",
            StarOfDavid => "Star of David",
            Spirals => "Spirals",
            Wave => "Wave",
            HexagonSpades => "Hexagon Spades",
            DanceOfPlanets => "Dance of Planets",
            Sun => "Sun",
            Vortex => "Vortex",
            Polygon => "Polygon",
            Flower => "Flower",
            Assymetry => "Assymetry",
            Spiral => "Spiral",
            MaurerRose => "Maurer Rose",
            Comet => "Comet",
            FlowerOfLife => "Flower of Life",
            Crosses => "Crosses",
            Lotus => "Lotus",
        }
    }

    /// i18n label key suffix (web `labelKey`).
    pub fn label_key(self) -> &'static str {
        use PatternId::*;
        match self {
            Mandala => "mandala",
            Star => "star",
            Freestyle => "freestyle",
            StarOfDavid => "starOfDavid",
            Spirals => "spirals",
            Wave => "wave",
            HexagonSpades => "hexagonSpades",
            DanceOfPlanets => "danceOfPlanets",
            Sun => "sun",
            Vortex => "vortex",
            Polygon => "polygon",
            Flower => "flower",
            Assymetry => "assymetry",
            Spiral => "spiral",
            MaurerRose => "maurerRose",
            Comet => "comet",
            FlowerOfLife => "flowerOfLife",
            Crosses => "crosses",
            Lotus => "lotus",
        }
    }

    pub fn default_params(self) -> GeneratorParams {
        use GeneratorParams as G;
        match self {
            PatternId::Mandala => G::Mandala {
                n: 180.0,
                base: 2.0,
                layers: 1.0,
            },
            PatternId::Star => G::Star {
                side_nails: 24.0,
                star_points: 5.0,
                outer_ratio: 1.0,
                inner_ratio: 0.0,
                rotation: 0.0,
            },
            PatternId::Freestyle => G::Freestyle {
                circles: vec![
                    FreestyleCircle {
                        enabled: true,
                        nails: 80.0,
                        radius_ratio: 0.5,
                        center_x_ratio: 0.0,
                        center_y_ratio: -0.5,
                    },
                    FreestyleCircle {
                        enabled: true,
                        nails: 80.0,
                        radius_ratio: 0.5,
                        center_x_ratio: -0.43,
                        center_y_ratio: 0.25,
                    },
                    FreestyleCircle {
                        enabled: true,
                        nails: 80.0,
                        radius_ratio: 0.5,
                        center_x_ratio: 0.43,
                        center_y_ratio: 0.25,
                    },
                ],
            },
            PatternId::StarOfDavid => G::StarOfDavid {
                depth: 10.0,
                layer_angle: 0.063,
                rotation: 0.0,
                mirror_tiling: false,
            },
            PatternId::Spirals => G::Spirals {
                arms: 3.0,
                nails_per_spiral: 80.0,
                total_turns: 0.52,
                rotation: 0.0,
            },
            PatternId::Wave => G::Wave {
                n: 180.0,
                base: 2.0,
                layers: 4.0,
                layer_fill: 60.0,
                layer_spread: 15.0,
            },
            PatternId::HexagonSpades => G::HexagonSpades {
                depth: 8.0,
                layer_angle: 0.05,
                rotation: 0.0,
                mirror_tiling: false,
            },
            PatternId::DanceOfPlanets => G::DanceOfPlanets {
                outer_type: RingType::Circle,
                outer_nails: 120.0,
                outer_sides: 6.0,
                inner_type: RingType::Circle,
                inner_nails: 60.0,
                inner_sides: 6.0,
                inner_size_ratio: 0.4,
                rounds: 2.0,
                reverse: false,
                rotation: 0.0,
            },
            PatternId::Sun => G::Sun {
                side_nails: 50.0,
                star_points: 16.0,
                outer_ratio: 1.0,
                inner_ratio: 0.15,
                rotation: PI,
                layers: 4.0,
                layer_spread: 0.1625,
            },
            PatternId::Vortex => G::Vortex {
                sides: 4.0,
                nails_per_side: 15.0,
                layers: 12.0,
                layer_angle: 0.05,
                rotation: 0.0,
            },
            PatternId::Polygon => G::Polygon {
                sides: 6.0,
                nails_per_side: 20.0,
                bezier_step: 2.0,
                rotation: 0.0,
            },
            PatternId::Flower => G::Flower {
                sides: 6.0,
                nails_per_side: 16.0,
                layers: 5.0,
                rotation: 0.0,
            },
            PatternId::Assymetry => G::Assymetry {
                circle_nails: 137.0,
                layers: vec![
                    AsymmetryLayer {
                        enabled: true,
                        start: 0.25,
                        end: 1.0,
                        reverse: false,
                    },
                    AsymmetryLayer {
                        enabled: true,
                        start: 0.125,
                        end: 0.888,
                        reverse: false,
                    },
                    AsymmetryLayer {
                        enabled: true,
                        start: 0.0,
                        end: 0.826,
                        reverse: true,
                    },
                ],
                rotation: 0.0,
            },
            PatternId::Spiral => G::Spiral {
                n: 180.0,
                repetition: 4.0,
                inner_length: 70.0,
                rotation: 0.0,
            },
            PatternId::MaurerRose => G::MaurerRose {
                n: 7.0,
                max_steps: 180.0,
                angle_degrees: 71.0,
                rotation: 0.0,
            },
            PatternId::Comet => G::Comet {
                n: 150.0,
                layers: 15.0,
                first_layer_size: 70.0,
                layer_distance: 3.0,
                cluster_strength: 0.7,
                distortion: 0.38,
                rotation: 0.0,
            },
            PatternId::FlowerOfLife => G::FlowerOfLife {
                levels: 3.0,
                density: 6.0,
                rotation: 0.0,
                ring_enabled: true,
                ring_nails: 144.0,
                ring_base: 2.0,
            },
            PatternId::Crosses => G::Crosses {
                nails_per_line: 25.0,
                vertical: true,
                gap: 0.27,
                sides_rotation: 0.0,
            },
            PatternId::Lotus => G::Lotus {
                sides: 18.0,
                density: 15.0,
                rotation: 0.0,
                remove_sections: 4.0 / 7.0,
                render_center: true,
                center_radius: 1.0,
                radial_color: false,
            },
        }
    }
}

impl GeneratorParams {
    pub fn id(&self) -> PatternId {
        use GeneratorParams as G;
        match self {
            G::Mandala { .. } => PatternId::Mandala,
            G::Star { .. } => PatternId::Star,
            G::Freestyle { .. } => PatternId::Freestyle,
            G::StarOfDavid { .. } => PatternId::StarOfDavid,
            G::Spirals { .. } => PatternId::Spirals,
            G::Wave { .. } => PatternId::Wave,
            G::HexagonSpades { .. } => PatternId::HexagonSpades,
            G::DanceOfPlanets { .. } => PatternId::DanceOfPlanets,
            G::Sun { .. } => PatternId::Sun,
            G::Vortex { .. } => PatternId::Vortex,
            G::Polygon { .. } => PatternId::Polygon,
            G::Flower { .. } => PatternId::Flower,
            G::Assymetry { .. } => PatternId::Assymetry,
            G::Spiral { .. } => PatternId::Spiral,
            G::MaurerRose { .. } => PatternId::MaurerRose,
            G::Comet { .. } => PatternId::Comet,
            G::FlowerOfLife { .. } => PatternId::FlowerOfLife,
            G::Crosses { .. } => PatternId::Crosses,
            G::Lotus { .. } => PatternId::Lotus,
        }
    }

    /// Most distinct colours the current parameters can use (web `maxGeneratorColours`).
    pub fn max_colours(&self) -> usize {
        use GeneratorParams as G;
        let u = |v: f64| v.max(0.0) as usize;
        match self {
            G::Mandala { layers, .. }
            | G::Wave { layers, .. }
            | G::Vortex { layers, .. }
            | G::Comet { layers, .. } => u(*layers).max(1),
            G::Star { star_points, .. } => u(*star_points) * 3,
            G::Freestyle { .. }
            | G::Spirals { .. }
            | G::DanceOfPlanets { .. }
            | G::Spiral { .. }
            | G::MaurerRose { .. } => 1,
            G::StarOfDavid { .. } => 24,
            G::HexagonSpades { .. } => 18,
            G::Sun {
                star_points,
                layers,
                ..
            } => u(*star_points) * 3 + u(*layers),
            G::Polygon { sides, .. } => u(*sides),
            G::Flower { sides, layers, .. } => u(*sides) * u(*layers),
            G::Assymetry { layers, .. } => layers.iter().filter(|l| l.enabled).count().max(1),
            G::FlowerOfLife {
                levels,
                ring_enabled,
                ..
            } => 6 * u(*levels) * u(*levels) + usize::from(*ring_enabled),
            G::Crosses { .. } => 10,
            G::Lotus {
                sides,
                remove_sections,
                render_center,
                radial_color,
                ..
            } => {
                let s = u(*sides);
                let sections = lotus::sections_count(s);
                let removed = lotus::removed_sections_count(s, *remove_sections);
                lotus::colour_group_count(s, sections, removed, *render_center, *radial_color)
            }
        }
    }
}

/// Conservative inscribed radius for the active board shape.
pub fn max_inscribed_radius(board: &Board) -> f64 {
    let d = &board.dimensions;
    match board.shape {
        BoardShape::Circle => d.diameter.unwrap_or(60.0) / 2.0,
        BoardShape::Oval | BoardShape::Rectangle => {
            d.width.unwrap_or(60.0).min(d.height.unwrap_or(40.0)) / 2.0
        }
        BoardShape::Square => d.side.unwrap_or(50.0) / 2.0,
        BoardShape::Triangle => {
            d.side
                .or(d.base)
                .unwrap_or(50.0)
                .min(d.height.unwrap_or(30.0))
                / 3.0
        }
    }
}

pub struct BuildContext<'a> {
    pub center: Point,
    pub max_radius: f64,
    pub pin_style: &'a PinStyle,
    pub threads: &'a ThreadDefaults,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildResult {
    pub pin_paths: Vec<PinPath>,
    pub thread_paths: Vec<ThreadPath>,
}

impl BuildContext<'_> {
    fn palette(&self) -> Vec<String> {
        if self.threads.colours.is_empty() {
            vec!["#5b8def".into()]
        } else {
            self.threads.colours.clone()
        }
    }

    fn thread(&self, ids: Vec<String>, colours: Vec<String>) -> ThreadPath {
        ThreadPath::new(ids, colours, self.threads.width, self.threads.twist_pitch)
    }

    fn create(&self, g: PinPathGeometry, count_length: f64, count: usize) -> PinPath {
        let spacing = spacing_for_pin_count(count_length, count.max(1)).unwrap_or(1.0);
        PinPath::new(g, spacing, self.pin_style, SymmetryConfig::None)
    }

    /// Pins exactly at `points` (no resampling), as a freehand path.
    fn raw(&self, points: Vec<Point>, spacing: f64) -> PinPath {
        PinPath {
            id: next_path_id(),
            pins: points
                .iter()
                .map(|p| Pin {
                    id: next_pin_id(),
                    x: p.x,
                    y: p.y,
                })
                .collect(),
            geometry: PinPathGeometry::Freehand { points },
            requested_spacing: spacing,
            actual_spacing: spacing,
            guide_visible: self.pin_style.guide_visible,
            colour: self.pin_style.colour.clone(),
            diameter: self.pin_style.diameter,
            symmetry: SymmetryConfig::None,
        }
    }
}

fn pid(p: &PinPath, i: usize) -> String {
    p.pins
        .get(i)
        .or(p.pins.last())
        .map(|q| q.id.clone())
        .unwrap_or_default()
}

fn circle(ctx: &BuildContext, center: Point, radius: f64, nails: usize) -> PinPath {
    ctx.create(
        PinPathGeometry::Circle { center, radius },
        2.0 * PI * radius,
        nails,
    )
}

fn polygon_path(
    ctx: &BuildContext,
    center: Point,
    radius: f64,
    sides: usize,
    rotation: f64,
    per_side: usize,
) -> PinPath {
    let side_len = 2.0 * radius * (PI / sides as f64).sin();
    ctx.create(
        PinPathGeometry::RegularPolygon {
            center,
            radius,
            sides,
            rotation,
        },
        side_len,
        per_side,
    )
}

fn line(ctx: &BuildContext, start: Point, end: Point, segments: usize) -> PinPath {
    ctx.create(
        PinPathGeometry::Line { start, end },
        start.distance(end),
        segments,
    )
}

fn u(v: f64) -> usize {
    js_round(v).max(0.0) as usize
}

fn star(
    ctx: &BuildContext,
    side_nails: usize,
    points: usize,
    outer_ratio: f64,
    inner_ratio: f64,
    rotation: f64,
) -> BuildResult {
    let (inner, outer) = (ctx.max_radius * inner_ratio, ctx.max_radius * outer_ratio);
    let side_nails = side_nails.max(2);
    let circle_path = circle(ctx, ctx.center, ctx.max_radius, points * (side_nails - 1));
    let spokes: Vec<PinPath> = (0..points)
        .map(|s| {
            let a = rotation + 2.0 * PI * s as f64 / points as f64;
            let p = |r: f64| Point::new(ctx.center.x + r * a.cos(), ctx.center.y + r * a.sin());
            ctx.create(
                PinPathGeometry::Line {
                    start: p(inner),
                    end: p(outer),
                },
                outer - inner,
                side_nails - 1,
            )
        })
        .collect();
    let palette = ctx.palette();
    let resolve = |n: &StarWeaveNode| match *n {
        StarWeaveNode::Circle(i) => pid(&circle_path, i),
        StarWeaveNode::Spoke { spoke, index } => pid(&spokes[spoke], index),
    };
    let mut threads = Vec::new();
    for s in 0..points {
        for dir in [1, -1] {
            let ids = star_spoke_circle_zigzag(points, side_nails, s, dir)
                .iter()
                .map(resolve)
                .collect();
            threads.push(ctx.thread(ids, vec![palette[threads.len() % palette.len()].clone()]));
        }
    }
    for s in 0..points {
        let ids = star_adjacent_spoke_zigzag(side_nails, s, (s + 1) % points)
            .iter()
            .map(resolve)
            .collect();
        threads.push(ctx.thread(ids, vec![palette[threads.len() % palette.len()].clone()]));
    }
    let mut pin_paths = vec![circle_path];
    pin_paths.extend(spokes);
    BuildResult {
        pin_paths,
        thread_paths: threads,
    }
}

fn tile_fans(
    ctx: &BuildContext,
    tiles: &[TileRingTile],
    radius_for: impl Fn(&TileRingTile) -> f64,
    layer_angle: f64,
    depth: usize,
) -> BuildResult {
    let pin_paths: Vec<PinPath> = tiles
        .iter()
        .map(|t| {
            let r = radius_for(t);
            let levels =
                nested_polygon_levels(t.sides, r, t.base_rotation, layer_angle, depth, t.direction);
            ctx.raw(
                nested_polygon_vertices(t.center, t.sides, &levels),
                r / depth.max(1) as f64,
            )
        })
        .collect();
    let palette = ctx.palette();
    let mut threads = Vec::new();
    for (ti, t) in tiles.iter().enumerate() {
        for s in 0..t.sides {
            let ids = connect_two_sides_local_indices(t.sides, depth, s)
                .iter()
                .map(|&i| pid(&pin_paths[ti], i))
                .collect();
            threads.push(ctx.thread(ids, vec![palette[threads.len() % palette.len()].clone()]));
        }
    }
    BuildResult {
        pin_paths,
        thread_paths: threads,
    }
}

fn ring_path(
    ctx: &BuildContext,
    ty: RingType,
    sides: usize,
    radius: f64,
    rotation: f64,
    nails: usize,
) -> PinPath {
    match ty {
        RingType::Circle => circle(ctx, ctx.center, radius, nails),
        RingType::Polygon => {
            let sides = sides.max(3);
            polygon_path(
                ctx,
                ctx.center,
                radius,
                sides,
                rotation,
                (js_round(nails as f64 / sides as f64) as usize).max(1),
            )
        }
    }
}

fn polygon_core(
    ctx: &BuildContext,
    sides: usize,
    per_side: usize,
    bezier_step: usize,
    rotation: f64,
) -> BuildResult {
    let path = polygon_path(ctx, ctx.center, ctx.max_radius, sides, rotation, per_side);
    let palette = ctx.palette();
    let mut threads = Vec::new();
    for s in 0..sides {
        let other = (s + bezier_step) % sides;
        let ids = same_index_zigzag(per_side, per_side)
            .iter()
            .map(|(b, i)| pid(&path, if *b { other } else { s } * per_side + i))
            .collect();
        threads.push(ctx.thread(ids, vec![palette[threads.len() % palette.len()].clone()]));
    }
    BuildResult {
        pin_paths: vec![path],
        thread_paths: threads,
    }
}

/// Builds `params` (web `buildGeneratorPattern`).
pub fn build(params: &GeneratorParams, ctx: &BuildContext) -> BuildResult {
    use GeneratorParams as G;
    let palette = ctx.palette();
    let c = ctx.center;
    let r = ctx.max_radius;
    match params {
        G::Mandala { n, base, layers } => {
            let path = circle(ctx, c, r, u(*n));
            let threads = mandala_layer_sequences(u(*n), u(*base), u(*layers))
                .into_iter()
                .enumerate()
                .map(|(i, l)| {
                    ctx.thread(
                        l.iter().map(|&k| pid(&path, k)).collect(),
                        vec![palette[i % palette.len()].clone()],
                    )
                })
                .collect();
            BuildResult {
                pin_paths: vec![path],
                thread_paths: threads,
            }
        }
        G::Star {
            side_nails,
            star_points,
            outer_ratio,
            inner_ratio,
            rotation,
        } => star(
            ctx,
            u(*side_nails),
            u(*star_points).max(1),
            *outer_ratio,
            *inner_ratio,
            *rotation,
        ),
        G::Freestyle { circles } => {
            let paths: Vec<PinPath> = circles
                .iter()
                .filter(|k| k.enabled && k.nails > 0.0)
                .map(|k| {
                    let center = Point::new(c.x + k.center_x_ratio * r, c.y + k.center_y_ratio * r);
                    circle(ctx, center, r * k.radius_ratio, u(k.nails))
                })
                .collect();
            let ids: Vec<String> =
                round_robin_sequence(&paths.iter().map(|p| p.pins.len()).collect::<Vec<_>>())
                    .iter()
                    .map(|s| pid(&paths[s.group], s.local))
                    .collect();
            let threads = if ids.len() >= 2 {
                vec![ctx.thread(ids, ctx.threads.colours.clone())]
            } else {
                vec![]
            };
            BuildResult {
                pin_paths: paths,
                thread_paths: threads,
            }
        }
        G::StarOfDavid {
            depth,
            layer_angle,
            rotation,
            mirror_tiling,
        } => {
            let inner_hex = r / 3f64.sqrt();
            let tri = r / 3.0;
            let helper = inner_hex * (PI / 6.0).cos() + tri / 2.0;
            let mut tiles = vec![TileRingTile {
                sides: 6,
                center: c,
                base_rotation: *rotation,
                direction: 1.0,
            }];
            tiles.extend(tile_ring_layout(
                6,
                3,
                helper,
                PI / 6.0,
                *rotation,
                c,
                *mirror_tiling,
            ));
            tile_fans(
                ctx,
                &tiles,
                |t| if t.sides == 6 { inner_hex } else { tri },
                *layer_angle,
                u(*depth).max(1),
            )
        }
        G::HexagonSpades {
            depth,
            layer_angle,
            rotation,
            mirror_tiling,
        } => {
            let tiles = tile_ring_layout(6, 3, r * 0.5, PI / 6.0, *rotation, c, *mirror_tiling);
            tile_fans(ctx, &tiles, |_| r * 0.5, *layer_angle, u(*depth).max(1))
        }
        G::Spirals {
            arms,
            nails_per_spiral,
            total_turns,
            rotation,
        } => {
            let pts = spiral_arm_points(
                u(*arms),
                u(*nails_per_spiral),
                *total_turns,
                *rotation,
                r,
                c,
            );
            let path = ctx.raw(pts, r / (u(*nails_per_spiral).max(2) - 1) as f64);
            let ids = path.pins.iter().map(|p| p.id.clone()).collect();
            BuildResult {
                thread_paths: vec![ctx.thread(ids, ctx.threads.colours.clone())],
                pin_paths: vec![path],
            }
        }
        G::Wave {
            n,
            base,
            layers,
            layer_fill,
            layer_spread,
        } => {
            let path = circle(ctx, c, r, u(*n));
            let threads = wave_layer_sequences(
                u(*n),
                *base as i64,
                u(*layers),
                *layer_fill as i64,
                *layer_spread as i64,
            )
            .into_iter()
            .enumerate()
            .map(|(i, l)| {
                ctx.thread(
                    l.iter().map(|&k| pid(&path, k)).collect(),
                    vec![palette[i % palette.len()].clone()],
                )
            })
            .collect();
            BuildResult {
                pin_paths: vec![path],
                thread_paths: threads,
            }
        }
        G::DanceOfPlanets {
            outer_type,
            outer_nails,
            outer_sides,
            inner_type,
            inner_nails,
            inner_sides,
            inner_size_ratio,
            rounds,
            reverse,
            rotation,
        } => {
            let outer = ring_path(
                ctx,
                *outer_type,
                u(*outer_sides),
                r,
                *rotation,
                u(*outer_nails),
            );
            let inner = ring_path(
                ctx,
                *inner_type,
                u(*inner_sides),
                r * inner_size_ratio,
                *rotation,
                u(*inner_nails),
            );
            let ids: Vec<String> =
                dance_of_planets_walk(outer.pins.len(), inner.pins.len(), u(*rounds), *reverse)
                    .iter()
                    .map(|(is_inner, i)| pid(if *is_inner { &inner } else { &outer }, *i))
                    .collect();
            let threads = if ids.len() >= 2 {
                vec![ctx.thread(ids, ctx.threads.colours.clone())]
            } else {
                vec![]
            };
            BuildResult {
                pin_paths: vec![outer, inner],
                thread_paths: threads,
            }
        }
        G::Sun {
            side_nails,
            star_points,
            outer_ratio,
            inner_ratio,
            rotation,
            layers,
            layer_spread,
        } => {
            let points = u(*star_points).max(1);
            let mut out = star(
                ctx,
                u(*side_nails),
                points,
                *outer_ratio,
                *inner_ratio,
                *rotation,
            );
            for layer in 1..=u(*layers) {
                let ratio = (outer_ratio * (1.0 - layer_spread * layer as f64)).max(0.05);
                let inner_r = if *inner_ratio == 0.0 {
                    0.4
                } else {
                    *inner_ratio
                };
                let g = PinPathGeometry::Star {
                    center: c,
                    outer_radius: r * ratio,
                    inner_radius: r * ratio * inner_r.max(0.1),
                    points,
                    rotation: *rotation,
                };
                let sp = ctx.create(g, 2.0 * PI * r * ratio, points * 2);
                let ring = &mandala_layer_sequences(sp.pins.len(), 2, 1)[0];
                let ids = ring.iter().map(|&k| pid(&sp, k)).collect();
                let colour = palette[out.thread_paths.len() % palette.len()].clone();
                out.thread_paths.push(ctx.thread(ids, vec![colour]));
                out.pin_paths.push(sp);
            }
            out
        }
        G::Vortex {
            sides,
            nails_per_side,
            layers,
            layer_angle,
            rotation,
        } => {
            let (sides, per) = (u(*sides).max(3), u(*nails_per_side).max(1));
            let levels =
                nested_polygon_levels(sides, r, *rotation, *layer_angle, u(*layers).max(1), 1.0);
            let paths: Vec<PinPath> = levels
                .iter()
                .map(|l| polygon_path(ctx, c, l.radius, sides, l.rotation, per))
                .collect();
            let mut threads = Vec::new();
            for (lv, p) in paths.iter().enumerate() {
                for i in 0..per {
                    let ids = (0..=sides).map(|s| pid(p, (s % sides) * per + i)).collect();
                    threads.push(ctx.thread(ids, vec![palette[lv % palette.len()].clone()]));
                }
            }
            BuildResult {
                pin_paths: paths,
                thread_paths: threads,
            }
        }
        G::Polygon {
            sides,
            nails_per_side,
            bezier_step,
            rotation,
        } => polygon_core(
            ctx,
            u(*sides).max(3),
            u(*nails_per_side).max(1),
            u(*bezier_step),
            *rotation,
        ),
        G::Flower {
            sides,
            nails_per_side,
            layers,
            rotation,
        } => {
            let (sides, per, layers) = (
                u(*sides).max(3),
                u(*nails_per_side).max(3),
                u(*layers).max(1),
            );
            let spoke_len = r * 0.6;
            let mut out = BuildResult::default();
            for layer in 0..layers {
                let lr = rotation + layer as f64 * 2.0 * PI / (sides * layers) as f64;
                let boundary = polygon_path(ctx, c, r, sides, lr, per);
                let spokes: Vec<PinPath> = (0..sides)
                    .map(|s| {
                        let a = lr + 2.0 * PI * (s as f64 + 0.5) / sides as f64 - PI / 2.0;
                        let end = Point::new(c.x + spoke_len * a.cos(), c.y + spoke_len * a.sin());
                        line(ctx, c, end, per.saturating_sub(2).max(1))
                    })
                    .collect();
                for (s, spoke) in spokes.iter().enumerate() {
                    let (prev, next) = ((s + sides - 1) % sides, (s + 1) % sides);
                    let ids = flower_petal_weave(per)
                        .iter()
                        .map(|(w, i)| match w {
                            PetalSide::Centre => {
                                pid(spoke, (*i).min(spoke.pins.len().saturating_sub(1)))
                            }
                            PetalSide::Prev => pid(&boundary, prev * per + i),
                            PetalSide::Next => pid(&boundary, next * per + i),
                            PetalSide::Side => pid(&boundary, s * per + i),
                        })
                        .collect();
                    let colour = palette[out.thread_paths.len() % palette.len()].clone();
                    out.thread_paths.push(ctx.thread(ids, vec![colour]));
                }
                out.pin_paths.push(boundary);
                out.pin_paths.extend(spokes);
            }
            out
        }
        G::Assymetry {
            circle_nails,
            layers,
            rotation,
        } => {
            let n = u(*circle_nails).max(1);
            let circle_path = circle(ctx, c, r, n);
            let spoke_n = (js_round(n as f64 / (2.0 * PI)) as usize).max(1);
            let end = Point::new(c.x + r * rotation.cos(), c.y + r * rotation.sin());
            let spoke = line(ctx, c, end, spoke_n);
            let threads = layers
                .iter()
                .filter(|l| l.enabled)
                .enumerate()
                .filter_map(|(i, l)| {
                    let ids: Vec<String> = asymmetry_zigzag(n, spoke_n, l.start, l.end, l.reverse)
                        .iter()
                        .map(|(is_spoke, k)| pid(if *is_spoke { &spoke } else { &circle_path }, *k))
                        .collect();
                    (ids.len() >= 2)
                        .then(|| ctx.thread(ids, vec![palette[i % palette.len()].clone()]))
                })
                .collect();
            BuildResult {
                pin_paths: vec![circle_path, spoke],
                thread_paths: threads,
            }
        }
        G::Spiral {
            n,
            repetition,
            inner_length,
            rotation,
        } => {
            let nn = u(*n).max(1);
            let path = circle(ctx, c, r, nn);
            let shift = js_round(rotation / (2.0 * PI) * nn as f64) as i64;
            let ids: Vec<String> = spiral_decaying_walk(nn, u(*repetition), *inner_length)
                .iter()
                .map(|&i| {
                    pid(
                        &path,
                        (((i as i64 + shift) % nn as i64 + nn as i64) % nn as i64) as usize,
                    )
                })
                .collect();
            let threads = if ids.len() >= 2 {
                vec![ctx.thread(ids, ctx.threads.colours.clone())]
            } else {
                vec![]
            };
            BuildResult {
                pin_paths: vec![path],
                thread_paths: threads,
            }
        }
        G::MaurerRose {
            n,
            max_steps,
            angle_degrees,
            rotation,
        } => {
            let steps = u(*max_steps).max(1);
            let path = ctx.raw(
                maurer_rose_points(*n, steps, *angle_degrees, *rotation, r, c),
                r / steps as f64,
            );
            let ids = path.pins.iter().map(|p| p.id.clone()).collect();
            BuildResult {
                thread_paths: vec![ctx.thread(ids, ctx.threads.colours.clone())],
                pin_paths: vec![path],
            }
        }
        G::Comet {
            n,
            layers,
            first_layer_size,
            layer_distance,
            cluster_strength,
            distortion,
            rotation,
        } => {
            let nn = u(*n).max(2);
            let ry = r * (1.0 - distortion.clamp(0.0, 0.9));
            let pts = (0..nn)
                .map(|i| {
                    let a = rotation
                        + cluster_fraction(i as f64 / nn as f64, *cluster_strength) * 2.0 * PI;
                    Point::new(c.x + r * a.cos(), c.y + ry * a.sin())
                })
                .collect();
            let path = ctx.raw(pts, 2.0 * PI * r / nn as f64);
            let threads = comet_layer_sequences(
                nn,
                u(*layers),
                *first_layer_size as i64,
                *layer_distance as i64,
            )
            .into_iter()
            .enumerate()
            .map(|(i, l)| {
                ctx.thread(
                    l.iter().map(|&k| pid(&path, k)).collect(),
                    vec![palette[i % palette.len()].clone()],
                )
            })
            .collect();
            BuildResult {
                pin_paths: vec![path],
                thread_paths: threads,
            }
        }
        G::FlowerOfLife {
            levels,
            density,
            rotation,
            ring_enabled,
            ring_nails,
            ring_base,
        } => {
            let (lv, d) = (u(*levels).max(1), u(*density).max(1));
            let cells = hex_flower_grid(lv, r, *rotation, c);
            let tri_r = r / lv as f64 / 3f64.sqrt();
            let side = tri_r * 3f64.sqrt();
            let mut out = BuildResult::default();
            for (ci, cell) in cells.iter().enumerate() {
                let tp = ctx.create(
                    PinPathGeometry::RegularPolygon {
                        center: cell.center,
                        radius: tri_r,
                        sides: 3,
                        rotation: cell.rotation,
                    },
                    side,
                    d,
                );
                let seg = |a: usize, b: usize| {
                    same_index_zigzag(d, d)
                        .into_iter()
                        .map(move |(is_b, i)| (if is_b { b } else { a }) + i)
                };
                let ids = seg(0, d)
                    .chain(seg(d, 2 * d))
                    .chain(seg(2 * d, 0))
                    .map(|i| pid(&tp, i))
                    .collect();
                out.thread_paths
                    .push(ctx.thread(ids, vec![palette[ci % palette.len()].clone()]));
                out.pin_paths.push(tp);
            }
            if *ring_enabled {
                let rn = u(*ring_nails).max(1);
                let ring = circle(ctx, c, r, rn);
                let seq = &mandala_layer_sequences(rn, u(*ring_base), 1)[0];
                let ids = seq.iter().map(|&k| pid(&ring, k)).collect();
                let colour = palette[out.thread_paths.len() % palette.len()].clone();
                out.thread_paths.push(ctx.thread(ids, vec![colour]));
                out.pin_paths.push(ring);
            }
            out
        }
        G::Crosses {
            nails_per_line,
            vertical,
            gap,
            sides_rotation,
        } => {
            let n = u(*nails_per_line).max(2);
            let total = 2.0 * r;
            let seg_gap = gap * (total * 0.8) / 3.0;
            let seg_len = ((total - 3.0 * seg_gap) / 4.0).max(1e-6);
            let start0 = -total / 2.0;
            let axis = |l: f64, w: f64| {
                if *vertical {
                    Point::new(c.x + w, c.y + l)
                } else {
                    Point::new(c.x + l, c.y + w)
                }
            };
            let rot = |p: Point, pv: Point, a: f64| sa_domain::transforms::rotate_point(p, pv, a);
            let spans: Vec<(f64, f64)> = (0..4)
                .map(|k| {
                    let s = start0 + k as f64 * (seg_len + seg_gap);
                    (s, s + seg_len)
                })
                .collect();
            let spine: Vec<PinPath> = spans
                .iter()
                .map(|(s, e)| line(ctx, axis(*s, 0.0), axis(*e, 0.0), n - 1))
                .collect();
            let arm = gap * seg_len;
            let base_rot = if *vertical {
                *sides_rotation
            } else {
                -sides_rotation
            };
            let rows: Vec<(PinPath, PinPath)> = (0..3)
                .map(|row| {
                    let top = spans[row + 1].0 - seg_gap / 2.0;
                    let sign = match row {
                        0 => 1.0,
                        2 => -1.0,
                        _ => 0.0,
                    };
                    let (pl, pr) = (axis(top, -arm), axis(top, arm));
                    let ol = rot(axis(top, -arm - seg_len), pl, sign * base_rot);
                    let or = rot(axis(top, arm + seg_len), pr, -sign * base_rot);
                    (line(ctx, ol, pl, n - 1), line(ctx, or, pr, n - 1))
                })
                .collect();
            let conns = [
                (0, 0),
                (0, 1),
                (1, 0),
                (1, 1),
                (3, 2),
                (3, 1),
                (2, 1),
                (2, 2),
                (1, 2),
                (2, 0),
            ];
            let mut threads = Vec::new();
            for (si, row) in conns {
                let (l, rr) = &rows[row];
                let ids = crosses_weave(n, row < si)
                    .iter()
                    .map(|(w, i)| {
                        pid(
                            match w {
                                CrossSide::Spine => &spine[si],
                                CrossSide::Left => l,
                                CrossSide::Right => rr,
                            },
                            *i,
                        )
                    })
                    .collect();
                threads.push(ctx.thread(ids, vec![palette[threads.len() % palette.len()].clone()]));
            }
            let mut pin_paths = spine;
            for (l, rr) in rows {
                pin_paths.push(l);
                pin_paths.push(rr);
            }
            BuildResult {
                pin_paths,
                thread_paths: threads,
            }
        }
        G::Lotus {
            sides,
            density,
            rotation,
            remove_sections,
            render_center,
            center_radius,
            radial_color,
        } => {
            let sides = u(*sides).max(3);
            let sections = lotus::sections_count(sides);
            let removed = lotus::removed_sections_count(sides, *remove_sections);
            let fit = lotus::fit(sides, *density, r / 2.0, removed);
            let petals: Vec<PinPath> = (0..sides)
                .map(|j| {
                    let pc = lotus::petal_center(sides, *rotation, fit.radius, c, j);
                    let pts = (0..fit.n)
                        .map(|k| lotus::petal_point(&fit, sides, *rotation, pc, j, k))
                        .collect();
                    ctx.raw(pts, fit.radius / fit.n.max(1) as f64)
                })
                .collect();
            let has_circle = *render_center && *center_radius > 0.0;
            let centre = render_center.then(|| {
                let pts = if has_circle {
                    let cr = center_radius
                        * lotus::max_center_radius(&fit, sides, *rotation, c, sections, removed);
                    (0..sides)
                        .map(|k| lotus::center_circle_point(sides, *rotation, cr, c, k))
                        .collect()
                } else {
                    vec![c]
                };
                ctx.raw(pts, (fit.radius / sides as f64).max(1e-6))
            });
            let resolve = |n: &lotus::LotusNode| match *n {
                lotus::LotusNode::Petal { circle, index } => pid(&petals[circle], index),
                lotus::LotusNode::Center(i) => {
                    centre.as_ref().map(|p| pid(p, i)).unwrap_or_default()
                }
            };
            let mut threads = Vec::new();
            for patch in
                lotus::generate_patches(sides, removed, sections, *render_center, *radial_color)
            {
                let plan =
                    lotus::draw_patch(sides, fit.p, fit.n, removed, sections, has_circle, patch);
                let sources: Vec<&lotus::LotusNode> = plan.fan1.iter().chain(&plan.fan2).collect();
                let mut nodes = Vec::new();
                for (i, s) in sources.iter().enumerate() {
                    if i > 0 {
                        nodes.push(&plan.target);
                    }
                    nodes.push(*s);
                }
                let ids: Vec<String> = nodes.into_iter().map(resolve).collect();
                if ids.len() < 2 {
                    continue;
                }
                let ci = lotus::patch_colour_index(&patch, removed, *radial_color);
                threads.push(ctx.thread(ids, vec![palette[ci % palette.len()].clone()]));
            }
            let mut pin_paths = petals;
            pin_paths.extend(centre);
            BuildResult {
                pin_paths,
                thread_paths: threads,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pattern_builds_with_valid_thread_references() {
        let style = PinStyle {
            colour: "#fff".into(),
            diameter: 2.0,
            guide_visible: true,
        };
        let td = ThreadDefaults::default();
        let ctx = BuildContext {
            center: Point::ORIGIN,
            max_radius: 30.0,
            pin_style: &style,
            threads: &td,
        };
        for id in PatternId::ALL {
            let out = build(&id.default_params(), &ctx);
            assert!(!out.pin_paths.is_empty(), "{id:?} pins");
            assert!(!out.thread_paths.is_empty(), "{id:?} threads");
            let ids: std::collections::HashSet<&str> = out
                .pin_paths
                .iter()
                .flat_map(|p| p.pins.iter().map(|q| q.id.as_str()))
                .collect();
            for t in &out.thread_paths {
                assert!(
                    t.pin_ids.iter().all(|i| ids.contains(i.as_str())),
                    "{id:?} dangling pin ref"
                );
            }
            assert!(id.default_params().max_colours() >= 1);
        }
    }

    #[test]
    fn mandala_exact_pin_count() {
        let style = PinStyle {
            colour: "#fff".into(),
            diameter: 2.0,
            guide_visible: true,
        };
        let td = ThreadDefaults::default();
        let ctx = BuildContext {
            center: Point::ORIGIN,
            max_radius: 30.0,
            pin_style: &style,
            threads: &td,
        };
        let out = build(
            &GeneratorParams::Mandala {
                n: 180.0,
                base: 2.0,
                layers: 3.0,
            },
            &ctx,
        );
        assert_eq!(out.pin_paths[0].pins.len(), 180);
        assert_eq!(out.thread_paths.len(), 3);
        assert_eq!(out.thread_paths[0].pin_ids.len(), 360);
    }
}

impl crate::editor::Store {
    /// Generate / Re-generate: replaces the transient draft (not undoable).
    pub fn generate_pattern(&mut self, params: GeneratorParams, colours: Vec<String>) {
        let threads = ThreadDefaults {
            colours,
            ..self.editor.thread_defaults.clone()
        };
        let style = self.editor.pin_defaults.style();
        let ctx = BuildContext {
            center: Point::ORIGIN,
            max_radius: max_inscribed_radius(&self.doc.board),
            pin_style: &style,
            threads: &threads,
        };
        let out = build(&params, &ctx);
        self.editor.generator_draft = Some((params, out));
    }

    /// Confirm: the one undoable step — two new layers named after the pattern.
    pub fn confirm_generated_pattern(&mut self) {
        let Some((params, out)) = self.editor.generator_draft.take() else {
            return;
        };
        let name = format!("Generated — {}", params.id().name());
        let mut pl = crate::document::PinLayer::new(name.clone());
        pl.pin_paths = out.pin_paths;
        let mut tl = crate::document::ThreadLayer::new(name);
        tl.thread_paths = out.thread_paths;
        let (pid, tid) = (pl.id.clone(), tl.id.clone());
        self.commit(|d| {
            d.pin_layers.push(pl);
            d.thread_layers.push(tl);
        });
        self.editor.active_pin_layer_id = pid;
        self.editor.active_thread_layer_id = tid;
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;
    use crate::editor::{EditorMode, Store};

    #[test]
    fn generate_then_confirm_is_one_undo_step() {
        let mut s = Store::default();
        s.set_mode(EditorMode::Generate);
        s.generate_pattern(PatternId::Mandala.default_params(), vec!["#f00".into()]);
        assert!(s.editor.generator_draft.is_some());
        assert!(!s.can_undo());
        s.confirm_generated_pattern();
        assert_eq!(s.doc.pin_layers.len(), 2);
        assert_eq!(s.doc.pin_layers[1].name, "Generated — Mandala");
        s.undo();
        assert_eq!(s.doc.pin_layers.len(), 1);
        s.generate_pattern(PatternId::Star.default_params(), vec![]);
        s.set_mode(EditorMode::Pin);
        assert!(s.editor.generator_draft.is_none());
    }
}

#[cfg(test)]
mod parity_dump {
    use super::*;

    /// Writes per-pattern pin/thread counts to $STRINGARTIT_GEN_DUMP for JS parity diffing.
    #[test]
    fn dump() {
        let Some(path) = std::env::var_os("STRINGARTIT_GEN_DUMP") else {
            return;
        };
        let style = PinStyle {
            colour: "#fff".into(),
            diameter: 2.0,
            guide_visible: true,
        };
        let td = ThreadDefaults {
            colours: ["#a", "#b", "#c", "#d"].map(String::from).to_vec(),
            width: 1.5,
            twist_pitch: 6.0,
        };
        let ctx = BuildContext {
            center: Point::ORIGIN,
            max_radius: 30.0,
            pin_style: &style,
            threads: &td,
        };
        let rows: Vec<serde_json::Value> = PatternId::ALL
            .iter()
            .map(|id| {
                let r = build(&id.default_params(), &ctx);
                let r3 = |v: f64| (v * 1000.0).round() / 1000.0 + 0.0;
                serde_json::json!({
                    "pins": r.pin_paths.iter().map(|p| p.pins.len()).collect::<Vec<_>>(),
                    "threads": r.thread_paths.iter().map(|t| t.pin_ids.len()).collect::<Vec<_>>(),
                    "first": r.pin_paths.iter().map(|p| [r3(p.pins[0].x), r3(p.pins[0].y)]).collect::<Vec<_>>(),
                    "coords": r.pin_paths.iter().map(|p| p.pins.iter().map(|q| [r3(q.x), r3(q.y)]).collect::<Vec<_>>()).collect::<Vec<_>>(),
                    "seq": r.thread_paths.iter().map(|t| t.pin_ids.iter().map(|id| {
                        r.pin_paths.iter().enumerate().find_map(|(pi, p)| p.pins.iter().position(|q| &q.id == id).map(|k| [pi, k])).unwrap()
                    }).collect::<Vec<_>>()).collect::<Vec<_>>(),
                    "colours": r.thread_paths.iter().map(|t| t.colours.clone()).collect::<Vec<_>>(),
                })
            })
            .collect();
        std::fs::write(path, serde_json::to_string(&rows).unwrap()).unwrap();
    }
}
