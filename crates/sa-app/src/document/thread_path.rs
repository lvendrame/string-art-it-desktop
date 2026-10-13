//! Thread Paths (spec 12). Port of web `threadPath.ts`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ids::next_id;

/// `twist_pitch` is a unitless multiplier of strand width controlling how
/// tightly a 2/3-colour thread's strands twist.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadPath {
    pub id: String,
    pub colours: Vec<String>,
    pub width: f64,
    pub pin_ids: Vec<String>,
    #[serde(default = "default_twist_pitch")]
    pub twist_pitch: f64,
}

pub const DEFAULT_TWIST_PITCH: f64 = 6.0;

fn default_twist_pitch() -> f64 {
    DEFAULT_TWIST_PITCH
}

fn next_thread_id() -> String {
    next_id("threadpath")
}

impl ThreadPath {
    pub fn new(pin_ids: Vec<String>, colours: Vec<String>, width: f64, twist_pitch: f64) -> Self {
        Self {
            id: next_thread_id(),
            colours,
            width,
            pin_ids,
            twist_pitch,
        }
    }

    pub fn segment_count(&self) -> usize {
        self.pin_ids.len().saturating_sub(1)
    }

    fn fragment(&self, pin_ids: Vec<String>) -> Self {
        Self {
            id: next_thread_id(),
            pin_ids,
            ..self.clone()
        }
    }

    /// Spec 11: removing a pin removes every segment touching it; fragments with
    /// fewer than 2 pins are dropped.
    pub fn without_pin(&self, pin_id: &str) -> Vec<ThreadPath> {
        if !self.pin_ids.iter().any(|id| id == pin_id) {
            return vec![self.clone()];
        }
        let mut fragments = Vec::new();
        let mut current = Vec::new();
        for id in &self.pin_ids {
            if id == pin_id {
                if current.len() >= 2 {
                    fragments.push(self.fragment(std::mem::take(&mut current)));
                }
                current.clear();
            } else {
                current.push(id.clone());
            }
        }
        if current.len() >= 2 {
            fragments.push(self.fragment(current));
        }
        fragments
    }

    /// Spec 11 Segment Eraser: cut edge `segment_index` out, keeping both pins.
    pub fn split_at_segment(&self, segment_index: usize) -> Vec<ThreadPath> {
        let cut = (segment_index + 1).min(self.pin_ids.len());
        let (before, after) = self.pin_ids.split_at(cut);
        [before, after]
            .into_iter()
            .filter(|part| part.len() >= 2)
            .map(|part| self.fragment(part.to_vec()))
            .collect()
    }

    /// Merge tool: every merged-away id becomes `new_pin_id`; adjacent duplicates
    /// collapse. Keeps the same path id (a contraction, not a fragment).
    pub fn remap_pins(&self, old_pin_ids: &HashSet<String>, new_pin_id: &str) -> Vec<ThreadPath> {
        let mapping = old_pin_ids
            .iter()
            .map(|id| (id.clone(), new_pin_id.to_owned()))
            .collect();
        self.remap_pins_by_map(&mapping)
    }

    pub fn remap_pins_by_map(&self, mapping: &HashMap<String, String>) -> Vec<ThreadPath> {
        let mut collapsed: Vec<String> = Vec::with_capacity(self.pin_ids.len());
        for id in &self.pin_ids {
            let mapped = mapping.get(id).unwrap_or(id);
            if collapsed.last() != Some(mapped) {
                collapsed.push(mapped.clone());
            }
        }
        if collapsed.len() >= 2 {
            vec![Self {
                pin_ids: collapsed,
                ..self.clone()
            }]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(ids: &[&str]) -> ThreadPath {
        ThreadPath {
            id: "t1".into(),
            colours: vec!["red".into()],
            width: 1.0,
            pin_ids: ids.iter().map(|s| (*s).to_owned()).collect(),
            twist_pitch: 6.0,
        }
    }

    fn ids(paths: &[ThreadPath]) -> Vec<Vec<&str>> {
        paths
            .iter()
            .map(|p| p.pin_ids.iter().map(String::as_str).collect())
            .collect()
    }

    #[test]
    fn remove_pin() {
        assert_eq!(
            ids(&t(&["A", "B", "C"]).without_pin("A")),
            vec![vec!["B", "C"]]
        );
        assert!(t(&["A", "B", "C"]).without_pin("B").is_empty());
        assert_eq!(
            ids(&t(&["A", "B", "C", "D", "E"]).without_pin("C")),
            vec![vec!["A", "B"], vec!["D", "E"]]
        );
        assert_eq!(
            t(&["A", "B", "C"]).without_pin("Z"),
            vec![t(&["A", "B", "C"])]
        );
        assert!(t(&["A", "B"]).without_pin("B").is_empty());
    }

    #[test]
    fn split_segment() {
        assert_eq!(
            ids(&t(&["A", "B", "C", "D", "E"]).split_at_segment(1)),
            vec![vec!["A", "B"], vec!["C", "D", "E"]]
        );
        assert_eq!(
            ids(&t(&["A", "B", "C"]).split_at_segment(0)),
            vec![vec!["B", "C"]]
        );
        assert!(t(&["A", "B"]).split_at_segment(0).is_empty());
        let f = t(&["A", "B", "C", "D"]).split_at_segment(1);
        assert_ne!(f[0].id, f[1].id);
        assert_ne!(f[0].id, "t1");
    }

    #[test]
    fn remap() {
        let set = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<HashSet<_>>();
        let r = t(&["A", "B", "C", "D", "E"]).remap_pins(&set(&["B", "C"]), "M");
        assert_eq!(ids(&r), vec![vec!["A", "M", "D", "E"]]);
        assert_eq!(r[0].id, "t1");
        assert!(t(&["A", "B"]).remap_pins(&set(&["A", "B"]), "M").is_empty());
        let map: HashMap<String, String> = [("B", "M"), ("C", "M")]
            .iter()
            .map(|(a, b)| ((*a).into(), (*b).into()))
            .collect();
        assert_eq!(
            ids(&t(&["A", "B", "C", "D"]).remap_pins_by_map(&map)),
            vec![vec!["A", "M", "D"]]
        );
    }

    #[test]
    fn missing_twist_pitch_defaults() {
        let t: ThreadPath =
            serde_json::from_str(r##"{"id":"t","colours":["#f00"],"width":1,"pinIds":["a","b"]}"##)
                .unwrap();
        assert_eq!(t.twist_pitch, DEFAULT_TWIST_PITCH);
    }
}
