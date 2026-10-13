//! Edit mode: selection, Move/Rotate/Scale transforms, Merge, Delete (specs
//! 09, 21, 23, 26). Port of the selection half of web `EditorStore.ts` plus
//! `multiSelect.ts` and the transform hooks' commit math.

use std::collections::{HashMap, HashSet};

use sa_domain::Point;
use sa_domain::transforms::{rotate_point, scale_point};

use crate::document::layers::{self, find_layer, find_layer_mut, is_layer_locked};
use crate::document::symmetry::build_nearest_pin_remap;
use crate::document::{Document, Pin, PinLayer, PinPath, PinPathGeometry, pin_path::next_pin_id};
use crate::editor::{PinPathRef, PinRef, SelectGranularity, SelectTool, Selection, Store};

fn mean(points: impl IntoIterator<Item = Point>) -> Point {
    let (mut sx, mut sy, mut n) = (0.0, 0.0, 0usize);
    for p in points {
        sx += p.x;
        sy += p.y;
        n += 1;
    }
    if n == 0 {
        Point::ORIGIN
    } else {
        Point::new(sx / n as f64, sy / n as f64)
    }
}

/// Spec 26 pivot: one path → its own centroid; 2+ → mean of all real pins.
pub fn selection_centroid(paths: &[&PinPath]) -> Point {
    match paths {
        [one] => one.geometry.center(),
        _ => mean(paths.iter().flat_map(|p| p.pins.iter().map(Pin::point))),
    }
}

pub fn pins_centroid(pins: &[Pin]) -> Point {
    mean(pins.iter().map(Pin::point))
}

const MERGE_POSITION_TOLERANCE: f64 = 1e-6;

/// Path-mode Merge: pool pins, collapse coincident ones, remint every id.
pub fn combine_pin_paths(paths: &[&PinPath]) -> (Vec<Pin>, HashMap<String, String>) {
    let pooled: Vec<&Pin> = paths.iter().flat_map(|p| &p.pins).collect();
    let n = pooled.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..n {
        for j in i + 1..n {
            if pooled[i].point().distance(pooled[j].point()) <= MERGE_POSITION_TOLERANCE {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[a] = b;
                }
            }
        }
    }
    let mut order: Vec<usize> = Vec::new();
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        if !groups.contains_key(&r) {
            order.push(r);
        }
        groups.entry(r).or_default().push(i);
    }
    let mut pins = Vec::new();
    let mut remap = HashMap::new();
    for r in order {
        let members = &groups[&r];
        let c = mean(members.iter().map(|&i| pooled[i].point()));
        let pin = Pin {
            id: next_pin_id(),
            x: c.x,
            y: c.y,
        };
        for &i in members {
            remap.insert(pooled[i].id.clone(), pin.id.clone());
        }
        pins.push(pin);
    }
    (pins, remap)
}

fn path_numeric_id(id: &str) -> u64 {
    id.rsplit_once('-')
        .and_then(|(_, n)| n.parse().ok())
        .unwrap_or(u64::MAX)
}

/// Pins-mode Merge destination: most selected pins → closest mean → oldest id.
pub fn resolve_merge_destination(
    layers: &[PinLayer],
    refs: &[PinRef],
    new_pos: Point,
) -> Option<PinPathRef> {
    let mut by_path: Vec<(PinPathRef, Vec<Point>)> = Vec::new();
    for r in refs {
        let key = PinPathRef {
            layer_id: r.layer_id.clone(),
            path_id: r.path_id.clone(),
        };
        let pin = find_layer(layers, &r.layer_id)
            .and_then(|l| l.path(&r.path_id))
            .and_then(|p| p.pins.iter().find(|q| q.id == r.pin_id))
            .map(Pin::point);
        match by_path.iter_mut().find(|(k, _)| *k == key) {
            Some((_, pts)) => pts.extend(pin),
            None => by_path.push((key, pin.into_iter().collect())),
        }
    }
    let max = by_path.iter().map(|(_, p)| p.len()).max()?;
    let mut tied: Vec<_> = by_path
        .into_iter()
        .filter(|(_, p)| p.len() == max)
        .collect();
    if tied.len() > 1 {
        let d: Vec<f64> = tied
            .iter()
            .map(|(_, p)| mean(p.iter().copied()).distance(new_pos))
            .collect();
        let min = d.iter().copied().fold(f64::INFINITY, f64::min);
        tied = tied
            .into_iter()
            .zip(d)
            .filter(|(_, d)| (d - min).abs() < 1e-9)
            .map(|(t, _)| t)
            .collect();
    }
    tied.into_iter()
        .min_by_key(|(k, _)| path_numeric_id(&k.path_id))
        .map(|(k, _)| k)
}

