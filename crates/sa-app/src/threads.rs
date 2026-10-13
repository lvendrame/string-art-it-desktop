//! Thread sequence algorithms (specs 22, 35, 37, 38). Port of web
//! `threadPattern.ts`, `twoPinSequence.ts`, `radialSequence.ts`,
//! `repeatSequence.ts`.

use std::collections::HashSet;

use crate::document::symmetry::mirrored_pin_id;
use crate::document::{GeometryKind, PinLayer, PinPath};

/// A pin's 1-based-able position within its own Pin Path; `group_index` is the
/// mirror copy (`None` for a real, stored pin).
#[derive(Clone, Copy, Debug)]
pub struct PinPosition<'a> {
    pub path: &'a PinPath,
    pub index: usize,
    pub group_index: Option<usize>,
}

pub fn find_pin_position<'a>(layers: &'a [PinLayer], pin_id: &str) -> Option<PinPosition<'a>> {
    for l in layers {
        for path in &l.pin_paths {
            if let Some(index) = path.pins.iter().position(|p| p.id == pin_id) {
                return Some(PinPosition {
                    path,
                    index,
                    group_index: None,
                });
            }
        }
    }
    // Mirror ids look like `<source>~mirror-<g>`.
    let (source, g) = pin_id.rsplit_once("~mirror-")?;
    let g: usize = g.parse().ok()?;
    for l in layers {
        for path in &l.pin_paths {
            if let Some(index) = path.pins.iter().position(|p| p.id == source)
                && g < path.mirrored_pin_groups().len()
            {
                return Some(PinPosition {
                    path,
                    index,
                    group_index: Some(g),
                });
            }
        }
    }
    None
}

pub fn pin_id_at(path: &PinPath, index: usize, group: Option<usize>) -> String {
    let id = &path.pins[index].id;
    match group {
        None => id.clone(),
        Some(g) => mirrored_pin_id(id, g),
    }
}

fn wrap(n: i64, count: i64) -> i64 {
    (((n - 1) % count) + count) % count + 1
}

/// Spec 22 pattern-follow (Right Arrow): extrapolates the next vertex from the
/// last two members of the parity group receiving it.
pub fn compute_next_pattern_pin_id(layers: &[PinLayer], pin_ids: &[String]) -> Option<String> {
    if pin_ids.len() < 4 {
        return None;
    }
    let parity = pin_ids.len() % 2;
    let group: Vec<&String> = pin_ids
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == parity)
        .map(|(_, id)| id)
        .collect();
    let [.., second_last, last] = group.as_slice() else {
        return None;
    };
    let a = find_pin_position(layers, second_last)?;
    let b = find_pin_position(layers, last)?;
    let (n_a, n_b) = (a.index as i64 + 1, b.index as i64 + 1);
    let count = b.path.pins.len() as i64;
    if count == 0 {
        return None;
    }
    let next = wrap(n_b + (n_b - n_a), count);
    Some(pin_id_at(b.path, (next - 1) as usize, b.group_index))
}

// ---- Zig-zag / Parabolic (spec 35)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwoPinTool {
    Zigzag,
    Parabolic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoPinFillSettings {
    pub step_a: f64,
    pub step_b: f64,
    pub full_fill: bool,
    pub cycles: Option<f64>,
}

impl Default for TwoPinFillSettings {
    fn default() -> Self {
        Self {
            step_a: 0.0,
            step_b: 0.0,
            full_fill: false,
            cycles: None,
        }
    }
}

fn reverse_second_for(tool: TwoPinTool, same: bool) -> bool {
    if same {
        tool == TwoPinTool::Zigzag
    } else {
        tool == TwoPinTool::Parabolic
    }
}

fn stride<T: Clone>(list: &[T], step: f64) -> Vec<T> {
    let advance = step.floor().max(0.0) as usize + 1;
    list.iter().step_by(advance).cloned().collect()
}

pub fn is_path_closed(path: &PinPath) -> bool {
    path.geometry.kind() == GeometryKind::Text
        || path
            .geometry
            .contour_paths()
            .first()
            .is_some_and(|p| p.closed)
}

