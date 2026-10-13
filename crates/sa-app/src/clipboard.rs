//! Copy / paste (spec 39): a self-contained `stringartit/clipboard` v1 JSON
//! envelope carried as plain text — interoperable with the web app. Port of
//! web `clipboard.ts`.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use sa_domain::Point;
use sa_domain::transforms::{BoundingBox, bounding_box_of};

use crate::document::pin_path::{next_path_id, next_pin_id};
use crate::document::symmetry::mirrored_pin_id;
use crate::document::{Board, Pin, PinLayer, PinPath, ThreadLayer, ThreadPath};
use crate::ids::next_id;

pub const CLIPBOARD_KIND: &str = "stringartit/clipboard";
pub const CLIPBOARD_VERSION: u32 = 1;
/// cm per paste step.
pub const PASTE_OFFSET: f64 = 1.0;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardContent {
    pub pin_paths: Vec<PinPath>,
    pub thread_paths: Vec<ThreadPath>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardPayload {
    pub kind: String,
    pub version: u32,
    pub copy_id: String,
    pub source_board_id: String,
    pub pin_paths: Vec<PinPath>,
    pub thread_paths: Vec<ThreadPath>,
}

impl ClipboardPayload {
    pub fn content(&self) -> ClipboardContent {
        ClipboardContent {
            pin_paths: self.pin_paths.clone(),
            thread_paths: self.thread_paths.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("clipboard serializes")
    }
}

/// Unique-enough token identifying an open board / a copy operation.
pub fn instance_token() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("{:x}-{:x}", nanos, COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn owner_index(paths: &[&PinPath]) -> HashMap<String, PinPath> {
    let mut m = HashMap::new();
    for p in paths {
        for pin in p.all_pins_with_mirrors() {
            m.insert(pin.id, (*p).clone());
        }
    }
    m
}

fn with_resolvable_pins(t: &ThreadPath, owners: &HashMap<String, PinPath>) -> Option<ThreadPath> {
    let ids: Vec<String> = t
        .pin_ids
        .iter()
        .filter(|id| owners.contains_key(*id))
        .cloned()
        .collect();
    (ids.len() >= 2).then(|| ThreadPath {
        pin_ids: ids,
        ..t.clone()
    })
}

fn unique_paths(paths: impl IntoIterator<Item = PinPath>) -> Vec<PinPath> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|p| seen.insert(p.id.clone()))
        .collect()
}

fn all_paths(layers: &[PinLayer]) -> Vec<&PinPath> {
    layers.iter().flat_map(|l| &l.pin_paths).collect()
}

/// Selected Pin Paths plus every thread touching them (and those threads' pins'
/// owner paths, so the content is self-contained).
pub fn collect_from_pin_paths(
    pins: &[PinLayer],
    threads: &[ThreadLayer],
    paths: &[&PinPath],
) -> ClipboardContent {
    let owners = owner_index(&all_paths(pins));
    let selected: HashSet<String> = paths
        .iter()
        .flat_map(|p| p.all_pins_with_mirrors())
        .map(|p| p.id)
        .collect();
    let thread_paths: Vec<ThreadPath> = threads
        .iter()
        .flat_map(|l| &l.thread_paths)
        .filter(|t| t.pin_ids.iter().any(|id| selected.contains(id)))
        .filter_map(|t| with_resolvable_pins(t, &owners))
        .collect();
    let extra = thread_paths
        .iter()
        .flat_map(|t| t.pin_ids.iter().map(|id| owners[id].clone()));
    ClipboardContent {
        pin_paths: unique_paths(
            paths
                .iter()
                .map(|p| (*p).clone())
                .chain(extra)
                .collect::<Vec<_>>(),
        ),
        thread_paths,
    }
}

pub fn collect_from_thread(pins: &[PinLayer], thread: &ThreadPath) -> ClipboardContent {
    let owners = owner_index(&all_paths(pins));
    let Some(t) = with_resolvable_pins(thread, &owners) else {
        return ClipboardContent::default();
    };
    ClipboardContent {
        pin_paths: unique_paths(
            t.pin_ids
                .iter()
                .map(|id| owners[id].clone())
                .collect::<Vec<_>>(),
        ),
        thread_paths: vec![t],
    }
}

pub fn create_payload(content: ClipboardContent, source_board_id: &str) -> ClipboardPayload {
    ClipboardPayload {
        kind: CLIPBOARD_KIND.into(),
        version: CLIPBOARD_VERSION,
        copy_id: instance_token(),
        source_board_id: source_board_id.into(),
        pin_paths: content.pin_paths,
        thread_paths: content.thread_paths,
    }
}

/// Parses + validates an envelope; threads must resolve within its own paths.
pub fn parse_clipboard(text: &str) -> Option<ClipboardPayload> {
    let raw: Value = serde_json::from_str(text).ok()?;
    if raw.get("kind")?.as_str()? != CLIPBOARD_KIND
        || raw.get("version")?.as_f64()? > CLIPBOARD_VERSION as f64
    {
        return None;
    }
    let p: ClipboardPayload = serde_json::from_value(raw).ok()?;
    if p.pin_paths.is_empty() {
        return None;
    }
    let owners = owner_index(&p.pin_paths.iter().collect::<Vec<_>>());
    p.thread_paths
        .iter()
        .all(|t| t.pin_ids.iter().all(|id| owners.contains_key(id)))
        .then_some(p)
}

/// Fresh ids for everything; geometry, pins and symmetry shifted by `delta`.
pub fn instantiate(content: &ClipboardContent, delta: Point) -> ClipboardContent {
    let mut map: HashMap<String, String> = HashMap::new();
    let pin_paths = content
        .pin_paths
        .iter()
        .map(|src| {
            let pins: Vec<Pin> = src
                .pins
                .iter()
                .map(|p| Pin {
                    id: next_pin_id(),
                    x: p.x + delta.x,
                    y: p.y + delta.y,
                })
                .collect();
            for (old, new) in src.pins.iter().zip(&pins) {
                map.insert(old.id.clone(), new.id.clone());
            }
            for g in 0..src.mirrored_pin_groups().len() {
                for (old, new) in src.pins.iter().zip(&pins) {
                    map.insert(mirrored_pin_id(&old.id, g), mirrored_pin_id(&new.id, g));
                }
            }
            PinPath {
                id: next_path_id(),
                geometry: src.geometry.translated(delta),
                symmetry: src.symmetry.translated(delta),
                pins,
                ..src.clone()
            }
        })
        .collect();
    let thread_paths = content
        .thread_paths
        .iter()
        .map(|t| ThreadPath {
            id: next_id("threadpath"),
            pin_ids: t
                .pin_ids
                .iter()
                .map(|id| map.get(id).cloned().unwrap_or_else(|| id.clone()))
                .collect(),
            ..t.clone()
        })
        .collect();
    ClipboardContent {
        pin_paths,
        thread_paths,
    }
}

fn content_box(paths: &[PinPath]) -> BoundingBox {
    bounding_box_of(
        &paths
            .iter()
            .flat_map(PinPath::all_pins_with_mirrors)
            .map(|p| p.point())
            .collect::<Vec<_>>(),
    )
}

/// Spec 39 placement: cascade by 1 cm per paste; re-centre on the board when the
/// cascaded content would leave it.
pub fn paste_delta(
    content: &ClipboardContent,
    board: &Board,
    paste_index: usize,
    same_board: bool,
) -> Point {
    let cascade = |n: usize| Point::new(n as f64 * PASTE_OFFSET, n as f64 * PASTE_OFFSET);
    let offset = cascade(if same_board {
        paste_index
    } else {
        paste_index - 1
    });
    let src = content_box(&content.pin_paths);
    let target = bounding_box_of(&board.path().bounding_box_points());
    let inside = src.min_x + offset.x >= target.min_x
        && src.max_x + offset.x <= target.max_x
        && src.min_y + offset.y >= target.min_y
        && src.max_y + offset.y <= target.max_y;
    if inside {
        return offset;
    }
    let (from, to) = (src.center(), target.center());
    let r = cascade(paste_index - 1);
    Point::new(to.x - from.x + r.x, to.y - from.y + r.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{PinPathGeometry, PinStyle, SymmetryConfig};

    fn path() -> PinPath {
        PinPath::new(
            PinPathGeometry::Line {
                start: Point::ORIGIN,
                end: Point::new(3.0, 0.0),
            },
            1.0,
            &PinStyle {
                colour: "#fff".into(),
                diameter: 1.0,
                guide_visible: true,
            },
            SymmetryConfig::Vertical {
                axis: Point::new(-1.0, 0.0),
            },
        )
    }

    #[test]
    fn round_trip_and_instantiate_remaps_mirrors() {
        let p = path();
        let mirror = format!("{}~mirror-0", p.pins[1].id);
        let t = ThreadPath::new(
            vec![p.pins[0].id.clone(), mirror],
            vec!["#f00".into()],
            1.0,
            6.0,
        );
        let payload = create_payload(
            ClipboardContent {
                pin_paths: vec![p.clone()],
                thread_paths: vec![t],
            },
            "board",
        );
        let parsed = parse_clipboard(&payload.to_json()).unwrap();
        assert_eq!(parsed, payload);
        let inst = instantiate(&parsed.content(), Point::new(1.0, 1.0));
        let np = &inst.pin_paths[0];
        assert_ne!(np.id, p.id);
        assert_eq!(np.pins[0].point(), Point::new(1.0, 1.0));
        assert_eq!(
            np.symmetry,
            SymmetryConfig::Vertical {
                axis: Point::new(0.0, 1.0)
            }
        );
        assert_eq!(
            inst.thread_paths[0].pin_ids[1],
            format!("{}~mirror-0", np.pins[1].id)
        );
    }

    #[test]
    fn rejects_foreign_text() {
        assert!(parse_clipboard("hello").is_none());
        assert!(parse_clipboard(r#"{"kind":"other"}"#).is_none());
    }

    #[test]
    fn placement_cascades_then_recentres() {
        let c = ClipboardContent {
            pin_paths: vec![path()],
            thread_paths: vec![],
        };
        let board = Board::default();
        assert_eq!(paste_delta(&c, &board, 1, true), Point::new(1.0, 1.0));
        assert_eq!(paste_delta(&c, &board, 1, false), Point::ORIGIN);
    }
}
