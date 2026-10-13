//! Procedural wood grain — bit-for-bit port of web `woodGrain.ts` (seeded
//! mulberry32 + FNV-1a + Perlin fBm), so a preset renders identically to the web
//! app. Geometry is in objectBoundingBox units (0..1 across the board's bbox).

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{Arc, Mutex, OnceLock};

use sa_domain::Point;

use crate::color::{Rgba, mix};

const FOCUS_X: f64 = 0.5;
const FOCUS_Y: f64 = 0.5;
const HUMP_COUNT: usize = 13;
const BANDS_PER_HUMP: usize = 4;
const TURB_POWER_FRACTION: f64 = 2.2;
const TURB_SIZE: f64 = 0.35;
const RING_ANGLE_STEPS: usize = 72;

/// One filled band: `outer` minus `inner` (even-odd); the innermost has no inner.
#[derive(Clone, Debug, PartialEq)]
pub struct WoodBand {
    pub outer: Vec<Point>,
    pub inner: Option<Vec<Point>>,
    pub colour: Rgba,
}

/// mulberry32 seeded PRNG producing floats in [0, 1).
pub fn mulberry32(seed: u32) -> impl FnMut() -> f64 {
    let mut a = seed;
    move || {
        a = a.wrapping_add(0x6d2b_79f5);
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        (t ^ (t >> 14)) as f64 / 4_294_967_296.0
    }
}