pub fn remaining_in_direction(count: usize, index: usize, dir: i64, closed: bool) -> usize {
    if closed {
        count
    } else if dir == 1 {
        count - index
    } else {
        index + 1
    }
}

fn extract_ids(
    path: &PinPath,
    index: usize,
    dir: i64,
    count: usize,
    closed: bool,
    group: Option<usize>,
) -> Vec<String> {
    let n = path.pins.len() as i64;
    (0..count as i64)
        .map(|k| {
            let mut idx = index as i64 + k * dir;
            if closed {
                idx = ((idx % n) + n) % n;
            }
            pin_id_at(path, idx as usize, group)
        })
        .collect()
}

pub fn interleave<T: Clone>(a: &[T], b: &[T]) -> Vec<T> {
    a.iter()
        .zip(b)
        .flat_map(|(x, y)| [x.clone(), y.clone()])
        .collect()
}

fn hops_to_pass_own_origin(n: usize, stride: usize, cycles: usize) -> usize {
    let target = cycles * n;
    let boundary = target.div_ceil(stride);
    if target.is_multiple_of(stride) {
        boundary
    } else {
        boundary + 1
    }
}

fn ring_side(
    path: &PinPath,
    start: usize,
    stride: usize,
    hops: usize,
    group: Option<usize>,
) -> Vec<String> {
    let n = path.pins.len();
    let mut cur = start;
    (0..hops)
        .map(|_| {
            let id = pin_id_at(path, cur, group);
            cur = (cur + stride) % n;
            id
        })
        .collect()
}

fn ring_walk_sequence(
    path: &PinPath,
    a: usize,
    b: usize,
    group: Option<usize>,
    s: &TwoPinFillSettings,
) -> Vec<String> {
    let sa = s.step_a.floor().max(0.0) as usize + 1;
    let sb = s.step_b.floor().max(0.0) as usize + 1;
    let n = path.pins.len();
    let cycles = s.cycles.unwrap_or(1.0).floor().max(1.0) as usize;
    let hops = hops_to_pass_own_origin(n, sa, cycles).min(hops_to_pass_own_origin(n, sb, cycles));
    let side_a = ring_side(path, a, sa, hops + 1, group);
    let side_b = ring_side(path, b, sb, hops, group);
    let mut out = interleave(&side_a, &side_b);
    out.push(side_a[hops].clone());
    out
}

pub fn build_same_range_sequence(
    range: &[String],
    reverse_second: bool,
    s: &TwoPinFillSettings,
) -> Vec<String> {
    let n = range.len();
    let half = n / 2;
    let first = &range[..half];
    let mut second: Vec<String> = range[n - half..].to_vec();
    if reverse_second {
        second.reverse();
    }
    let mut out = interleave(&stride(first, s.step_a), &stride(&second, s.step_b));
    if n % 2 == 1 {
        out.push(range[half].clone());
    }
    out
}

pub fn build_cross_path_sequence(
    run_a: &[String],
    run_b: &[String],
    reverse_second: bool,
    s: &TwoPinFillSettings,
) -> Vec<String> {
    let len = run_a.len().min(run_b.len());
    let mut b: Vec<String> = run_b[..len].to_vec();
    if reverse_second {
        b.reverse();
    }
    interleave(&stride(&run_a[..len], s.step_a), &stride(&b, s.step_b))
}

#[derive(Clone, Debug, PartialEq)]
pub struct TwoPinCandidate {
    pub dir_a: i64,
    pub dir_b: Option<i64>,
    pub sequence: Vec<String>,
    /// Separate strands committed alongside `sequence` (closed full-fill zig-zag).
    pub extra_sequences: Vec<Vec<String>>,
}