/// A live transform of the current selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Transform {
    Move(Point),
    Rotate { pivot: Point, theta: f64 },
    Scale { pivot: Point, factor: f64 },
}

impl Transform {
    fn point(&self, p: Point) -> Point {
        match *self {
            Transform::Move(d) => Point::new(p.x + d.x, p.y + d.y),
            Transform::Rotate { pivot, theta } => rotate_point(p, pivot, theta),
            Transform::Scale { pivot, factor } => scale_point(p, pivot, factor),
        }
    }

    fn geometry(&self, g: &PinPathGeometry) -> PinPathGeometry {
        match *self {
            Transform::Move(d) => g.translated(d),
            Transform::Rotate { pivot, theta } => g.rotated(pivot, theta),
            Transform::Scale { pivot, factor } => g.scaled_about(pivot, factor),
        }
    }
}

/// Applies `t` to `selection` within `doc` (pure). Path granularity: Move/Rotate
/// keep pin ids (pins transformed in place); Scale recomputes pins and reattaches
/// threads to the nearest new pins. Pins granularity: raw x/y of the selected
/// pins only (geometry untouched).
pub fn transformed(doc: &Document, selection: &Selection, t: Transform) -> Document {
    let mut out = doc.clone();
    match selection {
        Selection::PinPaths(refs) => {
            let mut remap = HashMap::new();
            for r in refs {
                let Some(path) = find_layer_mut(&mut out.pin_layers, &r.layer_id)
                    .and_then(|l| l.path_mut(&r.path_id))
                else {
                    continue;
                };
                let old = path.clone();
                path.geometry = t.geometry(&old.geometry);
                if matches!(t, Transform::Scale { .. }) {
                    *path = path.recomputed();
                    remap.extend(build_nearest_pin_remap(&old, path));
                } else {
                    for pin in &mut path.pins {
                        let p = t.point(pin.point());
                        (pin.x, pin.y) = (p.x, p.y);
                    }
                }
            }
            if !remap.is_empty() {
                layers::remap_pins_in_all_threads_by_map(&mut out.thread_layers, &remap);
            }
        }
        Selection::Pins(refs) => {
            for r in refs {
                if let Some(pin) = find_layer_mut(&mut out.pin_layers, &r.layer_id)
                    .and_then(|l| l.path_mut(&r.path_id))
                    .and_then(|p| p.pins.iter_mut().find(|q| q.id == r.pin_id))
                {
                    let p = t.point(pin.point());
                    (pin.x, pin.y) = (p.x, p.y);
                }
            }
        }
        _ => {}
    }
    out
}

/// In-progress Move/Rotate/Scale drag: previews without history, commits once.
#[derive(Clone, Debug)]
pub struct TransformSession {
    before: Document,
    selection: Selection,
}

impl Store {
    pub fn select(&mut self, selection: Selection) {
        self.editor.selection = selection;
    }

    pub fn set_select_tool(&mut self, tool: SelectTool) {
        self.editor.select_tool = tool;
    }

    /// Flipping granularity clears the selection (spec 26).
    pub fn set_select_granularity(&mut self, g: SelectGranularity) {
        self.editor.select_granularity = g;
        self.editor.selection = Selection::None;
    }

    pub fn selected_pin_paths(&self) -> Vec<&PinPath> {
        let Selection::PinPaths(refs) = &self.editor.selection else {
            return Vec::new();
        };
        refs.iter()
            .filter_map(|r| find_layer(&self.doc.pin_layers, &r.layer_id)?.path(&r.path_id))
            .collect()
    }

    pub fn selected_pins(&self) -> Vec<Pin> {
        let Selection::Pins(refs) = &self.editor.selection else {
            return Vec::new();
        };
        refs.iter()
            .filter_map(|r| {
                find_layer(&self.doc.pin_layers, &r.layer_id)?
                    .path(&r.path_id)?
                    .pins
                    .iter()
                    .find(|q| q.id == r.pin_id)
                    .cloned()
            })
            .collect()
    }

    fn selection_layer_ids(&self) -> Vec<String> {
        match &self.editor.selection {
            Selection::PinPaths(r) => r.iter().map(|r| r.layer_id.clone()).collect(),
            Selection::Pins(r) => r.iter().map(|r| r.layer_id.clone()).collect(),
            _ => Vec::new(),
        }
    }

