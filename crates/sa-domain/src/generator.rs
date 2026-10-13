//! Generator-mode index maths (spec 32): pure, pin-count driven sequences.
//! Port of web `src/domain/generator/*` (Lotus lives in `lotus.rs`).

use std::f64::consts::PI;

use crate::Point;

fn wrap(i: i64, n: i64) -> i64 {
    ((i % n) + n) % n
}

/// JS `Math.round` (half toward +∞).
pub fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoundRobinStep {
    pub group: usize,
    pub local: usize,
}

pub fn round_robin_sequence(counts: &[usize]) -> Vec<RoundRobinStep> {
    let rounds = counts.iter().copied().filter(|c| *c > 0).max().unwrap_or(0);
    let mut steps = Vec::new();
    for r in 0..rounds {
        for (group, &count) in counts.iter().enumerate() {
            if count > 0 {
                steps.push(RoundRobinStep {
                    group,
                    local: r % count,
                });
            }
        }
    }
    steps
}

/// Mandala: `(i·base) mod n`, each layer shifted by `floor(n/layers)`.
pub fn mandala_layer_sequences(n: usize, base: usize, layers: usize) -> Vec<Vec<usize>> {
    if n == 0 || layers == 0 {
        return Vec::new();
    }
    let shift_step = n / layers;
    (0..layers)
        .map(|layer| {
            let shift = shift_step * layer;
            (0..n)
                .flat_map(|i| [(i + shift) % n, ((i * base) % n + shift) % n])
                .collect()
        })
        .collect()
}