pub fn compute_same_path_candidates(
    layers: &[PinLayer],
    first: &str,
    second: &str,
    tool: TwoPinTool,
    s: &TwoPinFillSettings,
) -> Vec<TwoPinCandidate> {
    let (Some(a), Some(b)) = (
        find_pin_position(layers, first),
        find_pin_position(layers, second),
    ) else {
        return Vec::new();
    };
    if a.path.id != b.path.id || a.index == b.index {
        return Vec::new();
    }
    let path = a.path;
    let closed = is_path_closed(path);
    let count = path.pins.len();
    let reverse = reverse_second_for(tool, true);
    let arc = |dir: i64, n: usize| {
        build_same_range_sequence(
            &extract_ids(path, a.index, dir, n, closed, a.group_index),
            reverse,
            s,
        )
    };
    let cand = |dir: i64, sequence: Vec<String>| TwoPinCandidate {
        dir_a: dir,
        dir_b: None,
        sequence,
        extra_sequences: Vec::new(),
    };

    if !closed {
        let dir = if b.index > a.index { 1 } else { -1 };
        return vec![cand(dir, arc(dir, a.index.abs_diff(b.index) + 1))];
    }
    let forward = ((b.index as i64 - a.index as i64).rem_euclid(count as i64)) as usize + 1;
    let backward = count + 2 - forward;
    if s.full_fill {
        if tool == TwoPinTool::Parabolic {
            return vec![cand(
                1,
                ring_walk_sequence(path, a.index, b.index, a.group_index, s),
            )];
        }
        let cycles = s.cycles.unwrap_or(1.0).floor().max(1.0) as usize;
        let mut strands: Vec<Vec<String>> = Vec::new();
        for _ in 0..cycles {
            strands.push(arc(1, forward));
            if backward != forward {
                strands.push(arc(-1, backward));
            }
        }
        let sequence = strands.remove(0);
        return vec![TwoPinCandidate {
            dir_a: 1,
            dir_b: None,
            sequence,
            extra_sequences: strands,
        }];
    }
    let mut out = vec![cand(1, arc(1, forward))];
    if backward != forward {
        out.push(cand(-1, arc(-1, backward)));
    }
    out
}

fn anchor_runs(
    path: &PinPath,
    index: usize,
    group: Option<usize>,
    closed: bool,
    full_fill: bool,
) -> Vec<(i64, Vec<String>)> {
    let n = path.pins.len();
    if full_fill && !closed {
        let mut pos = extract_ids(
            path,
            index,
            1,
            remaining_in_direction(n, index, 1, closed),
            closed,
            group,
        );
        let neg = extract_ids(
            path,
            index,
            -1,
            remaining_in_direction(n, index, -1, closed),
            closed,
            group,
        );
        pos.extend(neg.into_iter().skip(1));
        return vec![(1, pos)];
    }
    [1i64, -1]
        .into_iter()
        .map(|dir| {
            (
                dir,
                extract_ids(
                    path,
                    index,
                    dir,
                    remaining_in_direction(n, index, dir, closed),
                    closed,
                    group,
                ),
            )
        })
        .filter(|(_, r)| !r.is_empty())
        .collect()
}

pub fn compute_cross_path_candidates(
    layers: &[PinLayer],
    first: &str,
    second: &str,
    tool: TwoPinTool,
    s: &TwoPinFillSettings,
) -> Vec<TwoPinCandidate> {
    let (Some(a), Some(b)) = (
        find_pin_position(layers, first),
        find_pin_position(layers, second),
    ) else {
        return Vec::new();
    };
    if a.path.id == b.path.id {
        return Vec::new();
    }
    let reverse = reverse_second_for(tool, false);
    let runs_a = anchor_runs(
        a.path,
        a.index,
        a.group_index,
        is_path_closed(a.path),
        s.full_fill,
    );
    let runs_b = anchor_runs(
        b.path,
        b.index,
        b.group_index,
        is_path_closed(b.path),
        s.full_fill,
    );
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (da, ra) in &runs_a {
        for (db, rb) in &runs_b {
            let sequence = build_cross_path_sequence(ra, rb, reverse, s);
            if seen.insert(sequence.join(",")) {
                out.push(TwoPinCandidate {
                    dir_a: *da,
                    dir_b: Some(*db),
                    sequence,
                    extra_sequences: Vec::new(),
                });
            }
        }
    }
    out
}