    fn selection_locked(&self) -> bool {
        self.selection_layer_ids()
            .iter()
            .any(|l| is_layer_locked(&self.doc.pin_layers, l))
    }

    pub fn has_transformable_selection(&self) -> bool {
        match &self.editor.selection {
            Selection::PinPaths(r) => !r.is_empty(),
            Selection::Pins(r) => !r.is_empty(),
            _ => false,
        }
    }

    /// Scale/keyboard pivot for the current selection.
    pub fn selection_pivot(&self) -> Option<Point> {
        match &self.editor.selection {
            Selection::PinPaths(_) => {
                let paths = self.selected_pin_paths();
                (!paths.is_empty()).then(|| selection_centroid(&paths))
            }
            Selection::Pins(_) => {
                let pins = self.selected_pins();
                (!pins.is_empty()).then(|| pins_centroid(&pins))
            }
            _ => None,
        }
    }

    pub fn begin_transform(&self) -> Option<TransformSession> {
        self.has_transformable_selection()
            .then(|| TransformSession {
                before: self.doc.clone(),
                selection: self.editor.selection.clone(),
            })
    }

    /// Live preview (no history).
    pub fn preview_transform(&mut self, s: &TransformSession, t: Transform) {
        let next = transformed(&s.before, &s.selection, t);
        self.preview(|d| *d = next);
    }

    /// One undo step; all-or-nothing abort when any involved layer is locked.
    pub fn commit_transform(&mut self, s: TransformSession, t: Transform) {
        if self.selection_locked() {
            self.preview(|d| *d = s.before);
            return;
        }
        let next = transformed(&s.before, &s.selection, t);
        self.preview(|d| *d = next);
        self.commit_from(s.before);
    }

    /// Esc: restore the pre-gesture document.
    pub fn abort_transform(&mut self, s: TransformSession) {
        self.preview(|d| *d = s.before);
    }

    /// One-shot transform (keyboard nudges, spec 23).
    pub fn apply_transform(&mut self, key: &str, t: Transform) {
        if !self.has_transformable_selection() || self.selection_locked() {
            return;
        }
        let selection = self.editor.selection.clone();
        self.commit_coalesced(key, |d| *d = transformed(d, &selection, t));
    }

    /// Spec 26: at least two members in a path/pins selection.
    pub fn can_merge_selection(&self) -> bool {
        match &self.editor.selection {
            Selection::PinPaths(r) => r.len() >= 2,
            Selection::Pins(r) => r.len() >= 2,
            _ => false,
        }
    }

    pub fn commit_selection_merge(&mut self) {
        if !self.can_merge_selection() || self.selection_locked() {
            return;
        }
        match self.editor.selection.clone() {
            Selection::PinPaths(refs) => {
                let paths = self.selected_pin_paths();
                if paths.len() < 2 {
                    return;
                }
                let (pins, remap) = combine_pin_paths(&paths);
                let dest = refs[0].clone();
                // Freehand through the merged pins keeps later recomputes lossless.
                let geometry = PinPathGeometry::Freehand {
                    points: pins.iter().map(Pin::point).collect(),
                };
                self.commit(|d| {
                    if let Some(p) = find_layer_mut(&mut d.pin_layers, &dest.layer_id)
                        .and_then(|l| l.path_mut(&dest.path_id))
                    {
                        p.geometry = geometry;
                        p.pins = pins;
                    }
                    for r in &refs[1..] {
                        if let Some(l) = find_layer_mut(&mut d.pin_layers, &r.layer_id) {
                            l.pin_paths.retain(|p| p.id != r.path_id);
                        }
                    }
                    layers::remap_pins_in_all_threads_by_map(&mut d.thread_layers, &remap);
                });
                self.editor.selection = Selection::PinPaths(vec![dest]);
            }
            Selection::Pins(refs) => {
                let pts = self.selected_pins();
                if pts.len() < 2 {
                    return;
                }
                let pos = pins_centroid(&pts);
                let Some(dest) = resolve_merge_destination(&self.doc.pin_layers, &refs, pos) else {
                    return;
                };
                let new_pin = Pin {
                    id: next_pin_id(),
                    x: pos.x,
                    y: pos.y,
                };
                let old: HashSet<String> = refs.iter().map(|r| r.pin_id.clone()).collect();
                let new_id = new_pin.id.clone();
                self.commit(|d| {
                    layers::merge_pins(
                        &mut d.pin_layers,
                        &old,
                        &dest.layer_id,
                        &dest.path_id,
                        new_pin,
                    );
                    layers::remap_pins_in_all_threads(&mut d.thread_layers, &old, &new_id);
                });
                self.editor.selection = Selection::Pins(vec![PinRef {
                    layer_id: dest.layer_id,
                    path_id: dest.path_id,
                    pin_id: new_id,
                }]);
            }
            _ => {}
        }
    }

