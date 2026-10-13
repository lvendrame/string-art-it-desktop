//! Pin and Thread layers (spec 13, 36). Port of web `pinLayer.ts`,
//! `threadLayer.ts`, `layerOps.ts`. Operations mutate in place; undo is handled
//! by document snapshots in [`crate::history`].

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::pin_path::{Pin, PinPath};
use super::thread_path::ThreadPath;
use crate::ids::next_id;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinLayer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub pin_paths: Vec<PinPath>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadLayer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub thread_paths: Vec<ThreadPath>,
}

/// Shared `{id, name, visible, locked}` shape for generic layer operations.
pub trait Layer: Clone {
    fn id(&self) -> &str;
    fn name_mut(&mut self) -> &mut String;
    fn visible_mut(&mut self) -> &mut bool;
    fn locked_mut(&mut self) -> &mut bool;
    fn is_locked(&self) -> bool;
    /// Duplicate with fresh ids and a " copy" name suffix.
    fn duplicated(&self) -> Self;
    /// Appends `other`'s content after this layer's own.
    fn absorb(&mut self, other: Self);
}

impl Layer for PinLayer {
    fn id(&self) -> &str {
        &self.id
    }
    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    fn visible_mut(&mut self) -> &mut bool {
        &mut self.visible
    }
    fn locked_mut(&mut self) -> &mut bool {
        &mut self.locked
    }
    fn is_locked(&self) -> bool {
        self.locked
    }
    fn duplicated(&self) -> Self {
        Self {
            id: next_id("pinlayer-dup"),
            name: format!("{} copy", self.name),
            pin_paths: self.pin_paths.iter().map(PinPath::cloned_fresh).collect(),
            ..self.clone()
        }
    }
    fn absorb(&mut self, other: Self) {
        self.pin_paths.extend(other.pin_paths);
    }
}

impl Layer for ThreadLayer {
    fn id(&self) -> &str {
        &self.id
    }
    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    fn visible_mut(&mut self) -> &mut bool {
        &mut self.visible
    }
    fn locked_mut(&mut self) -> &mut bool {
        &mut self.locked
    }
    fn is_locked(&self) -> bool {
        self.locked
    }
    fn duplicated(&self) -> Self {
        Self {
            id: next_id("threadlayer-dup"),
            name: format!("{} copy", self.name),
            thread_paths: self
                .thread_paths
                .iter()
                .map(|t| {
                    ThreadPath::new(t.pin_ids.clone(), t.colours.clone(), t.width, t.twist_pitch)
                })
                .collect(),
            ..self.clone()
        }
    }
    fn absorb(&mut self, other: Self) {
        self.thread_paths.extend(other.thread_paths);
    }
}

impl PinLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: next_id("pinlayer"),
            name: name.into(),
            visible: true,
            locked: false,
            pin_paths: Vec::new(),
        }
    }

    pub fn path(&self, path_id: &str) -> Option<&PinPath> {
        self.pin_paths.iter().find(|p| p.id == path_id)
    }

    pub fn path_mut(&mut self, path_id: &str) -> Option<&mut PinPath> {
        self.pin_paths.iter_mut().find(|p| p.id == path_id)
    }
}

impl ThreadLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: next_id("threadlayer"),
            name: name.into(),
            visible: true,
            locked: false,
            thread_paths: Vec::new(),
        }
    }

    pub fn path(&self, path_id: &str) -> Option<&ThreadPath> {
        self.thread_paths.iter().find(|p| p.id == path_id)
    }

    pub fn path_mut(&mut self, path_id: &str) -> Option<&mut ThreadPath> {
        self.thread_paths.iter_mut().find(|p| p.id == path_id)
    }
}

// ---- generic layer-list operations (layerOps.ts)

pub fn find_layer<'a, L: Layer>(layers: &'a [L], id: &str) -> Option<&'a L> {
    layers.iter().find(|l| l.id() == id)
}

pub fn find_layer_mut<'a, L: Layer>(layers: &'a mut [L], id: &str) -> Option<&'a mut L> {
    layers.iter_mut().find(|l| l.id() == id)
}

pub fn is_layer_locked<L: Layer>(layers: &[L], id: &str) -> bool {
    find_layer(layers, id).is_some_and(Layer::is_locked)
}

pub fn rename_layer<L: Layer>(layers: &mut [L], id: &str, name: &str) {
    if let Some(l) = find_layer_mut(layers, id) {
        *l.name_mut() = name.to_owned();
    }
}

