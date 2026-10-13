//! Lotus generator pattern (spec 32). Port of web `domain/generator/lotus.ts`.

use std::f64::consts::PI;

use crate::Point;
use crate::generator::js_round;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LotusFit {
    /// Nails per petal circle.
    pub n: usize,
    /// Nails per section.
    pub p: usize,
    pub radius: f64,
    /// Half-angle excluded from each petal arc; 0 = full circle.
    pub alpha: f64,
}

pub fn sections_count(sides: usize) -> usize {
    sides.div_ceil(2)
}

pub fn max_removable_sections(sides: usize) -> usize {
    sections_count(sides).saturating_sub(3)
}

pub fn removed_sections_count(sides: usize, remove: f64) -> usize {
    let rmax = max_removable_sections(sides);
    if rmax == 0 {
        return 0;
    }
    (js_round(remove * rmax as f64).max(0.0) as usize).min(rmax)
}

pub fn fit(sides: usize, density: f64, max_petal_radius: f64, removed: usize) -> LotusFit {
    let s = sides as f64;
    let n0 = density * s;
    if removed == 0 {
        return LotusFit {
            n: n0 as usize,
            p: (n0 / s).floor() as usize,
            radius: max_petal_radius,
            alpha: 0.0,
        };
    }
    let alpha = removed as f64 * (2.0 * PI / s);
    let top = 2.0 * max_petal_radius * ((PI - alpha) / 2.0).sin();
    let f = 2.0 * max_petal_radius / top;
    let scaled = n0 * f;
    let full = s.max(scaled - scaled % s);
    let n = js_round(full * (1.0 - 2.0 * removed as f64 / s) + 1.0);
    let p = (n / (s - 2.0 * removed as f64)).floor();
    LotusFit {
        n: n as usize,
        p: p as usize,
        radius: max_petal_radius * f,
        alpha,
    }
}

pub fn petal_center(
    sides: usize,
    rotation: f64,
    helper_radius: f64,
    origin: Point,
    j: usize,
) -> Point {
    let a = 2.0 * PI * j as f64 / sides as f64 - rotation;
    Point::new(
        origin.x + helper_radius * a.sin(),
        origin.y + helper_radius * a.cos(),
    )
}

pub fn petal_point(
    f: &LotusFit,
    sides: usize,
    rotation: f64,
    petal_center: Point,
    j: usize,
    k: usize,
) -> Point {
    let base = if f.alpha > 0.0 {
        f.alpha + (2.0 * PI - 2.0 * f.alpha) * (k as f64 / (f.n as f64 - 1.0))
    } else {
        2.0 * PI * k as f64 / f.n as f64
    };
    let a = base - rotation + 2.0 * PI * j as f64 / sides as f64;
    Point::new(
        petal_center.x + f.radius * a.sin(),
        petal_center.y + f.radius * a.cos(),
    )
}

pub fn center_circle_point(
    sides: usize,
    rotation: f64,
    center_radius: f64,
    origin: Point,
    k: usize,
) -> Point {
    let cr = rotation - PI * ((sides as f64 - 4.0) / 2.0).ceil() / sides as f64;
    let a = 2.0 * PI * k as f64 / sides as f64 - cr;
    Point::new(
        origin.x + center_radius * a.sin(),
        origin.y + center_radius * a.cos(),
    )
}

pub fn max_center_radius(
    f: &LotusFit,
    sides: usize,
    rotation: f64,
    origin: Point,
    sections: usize,
    removed: usize,
) -> f64 {
    let c0 = petal_center(sides, rotation, f.radius, origin, 0);
    let k = (sections as i64 - 1 - removed as i64).max(0) as usize * f.p;
    petal_point(f, sides, rotation, c0, 0, k).distance(origin)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LotusNode {
    Petal { circle: usize, index: usize },
    Center(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LotusPatch {
    pub circle: usize,
    pub section: usize,
}

pub fn generate_patches(
    sides: usize,
    removed: usize,
    sections: usize,
    render_center: bool,
    radial_color: bool,
) -> Vec<LotusPatch> {
    let last = sections.saturating_sub(if render_center { 1 } else { 2 });
    let mut v = Vec::new();
    if radial_color {
        for section in removed..last {
            for circle in 0..sides {
                v.push(LotusPatch { circle, section });
            }
        }
    } else {
        for circle in 0..sides {
            for section in removed..last {
                v.push(LotusPatch { circle, section });
            }
        }
    }
    v
}

pub fn patch_colour_index(p: &LotusPatch, removed: usize, radial_color: bool) -> usize {
    if radial_color {
        p.section - removed
    } else {
        p.circle
    }
}

pub fn colour_group_count(
    sides: usize,
    sections: usize,
    removed: usize,
    render_center: bool,
    radial_color: bool,
) -> usize {
    if !radial_color {
        return sides;
    }
    let last = sections.saturating_sub(if render_center { 1 } else { 2 });
    last.saturating_sub(removed).max(1)
}

pub struct PatchPlan {
    pub target: LotusNode,
    pub fan1: Vec<LotusNode>,
    pub fan2: Vec<LotusNode>,
}

pub fn draw_patch(
    sides: usize,
    p: usize,
    n: usize,
    removed: usize,
    sections: usize,
    has_center: bool,
    patch: LotusPatch,
) -> PatchPlan {
    let LotusPatch { circle, section } = patch;
    let prev = if circle == 0 { sides - 1 } else { circle - 1 };
    let petal = |c: usize, index: usize| LotusNode::Petal { circle: c, index };
    if section == 0 {
        return PatchPlan {
            target: petal(prev, p * 2),
            fan1: (n.saturating_sub(p)..n).map(|i| petal(circle, i)).collect(),
            fan2: (0..=p).map(|i| petal(circle, i)).collect(),
        };
    }
    let last = section + 2 == sections;
    let target = if last {
        LotusNode::Center(if has_center { circle } else { 0 })
    } else {
        petal(prev, p * (section + 2 - removed))
    };
    let first_circle = (circle + section) % sides;
    let start1 = n as i64 - ((section + 1 - removed) * p) as i64 - if removed > 0 { 1 } else { 0 };
    let fan1 = (0..=p)
        .map(|i| petal(first_circle, (start1 + i as i64).max(0) as usize))
        .collect();
    let start2 = (section - removed) * p + 1;
    let fan2 = (start2..start2 + p).map(|i| petal(circle, i)).collect();
    PatchPlan { target, fan1, fan2 }
}

pub fn step_count(sides: usize, p: usize, sections: usize, removed: usize) -> usize {
    sides * (2 * p + 1) * (sections.saturating_sub(removed + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_values() {
        let f = fit(18, 15.0, 10.0, 0);
        assert_eq!((f.n, f.p), (270, 15));
        assert_eq!(removed_sections_count(18, 0.5), 3);
        let f = fit(18, 15.0, 10.0, 3);
        assert!(f.alpha > 0.0 && f.n > 0 && f.p > 0);
        assert_eq!(generate_patches(6, 0, 3, true, false).len(), 12);
    }
}