/// FNV-1a over UTF-16 code units (JS `charCodeAt`).
pub fn hash_seed(s: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for c in s.encode_utf16() {
        h ^= c as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

const GRAD: [(f64, f64); 8] = [
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (1.0, 1.0),
    (-1.0, 1.0),
    (1.0, -1.0),
    (-1.0, -1.0),
];

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

struct Noise {
    perm: [u8; 512],
}

impl Noise {
    fn new(seed: u32) -> Self {
        let mut rand = mulberry32(seed);
        let mut base: [u8; 256] = std::array::from_fn(|i| i as u8);
        for i in (1..256).rev() {
            let j = (rand() * (i + 1) as f64).floor() as usize;
            base.swap(i, j);
        }
        Self {
            perm: std::array::from_fn(|i| base[i & 255]),
        }
    }

    fn noise(&self, x: f64, y: f64) -> f64 {
        let xi = ((x.floor() as i64) & 255) as usize;
        let yi = ((y.floor() as i64) & 255) as usize;
        let xf = x - x.floor();
        let yf = y - y.floor();
        let (u, v) = (fade(xf), fade(yf));
        let grad = |hash: u8, gx: f64, gy: f64| {
            let g = GRAD[(hash & 7) as usize];
            g.0 * gx + g.1 * gy
        };
        let p = &self.perm;
        let aa = p[p[xi] as usize + yi];
        let ab = p[p[xi] as usize + yi + 1];
        let ba = p[p[xi + 1] as usize + yi];
        let bb = p[p[xi + 1] as usize + yi + 1];
        let x1 = lerp(grad(aa, xf, yf), grad(ba, xf - 1.0, yf), u);
        let x2 = lerp(grad(ab, xf, yf - 1.0), grad(bb, xf - 1.0, yf - 1.0), u);
        lerp(x1, x2, v)
    }

    fn turbulence(&self, x: f64, y: f64, size: f64) -> f64 {
        let mut value = 0.0;
        let mut s = size;
        while s >= 1.0 / 32.0 {
            value += self.noise(x / s, y / s) * s;
            s /= 2.0;
        }
        value / (2.0 * size)
    }
}

/// JS `toFixed(4)` as applied to the web path strings.
fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

fn trace_contour(target_r: f64, turb_power: f64, noise: &Noise) -> Vec<Point> {
    (0..RING_ANGLE_STEPS)
        .map(|i| {
            let theta = i as f64 / RING_ANGLE_STEPS as f64 * PI * 2.0;
            let (sin, cos) = theta.sin_cos();
            let mut r = target_r;
            for _ in 0..3 {
                let x = FOCUS_X + r * cos;
                let y = FOCUS_Y + r * sin;
                r = target_r - turb_power * noise.turbulence(x, y, TURB_SIZE);
            }
            Point::new(round4(FOCUS_X + r * cos), round4(FOCUS_Y + r * sin))
        })
        .collect()
}

fn build(preset_id: &str, colours: [Rgba; 3]) -> Vec<WoodBand> {
    let noise = Noise::new(hash_seed(preset_id));
    let [light, _, dark] = colours;
    let max_r = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
        .iter()
        .map(|(x, y): &(f64, f64)| (x - FOCUS_X).hypot(y - FOCUS_Y))
        .fold(0.0, f64::max);
    let hump_width = max_r / HUMP_COUNT as f64;
    let turb_power = hump_width * TURB_POWER_FRACTION;
    let sub_width = hump_width / BANDS_PER_HUMP as f64;
    let total = HUMP_COUNT * BANDS_PER_HUMP;
    let contours: Vec<Vec<Point>> = (0..=total)
        .map(|k| trace_contour(k as f64 * sub_width, turb_power, &noise))
        .collect();
    (0..total)
        .map(|idx| {
            let phase = ((idx % BANDS_PER_HUMP) as f64 + 0.5) / BANDS_PER_HUMP as f64;
            let brightness = (phase * PI).sin().abs();
            WoodBand {
                outer: contours[idx + 1].clone(),
                inner: (idx > 0).then(|| contours[idx].clone()),
                colour: mix(dark, light, brightness),
            }
        })
        .collect()
}

type Cache = Mutex<HashMap<(String, [Rgba; 3]), Arc<Vec<WoodBand>>>>;

/// Deterministic, cached wood-grain bands for a preset.
pub fn wood_grain(preset_id: &str, colours: [Rgba; 3]) -> Arc<Vec<WoodBand>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (preset_id.to_owned(), colours);
    if let Some(hit) = cache.lock().expect("wood cache").get(&key) {
        return hit.clone();
    }
    let bands = Arc::new(build(preset_id, colours));
    cache.lock().expect("wood cache").insert(key, bands.clone());
    bands
}

const fn hex(v: u32) -> Rgba {
    Rgba::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// (id, [light, mid, dark]) — web `WOOD_PRESETS`.
pub const WOOD_PRESETS: [(&str, [Rgba; 3]); 8] = [
    ("walnut", [hex(0x8a5a3c), hex(0x6b3f28), hex(0x4a2a1a)]),
    ("oak", [hex(0xc9a06a), hex(0xa97c46), hex(0x7a5730)]),
    ("ash", [hex(0xd8c9a8), hex(0xb8a578), hex(0x8f7c50)]),
    ("cherry", [hex(0xa8543a), hex(0x833f2b), hex(0x5c2c1d)]),
    ("mahogany", [hex(0x6b2f2a), hex(0x4e211d), hex(0x331512)]),
    ("maple", [hex(0xe8d3a3), hex(0xcbb27e), hex(0xa68f5f)]),
    ("ebony", [hex(0x3a332f), hex(0x26211e), hex(0x141110)]),
    ("pine", [hex(0xe3c68f), hex(0xc9a468), hex(0xa9824a)]),
];

/// Web `PAINT_PRESETS`: grain tinted toward a paint colour.
pub const PAINT_PRESETS: [(&str, [Rgba; 3]); 4] = [
    ("sage", [hex(0x9fb08a), hex(0x7d9166), hex(0x5c6e47)]),
    ("slate", [hex(0x7a8a95), hex(0x5e6d78), hex(0x42505a)]),
    ("terracotta", [hex(0xc97a56), hex(0xa85f3f), hex(0x824628)]),
    ("charcoal", [hex(0x5a5a5a), hex(0x404040), hex(0x2a2a2a)]),
];

fn lookup(table: &[(&str, [Rgba; 3])], id: &str) -> Option<[Rgba; 3]> {
    table.iter().find(|(k, _)| *k == id).map(|(_, c)| *c)
}

pub fn wood_colours(id: &str) -> [Rgba; 3] {
    lookup(&WOOD_PRESETS, id).unwrap_or(WOOD_PRESETS[0].1)
}

pub fn paint_colours(id: &str) -> [Rgba; 3] {
    lookup(&PAINT_PRESETS, id)
        .or_else(|| lookup(&WOOD_PRESETS, id))
        .unwrap_or(WOOD_PRESETS[0].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mulberry32_matches_js_reference() {
        // JS: const r = mulberry32(42); [r(), r(), r()]
        let mut r = mulberry32(42);
        let got = [r(), r(), r()];
        let expected = [0.6011037519201636, 0.44829055899754167, 0.8524657934904099];
        for (g, e) in got.iter().zip(expected) {
            assert!((g - e).abs() < 1e-15, "{got:?}");
        }
    }

    #[test]
    fn fnv1a_matches_js_reference() {
        assert_eq!(hash_seed(""), 0x811c9dc5);
        assert_eq!(hash_seed("walnut"), 3350065134);
        assert_eq!(hash_seed("a"), 0xe40c292c);
    }

    #[test]
    fn matches_web_geometry_for_oak() {
        // Reference from web buildWoodGrainGeometry("oak", ...) run under node.
        let bands = build("oak", wood_colours("oak"));
        assert_eq!(bands.len(), 52);
        assert_eq!(bands[0].colour.to_hex(), "#987346");
        assert_eq!(
            &bands[0].outer[..3],
            &[
                Point::new(0.4943, 0.5),
                Point::new(0.4943, 0.4995),
                Point::new(0.4944, 0.499)
            ]
        );
        assert_eq!(bands[17].colour.to_hex(), "#c39a66");
        assert_eq!(
            &bands[17].outer[..3],
            &[
                Point::new(0.7167, 0.5),
                Point::new(0.716, 0.5189),
                Point::new(0.7134, 0.5376)
            ]
        );
    }

    #[test]
    fn deterministic_band_count_and_colours() {
        let a = wood_grain("oak", wood_colours("oak"));
        let b = build("oak", wood_colours("oak"));
        assert_eq!(*a, b);
        assert_eq!(a.len(), HUMP_COUNT * BANDS_PER_HUMP);
        assert!(a[0].inner.is_none() && a[1].inner.is_some());
        assert_eq!(a[0].outer.len(), RING_ANGLE_STEPS);
        assert_ne!(
            build("walnut", wood_colours("walnut"))[5].outer,
            build("pine", wood_colours("walnut"))[5].outer
        );
    }
}
