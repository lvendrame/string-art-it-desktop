//! Statistics (spec 17). Port of web `statistics.ts`.

use crate::document::layers::find_pin_by_id;
use crate::document::{PinLayer, PinPath, ThreadLayer, ThreadPath};

#[derive(Clone, Debug, PartialEq)]
pub struct PinPathStatistics {
    pub pins: usize,
    pub requested_spacing: f64,
    pub actual_spacing: f64,
    pub diameter: f64,
    pub perimeter_cm: f64,
}

/// Perimeter sums every contour (multi-contour text paths count each piece).
pub fn pin_path_statistics(path: &PinPath) -> PinPathStatistics {
    PinPathStatistics {
        pins: path.pins.len(),
        requested_spacing: path.requested_spacing,
        actual_spacing: path.actual_spacing,
        diameter: path.diameter,
        perimeter_cm: path
            .geometry
            .contour_paths()
            .iter()
            .map(|c| c.length())
            .sum(),
    }
}

pub fn project_total_pins(layers: &[PinLayer]) -> usize {
    layers
        .iter()
        .flat_map(|l| &l.pin_paths)
        .map(|p| p.pins.len())
        .sum()
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThreadPathStatistics {
    pub segments: usize,
    pub length_cm: f64,
    /// Occurrences along `pin_ids`, repeats included.
    pub pins_visited: usize,
    pub colours: Vec<String>,
}

pub fn thread_path_statistics(
    thread: &ThreadPath,
    pin_layers: &[PinLayer],
) -> ThreadPathStatistics {
    let pts: Vec<_> = thread
        .pin_ids
        .iter()
        .map(|id| find_pin_by_id(pin_layers, id))
        .collect();
    let length_cm = pts
        .windows(2)
        .filter_map(|w| Some((w[0].as_ref()?, w[1].as_ref()?)))
        .map(|(a, b)| (b.x - a.x).hypot(b.y - a.y))
        .sum();
    ThreadPathStatistics {
        segments: thread.pin_ids.len().saturating_sub(1),
        length_cm,
        pins_visited: thread.pin_ids.len(),
        colours: thread.colours.clone(),
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectThreadTotals {
    pub thread_count: usize,
    pub total_length_cm: f64,
}

pub fn project_thread_totals(
    thread_layers: &[ThreadLayer],
    pin_layers: &[PinLayer],
) -> ProjectThreadTotals {
    let mut t = ProjectThreadTotals::default();
    for th in thread_layers.iter().flat_map(|l| &l.thread_paths) {
        t.thread_count += 1;
        t.total_length_cm += thread_path_statistics(th, pin_layers).length_cm;
    }
    t
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThreadTypeStatistics {
    pub colours: Vec<String>,
    pub width: f64,
    pub thread_count: usize,
    pub total_segments: usize,
    pub total_length_cm: f64,
    pub total_pins_visited: usize,
}

/// Groups threads by their exact (colours, width) combination, in first-seen order.
pub fn thread_statistics_by_type(
    thread_layers: &[ThreadLayer],
    pin_layers: &[PinLayer],
) -> Vec<ThreadTypeStatistics> {
    let mut out: Vec<ThreadTypeStatistics> = Vec::new();
    for th in thread_layers.iter().flat_map(|l| &l.thread_paths) {
        let s = thread_path_statistics(th, pin_layers);
        let idx = match out
            .iter()
            .position(|b| b.colours == th.colours && b.width == th.width)
        {
            Some(i) => i,
            None => {
                out.push(ThreadTypeStatistics {
                    colours: th.colours.clone(),
                    width: th.width,
                    thread_count: 0,
                    total_segments: 0,
                    total_length_cm: 0.0,
                    total_pins_visited: 0,
                });
                out.len() - 1
            }
        };
        let b = &mut out[idx];
        b.thread_count += 1;
        b.total_segments += s.segments;
        b.total_length_cm += s.length_cm;
        b.total_pins_visited += s.pins_visited;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Pin, PinPathGeometry, PinStyle, SymmetryConfig};
    use sa_domain::Point;

    fn layers() -> (Vec<PinLayer>, Vec<ThreadLayer>) {
        let mut pl = PinLayer::new("P");
        let mut p = PinPath::new(
            PinPathGeometry::Line {
                start: Point::new(0.0, 0.0),
                end: Point::new(3.0, 4.0),
            },
            5.0,
            &PinStyle {
                colour: "#fff".into(),
                diameter: 2.0,
                guide_visible: true,
            },
            SymmetryConfig::None,
        );
        p.pins = vec![
            Pin {
                id: "a".into(),
                x: 0.0,
                y: 0.0,
            },
            Pin {
                id: "b".into(),
                x: 3.0,
                y: 4.0,
            },
        ];
        pl.pin_paths.push(p);
        let mut tl = ThreadLayer::new("T");
        tl.thread_paths.push(ThreadPath::new(
            vec!["a".into(), "b".into(), "a".into()],
            vec!["red".into()],
            1.0,
            6.0,
        ));
        tl.thread_paths.push(ThreadPath::new(
            vec!["a".into(), "b".into()],
            vec!["red".into()],
            1.0,
            6.0,
        ));
        tl.thread_paths.push(ThreadPath::new(
            vec!["a".into(), "b".into()],
            vec!["red".into(), "white".into()],
            1.0,
            6.0,
        ));
        (vec![pl], vec![tl])
    }

    #[test]
    fn per_thread_and_by_type() {
        let (pl, tl) = layers();
        let s = thread_path_statistics(&tl[0].thread_paths[0], &pl);
        assert_eq!((s.segments, s.pins_visited), (2, 3));
        assert!((s.length_cm - 10.0).abs() < 1e-9);
        let types = thread_statistics_by_type(&tl, &pl);
        assert_eq!(types.len(), 2);
        assert_eq!(
            (
                types[0].thread_count,
                types[0].total_segments,
                types[0].total_pins_visited
            ),
            (2, 3, 5)
        );
        assert!((types[0].total_length_cm - 15.0).abs() < 1e-9);
        assert_eq!(project_total_pins(&pl), 2);
        let tot = project_thread_totals(&tl, &pl);
        assert_eq!(tot.thread_count, 3);
        assert!((pin_path_statistics(&pl[0].pin_paths[0]).perimeter_cm - 5.0).abs() < 1e-9);
    }

    #[test]
    fn missing_pins_skip_segments() {
        let (pl, _) = layers();
        let t = ThreadPath::new(vec!["a".into(), "zz".into(), "b".into()], vec![], 1.0, 6.0);
        let s = thread_path_statistics(&t, &pl);
        assert_eq!(s.length_cm, 0.0);
        assert_eq!(s.segments, 2);
    }
}