// ---- Radial (spec 37)

pub fn compute_radial_sequence(
    layers: &[PinLayer],
    anchor: &str,
    target: &str,
    step: f64,
) -> Vec<String> {
    if anchor == target || find_pin_position(layers, anchor).is_none() {
        return Vec::new();
    }
    let Some(t) = find_pin_position(layers, target) else {
        return Vec::new();
    };
    let stride_n = step.floor().max(1.0) as usize;
    (0..t.path.pins.len())
        .step_by(stride_n)
        .map(|i| pin_id_at(t.path, i, t.group_index))
        .filter(|id| id != anchor)
        .flat_map(|id| [anchor.to_owned(), id])
        .collect()
}

// ---- Repeat (spec 38)

pub const MIN_REPEAT_PINS: usize = 4;

pub fn max_repeat_colours(cycles: usize) -> usize {
    cycles + 2
}

/// Parabolic stop rule for full-fill cycles.
pub fn full_fill_cycle_count(step: i64, pin_count: usize) -> usize {
    let d = step.unsigned_abs() as usize;
    if d == 0 {
        return 0;
    }
    let laps = pin_count.div_ceil(d);
    if laps * d == pin_count {
        laps.saturating_sub(2)
    } else {
        laps.saturating_sub(1)
    }
}

pub fn compute_repeat_groups(
    layers: &[PinLayer],
    pin_ids: &[String],
    cycles: usize,
    full_fill: bool,
) -> Option<Vec<Vec<String>>> {
    let usable = &pin_ids[..pin_ids.len() - pin_ids.len() % 2];
    if usable.len() < MIN_REPEAT_PINS {
        return None;
    }
    let half = usable.len() / 2;
    let (first, second) = usable.split_at(half);
    let mut pairs = Vec::new();
    for (fa, sb) in first.iter().zip(second) {
        let a = find_pin_position(layers, fa)?;
        let b = find_pin_position(layers, sb)?;
        pairs.push((b, b.index as i64 - a.index as i64));
    }
    let (lead, lead_step) = pairs[0];
    let cycle_count = if full_fill {
        full_fill_cycle_count(lead_step, lead.path.pins.len())
    } else {
        cycles
    };
    let mut groups = vec![first.to_vec(), second.to_vec()];
    for k in 1..=cycle_count as i64 {
        groups.push(
            pairs
                .iter()
                .map(|(pos, step)| {
                    let count = pos.path.pins.len() as i64;
                    let index = ((pos.index as i64 + k * step) % count + count) % count;
                    pin_id_at(pos.path, index as usize, pos.group_index)
                })
                .collect(),
        );
    }
    Some(groups)
}