    /// Copy (spec 39): the selection as a self-contained clipboard payload.
    pub fn copy_selection(&self) -> Option<crate::clipboard::ClipboardPayload> {
        use crate::clipboard::*;
        let content = match &self.editor.selection {
            Selection::ThreadPath { .. } => {
                collect_from_thread(&self.doc.pin_layers, self.selected_thread_path()?)
            }
            Selection::PinPaths(_) => collect_from_pin_paths(
                &self.doc.pin_layers,
                &self.doc.thread_layers,
                &self.selected_pin_paths(),
            ),
            Selection::Pins(refs) => {
                let mut seen = HashSet::new();
                let paths: Vec<&PinPath> = refs
                    .iter()
                    .filter(|r| seen.insert(r.path_id.clone()))
                    .filter_map(|r| find_layer(&self.doc.pin_layers, &r.layer_id)?.path(&r.path_id))
                    .collect();
                collect_from_pin_paths(&self.doc.pin_layers, &self.doc.thread_layers, &paths)
            }
            Selection::None => return None,
        };
        (!content.pin_paths.is_empty()).then(|| create_payload(content, &self.board_instance_id))
    }

    /// Paste into the active layers as one step; all-or-nothing on locks.
    pub fn paste_clipboard(&mut self, payload: &crate::clipboard::ClipboardPayload) -> bool {
        use crate::clipboard::*;
        let (pl, tl) = (
            self.editor.active_pin_layer_id.clone(),
            self.editor.active_thread_layer_id.clone(),
        );
        if is_layer_locked(&self.doc.pin_layers, &pl)
            || (!payload.thread_paths.is_empty() && is_layer_locked(&self.doc.thread_layers, &tl))
        {
            return false;
        }
        let index = match &self.last_paste {
            Some((id, n)) if *id == payload.copy_id => n + 1,
            _ => 1,
        };
        self.last_paste = Some((payload.copy_id.clone(), index));
        let content = payload.content();
        let delta = paste_delta(
            &content,
            &self.doc.board,
            index,
            payload.source_board_id == self.board_instance_id,
        );
        let pasted = instantiate(&content, delta);
        let (paths, threads) = (pasted.pin_paths.clone(), pasted.thread_paths.clone());
        self.commit(|d| {
            if let Some(l) = find_layer_mut(&mut d.pin_layers, &pl) {
                l.pin_paths.extend(paths);
            }
            if let Some(l) = find_layer_mut(&mut d.thread_layers, &tl) {
                l.thread_paths.extend(threads);
            }
        });
        use crate::editor::EditorMode;
        self.editor.selection = match self.editor.mode {
            EditorMode::Select if self.editor.select_granularity == SelectGranularity::Path => {
                Selection::PinPaths(
                    pasted
                        .pin_paths
                        .iter()
                        .map(|p| PinPathRef {
                            layer_id: pl.clone(),
                            path_id: p.id.clone(),
                        })
                        .collect(),
                )
            }
            EditorMode::Thread if pasted.thread_paths.len() == 1 => Selection::ThreadPath {
                layer_id: tl,
                path_id: pasted.thread_paths[0].id.clone(),
            },
            _ => Selection::None,
        };
        true
    }

