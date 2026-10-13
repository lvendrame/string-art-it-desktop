//! Splits a polyline into dash runs following SVG `stroke-dasharray` /
//! `stroke-dashoffset` semantics, continuous across vertices. Shared by every
//! backend so multi-colour twisted threads look identical everywhere.

use sa_domain::Point;

/// Returns the "on" runs of a dashed polyline. `pattern` alternates on/off
/// lengths; an odd-length pattern is repeated (SVG rule).
pub fn dash_polyline(points: &[Point], pattern: &[f64], offset: f64) -> Vec<Vec<Point>> {
    let mut pattern: Vec<f64> = pattern.iter().map(|v| v.max(0.0)).collect();
    if pattern.len() % 2 == 1 {
        pattern.extend_from_within(..);
    }
    let period: f64 = pattern.iter().sum();
    if points.len() < 2 || period <= 0.0 {
        return vec![points.to_vec()];
    }

    // Position within the pattern at distance 0.
    let mut phase = offset.rem_euclid(period);
    let mut idx = 0;
    while phase >= pattern[idx] {
        phase -= pattern[idx];
        idx = (idx + 1) % pattern.len();
    }
    let mut remaining = pattern[idx] - phase;
    let mut on = idx % 2 == 0;

    let mut runs = Vec::new();
    let mut current: Vec<Point> = if on { vec![points[0]] } else { Vec::new() };
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let seg_len = a.distance(b);
        let mut pos = 0.0;
        while seg_len - pos > remaining {
            pos += remaining;
            let t = pos / seg_len;
            let p = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            if on {
                current.push(p);
                runs.push(std::mem::take(&mut current));
            } else {
                current = vec![p];
            }
            on = !on;
            idx = (idx + 1) % pattern.len();
            remaining = pattern[idx];
        }
        remaining -= seg_len - pos;
        if on {
            current.push(b);
        }
    }
    if on && current.len() >= 2 {
        runs.push(current);
    }
    runs.retain(|r| r.len() >= 2);
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn simple_dash() {
        let runs = dash_polyline(&[p(0.0, 0.0), p(10.0, 0.0)], &[2.0, 3.0], 0.0);
        let xs: Vec<(f64, f64)> = runs.iter().map(|r| (r[0].x, r.last().unwrap().x)).collect();
        assert_eq!(xs, vec![(0.0, 2.0), (5.0, 7.0)]);
    }

    #[test]
    fn offset_shifts_pattern_start() {
        // offset 1 → first dash covers [0,1]
        let runs = dash_polyline(&[p(0.0, 0.0), p(10.0, 0.0)], &[2.0, 3.0], 1.0);
        assert_eq!((runs[0][0].x, runs[0].last().unwrap().x), (0.0, 1.0));
        assert_eq!((runs[1][0].x, runs[1].last().unwrap().x), (4.0, 6.0));
    }

    #[test]
    fn continuous_across_corner() {
        let runs = dash_polyline(&[p(0.0, 0.0), p(1.0, 0.0), p(1.0, 5.0)], &[2.0, 10.0], 0.0);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0], vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)]);
    }
}