pub fn spiral_arm_points(
    arms: usize,
    nails_per_spiral: usize,
    total_turns: f64,
    rotation: f64,
    max_radius: f64,
    center: Point,
) -> Vec<Point> {
    if arms == 0 || nails_per_spiral < 2 {
        return Vec::new();
    }
    let mut pts = Vec::new();
    for i in 1..nails_per_spiral {
        let p = i as f64 / (nails_per_spiral - 1) as f64;
        let radius = p * max_radius;
        for s in 0..arms {
            let a = rotation + p * total_turns * 2.0 * PI + 2.0 * PI * s as f64 / arms as f64;
            pts.push(Point::new(
                center.x + radius * a.cos(),
                center.y + radius * a.sin(),
            ));
        }
    }
    pts
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NestedPolygonLevel {
    pub radius: f64,
    pub rotation: f64,
}

pub fn nested_polygon_levels(
    sides: usize,
    base_radius: f64,
    base_rotation: f64,
    layer_angle: f64,
    depth: usize,
    direction: f64,
) -> Vec<NestedPolygonLevel> {
    if sides < 3 || depth < 1 {
        return Vec::new();
    }
    let n = sides as f64;
    let interior = (n - 2.0) * PI / n;
    let change = ((1.0 - 2.0 * layer_angle) / (interior / 2.0).tan()).atan();
    let inner_angle = PI / n - change;
    let scale = (PI / n).cos() / change.cos();
    (0..depth)
        .map(|q| NestedPolygonLevel {
            radius: base_radius * scale.powi(q as i32),
            rotation: base_rotation + direction * inner_angle * q as f64,
        })
        .collect()
}

pub fn nested_polygon_vertices(
    center: Point,
    sides: usize,
    levels: &[NestedPolygonLevel],
) -> Vec<Point> {
    levels
        .iter()
        .flat_map(|l| {
            (0..sides).map(move |s| {
                let a = l.rotation + 2.0 * PI * s as f64 / sides as f64 - PI / 2.0;
                Point::new(center.x + l.radius * a.cos(), center.y + l.radius * a.sin())
            })
        })
        .collect()
}

pub fn connect_two_sides_local_indices(sides: usize, depth: usize, s: usize) -> Vec<usize> {
    let next = (s + 1) % sides;
    (0..depth)
        .flat_map(|q| {
            let (a, b) = (q * sides + s, q * sides + next);
            if q % 2 == 0 { [a, b] } else { [b, a] }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StarWeaveNode {
    Circle(usize),
    Spoke { spoke: usize, index: usize },
}

pub fn star_spoke_circle_zigzag(
    sides: usize,
    side_nails: usize,
    spoke: usize,
    direction: i64,
) -> Vec<StarWeaveNode> {
    if side_nails < 2 {
        return Vec::new();
    }
    let total = (sides * (side_nails - 1)) as i64;
    let pivot = (spoke * (side_nails - 1)) as i64;
    let mut v = vec![StarWeaveNode::Circle(wrap(
        if direction == 1 { pivot } else { pivot + 1 },
        total,
    ) as usize)];
    for k in 0..(side_nails - 1) as i64 {
        v.push(StarWeaveNode::Spoke {
            spoke,
            index: k as usize,
        });
        v.push(StarWeaveNode::Circle(wrap(
            if direction == 1 {
                pivot + k + 1
            } else {
                pivot - k
            },
            total,
        ) as usize));
    }
    v
}

pub fn star_adjacent_spoke_zigzag(side_nails: usize, a: usize, b: usize) -> Vec<StarWeaveNode> {
    if side_nails < 2 {
        return Vec::new();
    }
    let mut v = vec![StarWeaveNode::Spoke {
        spoke: b,
        index: side_nails - 1,
    }];
    for k in 0..side_nails - 1 {
        v.push(StarWeaveNode::Spoke { spoke: a, index: k });
        v.push(StarWeaveNode::Spoke {
            spoke: b,
            index: side_nails - 2 - k,
        });
    }
    v
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileRingTile {
    pub sides: usize,
    pub center: Point,
    pub base_rotation: f64,
    pub direction: f64,
}

pub fn tile_ring_layout(
    count: usize,
    sides: usize,
    helper_radius: f64,
    angle_offset: f64,
    rotation: f64,
    center: Point,
    mirror: bool,
) -> Vec<TileRingTile> {
    (0..count)
        .map(|t| {
            let step = t as f64 * 2.0 * PI / count as f64;
            let pa = rotation + angle_offset + step - PI / 2.0;
            TileRingTile {
                sides,
                center: Point::new(
                    center.x + helper_radius * pa.cos(),
                    center.y + helper_radius * pa.sin(),
                ),
                base_rotation: rotation + angle_offset + step,
                direction: if mirror { 1.0 } else { -1.0 },
            }
        })
        .collect()
}

/// `(is_b, index)`.
pub fn two_ray_zigzag(count_a: usize, count_b: usize) -> Vec<(bool, usize)> {
    if count_a < 1 || count_b < 1 {
        return Vec::new();
    }
    let steps = count_a.min(count_b);
    let mut v = vec![(true, count_b - 1)];
    for k in 0..steps - 1 {
        v.push((false, k));
        v.push((true, count_b - 2 - k));
    }
    v
}

pub fn same_index_zigzag(count_a: usize, count_b: usize) -> Vec<(bool, usize)> {
    if count_a < 1 || count_b < 1 {
        return Vec::new();
    }
    let count = count_a.min(count_b);
    let mut v = Vec::new();
    let mut k = 0;
    while k + 1 < count {
        let k1 = (k + 1).min(count - 1);
        v.extend([(false, k), (true, k), (true, k1), (false, k1)]);
        k += 2;
    }
    v
}

pub fn wave_layer_sequences(
    n: usize,
    base: i64,
    layers: usize,
    layer_fill: i64,
    layer_spread: i64,
) -> Vec<Vec<usize>> {
    if n == 0 || layers == 0 {
        return Vec::new();
    }
    let ni = n as i64;
    let fill = layer_fill.max(1).min(ni);
    (0..layers as i64)
        .map(|layer| {
            let shift = layer_spread * layer;
            let mut v = Vec::new();
            let mut k = 1;
            while k < fill {
                let m = (k * base) % ni + shift;
                v.extend(
                    [
                        wrap(k + shift, ni),
                        wrap(m, ni),
                        wrap(m + base, ni),
                        wrap(k + shift + 1, ni),
                    ]
                    .map(|x| x as usize),
                );
                k += 2;
            }
            v
        })
        .collect()
}

/// `(is_spoke, index)`.
pub fn asymmetry_zigzag(
    circle: usize,
    spoke: usize,
    start_fraction: f64,
    end_fraction: f64,
    reverse: bool,
) -> Vec<(bool, usize)> {
    if circle < 1 || spoke < 1 {
        return Vec::new();
    }
    let total = (circle + spoke) as i64;
    let node = |g: i64| {
        let i = wrap(g, total) as usize;
        if i < circle {
            (false, i)
        } else {
            (true, i - circle)
        }
    };
    let point = |i: i64| if reverse { total - i } else { i };
    let start = js_round(start_fraction * total as f64) as i64;
    let end = (js_round(end_fraction * total as f64) as i64 - start).max(0);
    let advance = if reverse { -1 } else { 1 };
    let mut v = vec![node(point(0))];
    let mut prev = point(0);
    let mut prev_side = false;
    for index in 0..=end {
        if index != 0 {
            v.push(node(prev + advance));
        }
        prev = point(if prev_side { index } else { index + start });
        v.push(node(prev));
        prev_side = !prev_side;
    }
    v
}

pub fn spiral_decaying_walk(n: usize, repetition: usize, inner_length: f64) -> Vec<usize> {
    if n == 0 || repetition == 0 || inner_length <= 0.0 {
        return Vec::new();
    }
    let real = 2 * repetition as i64 - 1;
    let mut span = js_round(inner_length) as i64;
    let (mut count, mut point, mut prev) = (0i64, 0i64, false);
    let mut v = vec![0];
    while span > 0 {
        point = if prev { point - span + 1 } else { point + span };
        if count == real {
            span -= 1;
            count = 0;
            point += 1;
        } else {
            count += 1;
        }
        v.push(wrap(point, n as i64) as usize);
        prev = !prev;
    }
    v
}

pub fn maurer_rose_points(
    n: f64,
    max_steps: usize,
    angle_degrees: f64,
    rotation: f64,
    max_radius: f64,
    center: Point,
) -> Vec<Point> {
    (0..=max_steps)
        .map(|k| {
            let theta = (k as f64 * angle_degrees).to_radians();
            let r = max_radius * (n * theta).sin();
            let a = theta + rotation;
            Point::new(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect()
}

pub fn cluster_fraction(t: f64, strength: f64) -> f64 {
    let k = 1.0 + strength.clamp(0.0, 1.0) * 4.0;
    if k == 1.0 {
        return t;
    }
    let (a, b) = (t.powf(k), (1.0 - t).powf(k));
    a / (a + b)
}

pub fn comet_layer_sequences(
    n: usize,
    layers: usize,
    first_layer_size: i64,
    layer_distance: i64,
) -> Vec<Vec<usize>> {
    if n == 0 || layers == 0 {
        return Vec::new();
    }
    (0..layers as i64)
        .map(|layer| {
            let d = (first_layer_size - layer * layer_distance)
                .max(1)
                .min(n as i64 - 1)
                .max(0) as usize;
            (0..=n.saturating_sub(d))
                .flat_map(|i| [i % n, (i + d) % n])
                .collect()
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PetalSide {
    Prev,
    Side,
    Next,
    Centre,
}

pub fn flower_petal_weave(nails_per_side: usize) -> Vec<(PetalSide, usize)> {
    if nails_per_side < 3 {
        return Vec::new();
    }
    let mut v = vec![
        (PetalSide::Prev, 0),
        (PetalSide::Side, 0),
        (PetalSide::Next, 0),
    ];
    let mut alt = false;
    for index in 1..nails_per_side - 1 {
        let (ba, ia) = if alt {
            (PetalSide::Prev, index)
        } else {
            (PetalSide::Side, nails_per_side - index - 1)
        };
        v.push((ba, ia));
        v.push((PetalSide::Centre, nails_per_side - index - 1));
        let (bb, ib) = if alt {
            (PetalSide::Side, nails_per_side - index - 1)
        } else {
            (PetalSide::Prev, index)
        };
        v.push((bb, ib));
        alt = !alt;
    }
    v
}

/// `(is_inner, index)`.
pub fn dance_of_planets_walk(
    outer: usize,
    inner: usize,
    rounds: usize,
    reverse: bool,
) -> Vec<(bool, usize)> {
    if outer < 1 || inner < 1 || rounds < 1 {
        return Vec::new();
    }
    let (o, i_) = (outer as i64, inner as i64);
    let steps = o.max(i_) * rounds as i64;
    let inner_index = |step: i64| {
        if !reverse {
            return wrap(step, i_);
        }
        let m = wrap(step, i_);
        wrap(i_ - if m == 0 { i_ } else { m }, i_)
    };
    let mut v = vec![(false, 0)];
    let mut to_inner = true;
    for step in 0..steps {
        let idx = if to_inner {
            inner_index(step)
        } else {
            wrap(step, o)
        };
        v.push((to_inner, idx as usize));
        if step != steps - 1 {
            let next = if to_inner {
                inner_index(step + 1)
            } else {
                wrap(step + 1, o)
            };
            v.push((to_inner, next as usize));
            to_inner = !to_inner;
        }
    }
    v
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HexFlowerCell {
    pub center: Point,
    pub rotation: f64,
}

pub fn hex_flower_grid(
    levels: usize,
    radius: f64,
    rotation: f64,
    center: Point,
) -> Vec<HexFlowerCell> {
    if levels < 1 || radius <= 0.0 {
        return Vec::new();
    }
    let mut cells = Vec::new();
    let lv = levels as f64;
    for wedge in 0..6 {
        let wa = rotation + wedge as f64 * PI / 3.0;
        let u = (wa.cos(), wa.sin());
        let v = ((wa + PI / 3.0).cos(), (wa + PI / 3.0).sin());
        let lattice = |i: f64, j: f64| {
            Point::new(
                center.x + (i * u.0 + j * v.0) * radius / lv,
                center.y + (i * u.1 + j * v.1) * radius / lv,
            )
        };
        let centroid = |a: Point, b: Point, c: Point| {
            Point::new((a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0)
        };
        for row in 0..levels {
            for i in 0..=row {
                let j = (row - i) as f64;
                let i = i as f64;
                let (p1, p2, p3) = (lattice(i, j), lattice(i + 1.0, j), lattice(i, j + 1.0));
                cells.push(HexFlowerCell {
                    center: centroid(p1, p2, p3),
                    rotation: wa,
                });
                if i < row as f64 {
                    let p4 = lattice(i + 1.0, j - 1.0);
                    cells.push(HexFlowerCell {
                        center: centroid(p2, p3, p4),
                        rotation: wa + PI,
                    });
                }
            }
        }
    }
    cells
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossSide {
    Left,
    Right,
    Spine,
}

pub fn crosses_weave(nails_per_line: usize, reverse: bool) -> Vec<(CrossSide, usize)> {
    let n = nails_per_line.max(1);
    let mut v = vec![(CrossSide::Left, 0)];
    let mut side = CrossSide::Right;
    for i in 0..n {
        if i > 0 {
            v.push((side, i));
        }
        v.push((CrossSide::Spine, if reverse { i } else { n - 1 - i }));
        side = if i % 2 == 0 {
            CrossSide::Right
        } else {
            CrossSide::Left
        };
        v.push((side, i));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandala_and_round_robin() {
        let m = mandala_layer_sequences(10, 3, 2);
        assert_eq!(&m[0][..6], &[0, 0, 1, 3, 2, 6]);
        assert_eq!(&m[1][..4], &[5, 5, 6, 8]);
        let r = round_robin_sequence(&[2, 0, 3]);
        assert_eq!(r.len(), 6);
        assert_eq!(r[5], RoundRobinStep { group: 2, local: 2 });
    }

    #[test]
    fn spiral_walk_and_zigzags() {
        // References computed by running the web TS under node.
        assert_eq!(spiral_decaying_walk(20, 1, 3.0), vec![0, 3, 2, 4, 4, 5, 6]);
        assert_eq!(
            two_ray_zigzag(3, 3),
            vec![(true, 2), (false, 0), (true, 1), (false, 1), (true, 0)]
        );
        assert_eq!(
            same_index_zigzag(3, 3),
            vec![(false, 0), (true, 0), (true, 1), (false, 1)]
        );
    }

    #[test]
    fn matches_web_reference_outputs() {
        let f = |v: &[(bool, usize)]| v.iter().map(|(b, i)| (*b, *i)).collect::<Vec<_>>();
        assert_eq!(
            f(&asymmetry_zigzag(7, 5, 0.3, 0.8, false)),
            vec![
                (false, 0),
                (false, 4),
                (false, 5),
                (false, 1),
                (false, 2),
                (false, 6),
                (true, 0),
                (false, 3),
                (false, 4),
                (true, 1),
                (true, 2),
                (false, 5),
                (false, 6),
                (true, 3)
            ]
        );
        assert_eq!(
            f(&asymmetry_zigzag(7, 5, 0.25, 0.75, true)),
            vec![
                (false, 0),
                (true, 2),
                (true, 1),
                (true, 4),
                (true, 3),
                (true, 0),
                (false, 6),
                (true, 2),
                (true, 1),
                (false, 5),
                (false, 4),
                (true, 0),
                (false, 6),
                (false, 3)
            ]
        );
        assert_eq!(
            f(&dance_of_planets_walk(5, 3, 2, true)),
            vec![
                (false, 0),
                (true, 0),
                (true, 2),
                (false, 1),
                (false, 2),
                (true, 1),
                (true, 0),
                (false, 3),
                (false, 4),
                (true, 2),
                (true, 1),
                (false, 0),
                (false, 1),
                (true, 0),
                (true, 2),
                (false, 2),
                (false, 3),
                (true, 1),
                (true, 0),
                (false, 4)
            ]
        );
        assert_eq!(
            wave_layer_sequences(12, 5, 2, 7, 3),
            vec![
                vec![1, 5, 10, 2, 3, 3, 8, 4, 5, 1, 6, 6],
                vec![4, 8, 1, 5, 6, 6, 11, 7, 8, 4, 9, 9]
            ]
        );
        assert_eq!(
            comet_layer_sequences(8, 2, 4, 1),
            vec![
                vec![0, 4, 1, 5, 2, 6, 3, 7, 4, 0],
                vec![0, 3, 1, 4, 2, 5, 3, 6, 4, 7, 5, 0]
            ]
        );
        use CrossSide::*;
        assert_eq!(
            crosses_weave(4, false),
            vec![
                (Left, 0),
                (Spine, 3),
                (Right, 0),
                (Right, 1),
                (Spine, 2),
                (Left, 1),
                (Left, 2),
                (Spine, 1),
                (Right, 2),
                (Right, 3),
                (Spine, 0),
                (Left, 3)
            ]
        );
        use PetalSide::*;
        assert_eq!(
            flower_petal_weave(5),
            vec![
                (Prev, 0),
                (Side, 0),
                (Next, 0),
                (Side, 3),
                (Centre, 3),
                (Prev, 1),
                (Prev, 2),
                (Centre, 2),
                (Side, 2),
                (Side, 1),
                (Centre, 1),
                (Prev, 3)
            ]
        );
    }

    #[test]
    fn hex_grid_cell_count() {
        // levels² triangles per wedge × 6
        assert_eq!(hex_flower_grid(3, 10.0, 0.0, Point::ORIGIN).len(), 54);
    }

    #[test]
    fn cluster_fraction_identity_at_zero_strength() {
        assert_eq!(cluster_fraction(0.3, 0.0), 0.3);
        assert!((cluster_fraction(0.5, 1.0) - 0.5).abs() < 1e-12);
    }
}