/// One colour → one continuous path; otherwise one path per group, each later
/// group starting at the previous group's last pin.
pub fn build_repeat_thread_pins(groups: &[Vec<String>], colour_count: usize) -> Vec<Vec<String>> {
    if colour_count <= 1 {
        return vec![groups.concat()];
    }
    groups
        .iter()
        .enumerate()
        .map(|(g, group)| {
            if g == 0 {
                group.clone()
            } else {
                let mut v = vec![groups[g - 1].last().cloned().unwrap_or_default()];
                v.extend(group.iter().cloned());
                v
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Pin, PinPathGeometry, SymmetryConfig};
    use sa_domain::Point;

    fn path(id: &str, prefix: &str, count: usize, closed: bool) -> PinPath {
        PinPath {
            id: id.into(),
            geometry: if closed {
                PinPathGeometry::Circle {
                    center: Point::ORIGIN,
                    radius: count as f64,
                }
            } else {
                PinPathGeometry::Line {
                    start: Point::ORIGIN,
                    end: Point::new(count as f64, 0.0),
                }
            },
            requested_spacing: 1.0,
            actual_spacing: 1.0,
            pins: (0..count)
                .map(|i| Pin {
                    id: format!("{prefix}{}", i + 1),
                    x: i as f64,
                    y: 0.0,
                })
                .collect(),
            guide_visible: true,
            colour: "#fff".into(),
            diameter: 2.0,
            symmetry: SymmetryConfig::None,
        }
    }

    fn layer(paths: Vec<PinPath>) -> Vec<PinLayer> {
        let mut l = PinLayer::new("L");
        l.pin_paths = paths;
        vec![l]
    }

    fn s(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|x| (*x).to_owned()).collect()
    }

    const D: TwoPinFillSettings = TwoPinFillSettings {
        step_a: 0.0,
        step_b: 0.0,
        full_fill: false,
        cycles: None,
    };

    #[test]
    fn same_path_open() {
        let ls = layer(vec![path("a", "p", 10, false)]);
        let c = compute_same_path_candidates(&ls, "p1", "p10", TwoPinTool::Zigzag, &D);
        assert_eq!(c.len(), 1);
        assert_eq!(
            c[0].sequence,
            s(&["p1", "p10", "p2", "p9", "p3", "p8", "p4", "p7", "p5", "p6"])
        );
        let c = compute_same_path_candidates(&ls, "p1", "p10", TwoPinTool::Parabolic, &D);
        assert_eq!(
            c[0].sequence,
            s(&["p1", "p6", "p2", "p7", "p3", "p8", "p4", "p9", "p5", "p10"])
        );
        let c = compute_same_path_candidates(&ls, "p10", "p1", TwoPinTool::Zigzag, &D);
        assert_eq!(
            c[0].sequence,
            s(&["p10", "p1", "p9", "p2", "p8", "p3", "p7", "p4", "p6", "p5"])
        );
        assert_eq!(
            compute_same_path_candidates(&ls, "p4", "p5", TwoPinTool::Parabolic, &D)[0].sequence,
            s(&["p4", "p5"])
        );
        assert!(compute_same_path_candidates(&ls, "p3", "p3", TwoPinTool::Zigzag, &D).is_empty());
        let ls = layer(vec![path("a", "p", 11, false)]);
        let c = compute_same_path_candidates(&ls, "p1", "p11", TwoPinTool::Zigzag, &D);
        assert_eq!(
            c[0].sequence,
            s(&[
                "p1", "p11", "p2", "p10", "p3", "p9", "p4", "p8", "p5", "p7", "p6"
            ])
        );
        let c = compute_same_path_candidates(&ls, "p1", "p11", TwoPinTool::Parabolic, &D);
        assert_eq!(
            c[0].sequence,
            s(&[
                "p1", "p7", "p2", "p8", "p3", "p9", "p4", "p10", "p5", "p11", "p6"
            ])
        );
    }

    #[test]
    fn same_path_closed_two_arcs() {
        let ls = layer(vec![path("a", "p", 8, true)]);
        let c = compute_same_path_candidates(&ls, "p1", "p4", TwoPinTool::Zigzag, &D);
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|x| x.sequence[0] == "p1"));
        let mut lens: Vec<usize> = c.iter().map(|x| x.sequence.len()).collect();
        lens.sort();
        assert_eq!(lens, vec![4, 6]);
    }

    #[test]
    fn cross_path() {
        let ls = layer(vec![path("a", "a", 10, false), path("b", "b", 11, false)]);
        let c = compute_cross_path_candidates(&ls, "a1", "b1", TwoPinTool::Zigzag, &D);
        let best = c.iter().find(|c| c.sequence.len() == 20).unwrap();
        assert_eq!(best.sequence[..6], s(&["a1", "b1", "a2", "b2", "a3", "b3"]));
        let c = compute_cross_path_candidates(&ls, "a1", "b1", TwoPinTool::Parabolic, &D);
        let best = c.iter().find(|c| c.sequence.len() == 20).unwrap();
        assert_eq!(best.sequence[..4], s(&["a1", "b10", "a2", "b9"]));
        let ls2 = layer(vec![path("a", "a", 11, false), path("b", "b", 11, false)]);
        let c = compute_cross_path_candidates(&ls2, "a6", "b6", TwoPinTool::Zigzag, &D);
        assert_eq!(c.len(), 4);
        assert!(c.iter().all(|x| x.sequence.len() == 12));
        let ls3 = layer(vec![path("a", "a", 10, false), path("b", "b", 20, false)]);
        let st = TwoPinFillSettings {
            step_a: 2.0,
            step_b: 2.0,
            ..D
        };
        let c = compute_cross_path_candidates(&ls3, "a1", "b1", TwoPinTool::Zigzag, &st);
        let best = c
            .iter()
            .find(|c| c.dir_a == 1 && c.dir_b == Some(1))
            .unwrap();
        assert_eq!(
            best.sequence,
            s(&["a1", "b1", "a4", "b4", "a7", "b7", "a10", "b10"])
        );
    }

    #[test]
    fn pattern_follow() {
        let ls = layer(vec![path("a", "p", 20, true)]);
        let ids = s(&["p3", "p1", "p6", "p2"]);
        // next (5th) is group of positions 1,3: p3→p6 step 3 → p9
        assert_eq!(
            compute_next_pattern_pin_id(&ls, &ids).as_deref(),
            Some("p9")
        );
        let ids = s(&["p3", "p1", "p6", "p2", "p9"]);
        assert_eq!(
            compute_next_pattern_pin_id(&ls, &ids).as_deref(),
            Some("p3")
        );
        assert!(compute_next_pattern_pin_id(&ls, &ids[..3]).is_none());
        let ids = s(&["p18", "p1", "p19", "p2"]);
        assert_eq!(
            compute_next_pattern_pin_id(&ls, &ids).as_deref(),
            Some("p20")
        );
    }

    #[test]
    fn radial() {
        let ls = layer(vec![path("a", "a", 3, false), path("b", "b", 5, true)]);
        assert_eq!(
            compute_radial_sequence(&ls, "a1", "b3", 1.0),
            s(&["a1", "b1", "a1", "b2", "a1", "b3", "a1", "b4", "a1", "b5"])
        );
        assert_eq!(
            compute_radial_sequence(&ls, "a1", "b3", 2.0),
            s(&["a1", "b1", "a1", "b3", "a1", "b5"])
        );
        assert!(compute_radial_sequence(&ls, "a1", "a1", 1.0).is_empty());
    }

    #[test]
    fn repeat() {
        let ls = layer(vec![path("a", "p", 12, true)]);
        let g = compute_repeat_groups(&ls, &s(&["p1", "p2", "p4", "p5"]), 2, false).unwrap();
        assert_eq!(
            g,
            vec![
                s(&["p1", "p2"]),
                s(&["p4", "p5"]),
                s(&["p7", "p8"]),
                s(&["p10", "p11"])
            ]
        );
        assert!(compute_repeat_groups(&ls, &s(&["p1", "p2", "p3"]), 1, false).is_none());
        assert_eq!(full_fill_cycle_count(3, 12), 2);
        assert_eq!(full_fill_cycle_count(5, 12), 2);
        assert_eq!(full_fill_cycle_count(0, 12), 0);
        let paths = build_repeat_thread_pins(&g[..2], 2);
        assert_eq!(paths, vec![s(&["p1", "p2"]), s(&["p2", "p4", "p5"])]);
        assert_eq!(
            build_repeat_thread_pins(&g[..2], 1),
            vec![s(&["p1", "p2", "p4", "p5"])]
        );
    }

    #[test]
    fn mirror_positions() {
        let mut p = path("a", "p", 4, false);
        p.symmetry = SymmetryConfig::Vertical {
            axis: Point::new(-10.0, 0.0),
        };
        let ls = layer(vec![p]);
        let pos = find_pin_position(&ls, "p3~mirror-0").unwrap();
        assert_eq!((pos.index, pos.group_index), (2, Some(0)));
        assert!(find_pin_position(&ls, "p3~mirror-5").is_none());
    }
}