pub fn toggle_layer_visible<L: Layer>(layers: &mut [L], id: &str) {
    if let Some(l) = find_layer_mut(layers, id) {
        let v = l.visible_mut();
        *v = !*v;
    }
}

pub fn toggle_layer_locked<L: Layer>(layers: &mut [L], id: &str) {
    if let Some(l) = find_layer_mut(layers, id) {
        let v = l.locked_mut();
        *v = !*v;
    }
}

pub fn delete_layer<L: Layer>(layers: &mut Vec<L>, id: &str) {
    layers.retain(|l| l.id() != id);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// Up = earlier in the stacking list.
pub fn reorder_layer<L: Layer>(layers: &mut [L], id: &str, direction: Direction) {
    let Some(index) = layers.iter().position(|l| l.id() == id) else {
        return;
    };
    let target = match direction {
        Direction::Up => index.checked_sub(1),
        Direction::Down => (index + 1 < layers.len()).then_some(index + 1),
    };
    if let Some(target) = target {
        layers.swap(index, target);
    }
}

/// Appends a duplicate of the layer (web appends at the end); returns its id.
pub fn duplicate_layer<L: Layer>(layers: &mut Vec<L>, id: &str) -> Option<String> {
    let dup = find_layer(layers, id)?.duplicated();
    let new_id = dup.id().to_owned();
    layers.push(dup);
    Some(new_id)
}

/// Spec 36: moves the layer's content into the layer above and removes it.
/// No-op for the topmost layer. Ids are kept so thread references still resolve.
pub fn merge_layer_above<L: Layer>(layers: &mut Vec<L>, id: &str) -> bool {
    let Some(index) = layers.iter().position(|l| l.id() == id) else {
        return false;
    };
    if index == 0 {
        return false;
    }
    let source = layers.remove(index);
    layers[index - 1].absorb(source);
    true
}

// ---- pin-specific operations (pinLayer.ts)

/// Threads reference pins by document-wide id; also searches mirrored pins.
pub fn find_pin_by_id(layers: &[PinLayer], pin_id: &str) -> Option<Pin> {
    // Fast path: real pins.
    for l in layers {
        for p in &l.pin_paths {
            if let Some(pin) = p.pins.iter().find(|x| x.id == pin_id) {
                return Some(pin.clone());
            }
        }
    }
    if !pin_id.contains("~mirror-") {
        return None;
    }
    layers
        .iter()
        .flat_map(|l| &l.pin_paths)
        .flat_map(PinPath::mirrored_pin_groups)
        .flatten()
        .find(|p| p.id == pin_id)
}

/// All addressable pins (real + mirrored) indexed by id — use when resolving
/// many thread endpoints at once (rendering, export).
pub fn pin_index(layers: &[PinLayer]) -> HashMap<String, Pin> {
    layers
        .iter()
        .flat_map(|l| &l.pin_paths)
        .flat_map(PinPath::all_pins_with_mirrors)
        .map(|p| (p.id.clone(), p))
        .collect()
}

pub fn erase_pin(layers: &mut [PinLayer], layer_id: &str, path_id: &str, pin_id: &str) {
    if let Some(path) = find_layer_mut(layers, layer_id).and_then(|l| l.path_mut(path_id)) {
        path.pins.retain(|p| p.id != pin_id);
    }
}

/// Merge tool: strips merged-away pins everywhere, then adds `new_pin` to the
/// destination path.
pub fn merge_pins(
    layers: &mut [PinLayer],
    old_pin_ids: &HashSet<String>,
    destination_layer: &str,
    destination_path: &str,
    new_pin: Pin,
) {
    for path in layers.iter_mut().flat_map(|l| l.pin_paths.iter_mut()) {
        path.pins.retain(|p| !old_pin_ids.contains(&p.id));
    }
    if let Some(path) =
        find_layer_mut(layers, destination_layer).and_then(|l| l.path_mut(destination_path))
    {
        path.pins.push(new_pin);
    }
}

// ---- thread cascades (threadLayer.ts), applied across every layer at once so
// callers fold them into the same undo step.

pub fn remove_pin_from_all_threads(layers: &mut [ThreadLayer], pin_id: &str) {
    for l in layers {
        l.thread_paths = l
            .thread_paths
            .iter()
            .flat_map(|t| t.without_pin(pin_id))
            .collect();
    }
}

pub fn split_thread_in_layer(
    layers: &mut [ThreadLayer],
    layer_id: &str,
    path_id: &str,
    segment_index: usize,
) {
    if let Some(l) = find_layer_mut(layers, layer_id) {
        l.thread_paths = l
            .thread_paths
            .iter()
            .flat_map(|t| {
                if t.id == path_id {
                    t.split_at_segment(segment_index)
                } else {
                    vec![t.clone()]
                }
            })
            .collect();
    }
}

pub fn remap_pins_in_all_threads(
    layers: &mut [ThreadLayer],
    old_pin_ids: &HashSet<String>,
    new_pin_id: &str,
) {
    for l in layers {
        l.thread_paths = l
            .thread_paths
            .iter()
            .flat_map(|t| t.remap_pins(old_pin_ids, new_pin_id))
            .collect();
    }
}

pub fn remap_pins_in_all_threads_by_map(
    layers: &mut [ThreadLayer],
    mapping: &HashMap<String, String>,
) {
    for l in layers {
        l.thread_paths = l
            .thread_paths
            .iter()
            .flat_map(|t| t.remap_pins_by_map(mapping))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::pin_path::{PinPathGeometry, PinStyle};
    use crate::document::symmetry::SymmetryConfig;
    use sa_domain::Point;

    fn names<L: Layer>(layers: &mut [L]) -> Vec<String> {
        layers.iter_mut().map(|l| l.name_mut().clone()).collect()
    }

    #[test]
    fn generic_ops() {
        let mut ls = vec![PinLayer::new("A"), PinLayer::new("B"), PinLayer::new("C")];
        let b = ls[1].id.clone();
        rename_layer(&mut ls, &b, "Bee");
        toggle_layer_visible(&mut ls, &b);
        toggle_layer_locked(&mut ls, &b);
        assert!(!ls[1].visible && ls[1].locked && is_layer_locked(&ls, &b));
        reorder_layer(&mut ls, &b, Direction::Up);
        assert_eq!(names(&mut ls), ["Bee", "A", "C"]);
        reorder_layer(&mut ls, &b, Direction::Up); // already top: no-op
        assert_eq!(names(&mut ls), ["Bee", "A", "C"]);
        let c = ls[2].id.clone();
        reorder_layer(&mut ls, &c, Direction::Down); // already bottom
        delete_layer(&mut ls, &b);
        assert_eq!(names(&mut ls), ["A", "C"]);
        let dup = duplicate_layer(&mut ls, &c).unwrap();
        assert_eq!(names(&mut ls), ["A", "C", "C copy"]); // appended at end
        assert!(dup.starts_with("pinlayer-dup-"));
    }

    #[test]
    fn duplicate_gives_fresh_pin_ids_and_merge_keeps_them() {
        let style = PinStyle {
            colour: "#fff".into(),
            diameter: 1.0,
            guide_visible: true,
        };
        let mut top = PinLayer::new("top");
        let mut bottom = PinLayer::new("bottom");
        bottom.pin_paths.push(PinPath::new(
            PinPathGeometry::Circle {
                center: Point::ORIGIN,
                radius: 3.0,
            },
            1.0,
            &style,
            SymmetryConfig::None,
        ));
        let dup = bottom.duplicated();
        assert_ne!(dup.pin_paths[0].pins[0].id, bottom.pin_paths[0].pins[0].id);
        top.pin_paths.clear();
        let pin_id = bottom.pin_paths[0].pins[0].id.clone();
        let bottom_id = bottom.id.clone();
        let mut ls = vec![top, bottom];
        let top_id = ls[0].id.clone();
        assert!(!merge_layer_above(&mut ls, &top_id));
        assert!(merge_layer_above(&mut ls, &bottom_id));
        assert_eq!(ls.len(), 1);
        assert_eq!(find_pin_by_id(&ls, &pin_id).unwrap().id, pin_id);
    }

    #[test]
    fn mirrored_pin_lookup() {
        let style = PinStyle {
            colour: "#fff".into(),
            diameter: 1.0,
            guide_visible: true,
        };
        let mut l = PinLayer::new("L");
        l.pin_paths.push(PinPath::new(
            PinPathGeometry::Line {
                start: Point::new(2.0, 0.0),
                end: Point::new(2.0, 0.0),
            },
            1.0,
            &style,
            SymmetryConfig::Vertical {
                axis: Point::ORIGIN,
            },
        ));
        let id = format!("{}~mirror-0", l.pin_paths[0].pins[0].id);
        let ls = vec![l];
        assert_eq!(
            find_pin_by_id(&ls, &id).unwrap().point(),
            Point::new(-2.0, 0.0)
        );
        assert!(pin_index(&ls).contains_key(&id));
    }
}