    /// Delete key (spec 34): removes the selection as one step, cascading pins
    /// into thread segments. All-or-nothing on locked layers.
    pub fn delete_selection(&mut self) {
        match self.editor.selection.clone() {
            Selection::ThreadPath { layer_id, path_id } => {
                if is_layer_locked(&self.doc.thread_layers, &layer_id) {
                    return;
                }
                self.commit(|d| {
                    if let Some(l) = find_layer_mut(&mut d.thread_layers, &layer_id) {
                        l.thread_paths.retain(|t| t.id != path_id);
                    }
                });
            }
            Selection::PinPaths(refs) if !refs.is_empty() => {
                if self.selection_locked() {
                    return;
                }
                let pin_ids: Vec<String> = self
                    .selected_pin_paths()
                    .iter()
                    .flat_map(|p| p.pins.iter().map(|q| q.id.clone()))
                    .collect();
                self.commit(|d| {
                    for r in &refs {
                        if let Some(l) = find_layer_mut(&mut d.pin_layers, &r.layer_id) {
                            l.pin_paths.retain(|p| p.id != r.path_id);
                        }
                    }
                    for id in &pin_ids {
                        layers::remove_pin_from_all_threads(&mut d.thread_layers, id);
                    }
                });
            }
            Selection::Pins(refs) if !refs.is_empty() => {
                if self.selection_locked() {
                    return;
                }
                self.commit(|d| {
                    for r in &refs {
                        layers::erase_pin(&mut d.pin_layers, &r.layer_id, &r.path_id, &r.pin_id);
                        layers::remove_pin_from_all_threads(&mut d.thread_layers, &r.pin_id);
                    }
                });
            }
            _ => return,
        }
        self.editor.selection = Selection::None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::ThreadPath;
    use crate::editor::EditorMode;

    fn store_with_two_paths() -> (Store, String, String, String) {
        let mut s = Store::default();
        let layer = s.editor.active_pin_layer_id.clone();
        let a = s
            .add_pin_path(
                &layer,
                PinPathGeometry::Line {
                    start: Point::new(0.0, 0.0),
                    end: Point::new(4.0, 0.0),
                },
            )
            .unwrap();
        let b = s
            .add_pin_path(
                &layer,
                PinPathGeometry::Line {
                    start: Point::new(4.0, 0.0),
                    end: Point::new(4.0, 4.0),
                },
            )
            .unwrap();
        s.set_mode(EditorMode::Select);
        (s, layer, a, b)
    }

    fn refs(layer: &str, ids: &[&str]) -> Selection {
        Selection::PinPaths(
            ids.iter()
                .map(|id| PinPathRef {
                    layer_id: layer.into(),
                    path_id: (*id).into(),
                })
                .collect(),
        )
    }

    #[test]
    fn move_keeps_ids_and_commits_once() {
        let (mut s, layer, a, _) = store_with_two_paths();
        s.select(refs(&layer, &[&a]));
        let ids: Vec<String> = s
            .find_pin_path(&layer, &a)
            .unwrap()
            .pins
            .iter()
            .map(|p| p.id.clone())
            .collect();
        let session = s.begin_transform().unwrap();
        s.preview_transform(&session, Transform::Move(Point::new(1.0, 0.0)));
        s.preview_transform(&session, Transform::Move(Point::new(2.0, 1.0)));
        s.commit_transform(session, Transform::Move(Point::new(2.0, 1.0)));
        let p = s.find_pin_path(&layer, &a).unwrap();
        assert_eq!(p.pins.iter().map(|p| p.id.clone()).collect::<Vec<_>>(), ids);
        assert_eq!(p.pins[0].point(), Point::new(2.0, 1.0));
        assert_eq!(
            p.geometry,
            PinPathGeometry::Line {
                start: Point::new(2.0, 1.0),
                end: Point::new(6.0, 1.0)
            }
        );
        s.undo();
        assert_eq!(
            s.find_pin_path(&layer, &a).unwrap().pins[0].point(),
            Point::ORIGIN
        );
    }

    #[test]
    fn abort_restores() {
        let (mut s, layer, a, _) = store_with_two_paths();
        s.select(refs(&layer, &[&a]));
        let session = s.begin_transform().unwrap();
        s.preview_transform(
            &session,
            Transform::Rotate {
                pivot: Point::ORIGIN,
                theta: 1.0,
            },
        );
        s.abort_transform(session);
        assert_eq!(
            s.find_pin_path(&layer, &a).unwrap().pins[4].point(),
            Point::new(4.0, 0.0)
        );
        let _ = s.can_undo();
    }

    #[test]
    fn scale_reattaches_threads() {
        let (mut s, layer, a, _) = store_with_two_paths();
        let pins: Vec<String> = s
            .find_pin_path(&layer, &a)
            .unwrap()
            .pins
            .iter()
            .map(|p| p.id.clone())
            .collect();
        s.commit(|d| {
            d.thread_layers[0].thread_paths.push(ThreadPath::new(
                vec![pins[0].clone(), pins[4].clone()],
                vec!["#f00".into()],
                1.0,
                6.0,
            ))
        });
        s.select(refs(&layer, &[&a]));
        let session = s.begin_transform().unwrap();
        let pivot = s.selection_pivot().unwrap();
        s.commit_transform(session, Transform::Scale { pivot, factor: 2.0 });
        let new_ids: Vec<String> = s
            .find_pin_path(&layer, &a)
            .unwrap()
            .pins
            .iter()
            .map(|p| p.id.clone())
            .collect();
        assert_eq!(new_ids.len(), 9);
        assert!(
            s.doc.thread_layers[0].thread_paths[0]
                .pin_ids
                .iter()
                .all(|id| new_ids.contains(id))
        );
    }

    #[test]
    fn path_merge_collapses_shared_corner() {
        let (mut s, layer, a, b) = store_with_two_paths();
        s.select(refs(&layer, &[&a, &b]));
        assert!(s.can_merge_selection());
        s.commit_selection_merge();
        let paths = &s.doc.pin_layers[0].pin_paths;
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].pins.len(), 9); // 5 + 5 − 1 shared corner
        assert!(matches!(
            paths[0].geometry,
            PinPathGeometry::Freehand { .. }
        ));
    }

    #[test]
    fn pins_merge_and_delete() {
        let (mut s, layer, a, _) = store_with_two_paths();
        s.set_select_granularity(SelectGranularity::Pins);
        let pins = s.find_pin_path(&layer, &a).unwrap().pins.clone();
        s.select(Selection::Pins(
            pins[0..2]
                .iter()
                .map(|p| PinRef {
                    layer_id: layer.clone(),
                    path_id: a.clone(),
                    pin_id: p.id.clone(),
                })
                .collect(),
        ));
        s.commit_selection_merge();
        let p = s.find_pin_path(&layer, &a).unwrap();
        assert_eq!(p.pins.len(), 4);
        assert_eq!(p.pins.last().unwrap().point(), Point::new(0.5, 0.0));
        s.delete_selection();
        assert_eq!(s.find_pin_path(&layer, &a).unwrap().pins.len(), 3);
        assert_eq!(s.editor.selection, Selection::None);
    }

    #[test]
    fn copy_paste_cascades() {
        let (mut s, layer, a, _) = store_with_two_paths();
        s.select(refs(&layer, &[&a]));
        let payload = s.copy_selection().unwrap();
        assert!(s.paste_clipboard(&payload));
        assert!(s.paste_clipboard(&payload));
        let paths = &s.doc.pin_layers[0].pin_paths;
        assert_eq!(paths.len(), 4);
        assert_eq!(paths[3].pins[0].point(), Point::new(2.0, 2.0));
        assert!(
            matches!(&s.editor.selection, Selection::PinPaths(r) if r[0].path_id == paths[3].id)
        );
    }

    #[test]
    fn keyboard_nudges_coalesce() {
        let (mut s, layer, a, _) = store_with_two_paths();
        s.select(refs(&layer, &[&a]));
        for _ in 0..3 {
            s.apply_transform("kbd", Transform::Move(Point::new(1.0, 0.0)));
        }
        s.end_coalescing();
        assert_eq!(
            s.find_pin_path(&layer, &a).unwrap().pins[0].point(),
            Point::new(3.0, 0.0)
        );
        s.undo();
        assert_eq!(
            s.find_pin_path(&layer, &a).unwrap().pins[0].point(),
            Point::ORIGIN
        );
    }

    #[test]
    fn merge_destination_rule() {
        let (s, layer, a, b) = store_with_two_paths();
        let pa = s.find_pin_path(&layer, &a).unwrap().pins.clone();
        let pb = s.find_pin_path(&layer, &b).unwrap().pins.clone();
        let r = |path: &str, pin: &Pin| PinRef {
            layer_id: layer.clone(),
            path_id: path.into(),
            pin_id: pin.id.clone(),
        };
        let refs = vec![r(&a, &pa[0]), r(&b, &pb[1]), r(&b, &pb[2])];
        assert_eq!(
            resolve_merge_destination(&s.doc.pin_layers, &refs, Point::ORIGIN)
                .unwrap()
                .path_id,
            b
        );
        let refs = vec![r(&a, &pa[0]), r(&b, &pb[1])];
        // tie on count → closer mean to (0,0) wins → a
        assert_eq!(
            resolve_merge_destination(&s.doc.pin_layers, &refs, Point::ORIGIN)
                .unwrap()
                .path_id,
            a
        );
    }
}
