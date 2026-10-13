//! Store operations for Pin mode (specs 06, 08, 09, 11, 21, 33). Port of the
//! pin-related half of web `EditorStore.ts`.

use sa_domain::Point;

use crate::document::layers::{self, find_layer, find_layer_mut, is_layer_locked};
use crate::document::symmetry::build_nearest_pin_remap;
use crate::document::{PinPath, PinPathGeometry, SymmetryConfig};
use crate::editor::{EditorMode, PinPathRef, Selection, Store};

/// Partial pin-property edit (spec 08 properties panel).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PinPropertyPatch {
    pub spacing: Option<f64>,
    pub colour: Option<String>,
    pub diameter: Option<f64>,
    pub guide_visible: Option<bool>,
}

impl Store {
    /// The one selected Pin Path, when exactly one is selected.
    pub fn selected_pin_path(&self) -> Option<(&str, &PinPath)> {
        let Selection::PinPaths(refs) = &self.editor.selection else {
            return None;
        };
        let [r] = refs.as_slice() else { return None };
        let path = find_layer(&self.doc.pin_layers, &r.layer_id)?.path(&r.path_id)?;
        Some((&r.layer_id, path))
    }

    pub fn find_pin_path(&self, layer_id: &str, path_id: &str) -> Option<&PinPath> {
        find_layer(&self.doc.pin_layers, layer_id)?.path(path_id)
    }

    /// Adds a Pin Path built from the current defaults. `None` on a locked layer.
    pub fn add_pin_path(&mut self, layer_id: &str, geometry: PinPathGeometry) -> Option<String> {
        if is_layer_locked(&self.doc.pin_layers, layer_id) {
            return None;
        }
        let e = &self.editor;
        let path = PinPath::new(
            geometry,
            e.pin_defaults.spacing,
            &e.pin_defaults.style(),
            e.symmetry_defaults.clone(),
        );
        let id = path.id.clone();
        let changed = self.commit(|d| {
            if let Some(l) = find_layer_mut(&mut d.pin_layers, layer_id) {
                l.pin_paths.push(path);
            }
        });
        changed.then_some(id)
    }

    /// Hands a path to Edit mode with it selected (Text tool placement).
    pub fn edit_pin_path(&mut self, layer_id: &str, path_id: &str) {
        self.set_mode(EditorMode::Select);
        self.editor.selection = Selection::PinPaths(vec![PinPathRef {
            layer_id: layer_id.into(),
            path_id: path_id.into(),
        }]);
    }

    // ---- Path tool draft (spec 33)

    pub fn extend_polygon_draft(&mut self, p: Point) {
        self.editor
            .polygon_draft
            .get_or_insert_with(Vec::new)
            .push(p);
    }

    /// Removes the last vertex; removing the only one cancels the draft.
    pub fn retract_polygon_draft(&mut self) {
        if let Some(d) = &mut self.editor.polygon_draft {
            d.pop();
            if d.is_empty() {
                self.editor.polygon_draft = None;
            }
        }
    }

    /// Cut / Esc / click-on-first-vertex: commit if ≥ 3 vertices, else discard.
    pub fn finish_polygon_draft(&mut self, layer_id: &str) {
        if let Some(points) = self.editor.polygon_draft.take()
            && points.len() >= 3
        {
            self.add_pin_path(layer_id, PinPathGeometry::Polygon { points });
        }
    }

    pub fn cancel_polygon_draft(&mut self) {
        self.editor.polygon_draft = None;
    }

    // ---- symmetry (spec 06)

    /// With exactly one Pin Path selected edits it (undoable), else the defaults.
    pub fn set_symmetry_config(&mut self, config: SymmetryConfig) {
        if let Some((layer_id, path)) = self.selected_pin_path() {
            let (layer_id, path_id) = (layer_id.to_owned(), path.id.clone());
            if is_layer_locked(&self.doc.pin_layers, &layer_id) {
                return;
            }
            self.commit_coalesced(&format!("symmetry:{path_id}"), |d| {
                if let Some(p) =
                    find_layer_mut(&mut d.pin_layers, &layer_id).and_then(|l| l.path_mut(&path_id))
                {
                    p.symmetry = config;
                }
            });
            return;
        }
        self.editor.symmetry_defaults = config;
    }

    /// The symmetry shown by the panel/overlay: the selected path's or the defaults.
    pub fn active_symmetry(&self) -> &SymmetryConfig {
        self.selected_pin_path()
            .map(|(_, p)| &p.symmetry)
            .unwrap_or(&self.editor.symmetry_defaults)
    }

    // ---- path edits

    pub fn delete_pin_path(&mut self, layer_id: &str, path_id: &str) {
        if is_layer_locked(&self.doc.pin_layers, layer_id) {
            return;
        }
        self.commit(|d| {
            if let Some(l) = find_layer_mut(&mut d.pin_layers, layer_id) {
                l.pin_paths.retain(|p| p.id != path_id);
            }
        });
        self.drop_path_from_selection(path_id);
    }

    fn drop_path_from_selection(&mut self, path_id: &str) {
        if let Selection::PinPaths(refs) = &self.editor.selection
            && refs.iter().any(|r| r.path_id == path_id)
        {
            self.editor.selection = Selection::None;
        }
    }

    /// Geometry edit from the Selection panel: recomputes pins (fresh ids).
    pub fn update_pin_path_geometry(
        &mut self,
        key: &str,
        layer_id: &str,
        path_id: &str,
        geometry: PinPathGeometry,
    ) {
        if is_layer_locked(&self.doc.pin_layers, layer_id) {
            return;
        }
        self.commit_coalesced(key, |d| {
            if let Some(p) =
                find_layer_mut(&mut d.pin_layers, layer_id).and_then(|l| l.path_mut(path_id))
            {
                p.geometry = geometry;
                *p = p.recomputed();
            }
        });
    }

    /// Spec 08/11: edits the single selected path, else the next-draw defaults.
    /// A spacing change recomputes pins and reattaches threads in the same step.
    pub fn set_pin_property(&mut self, key: &str, patch: PinPropertyPatch) {
        let Some((layer_id, path)) = self.selected_pin_path() else {
            let d = &mut self.editor.pin_defaults;
            if let Some(v) = patch.spacing {
                d.spacing = v;
            }
            if let Some(v) = patch.colour {
                d.colour = v;
            }
            if let Some(v) = patch.diameter {
                d.diameter = v;
            }
            if let Some(v) = patch.guide_visible {
                d.guide_visible = v;
            }
            return;
        };
        let (layer_id, old) = (layer_id.to_owned(), path.clone());
        if is_layer_locked(&self.doc.pin_layers, &layer_id) {
            return;
        }
        let mut next = old.clone();
        if let Some(v) = patch.spacing {
            next.requested_spacing = v;
        }
        if let Some(v) = &patch.colour {
            next.colour = v.clone();
        }
        if let Some(v) = patch.diameter {
            next.diameter = v;
        }
        if let Some(v) = patch.guide_visible {
            next.guide_visible = v;
        }
        if patch.spacing.is_some() {
            next = next.recomputed();
            self.commit_pin_path_with_reattach(key, &layer_id, &old, next);
            return;
        }
        self.commit_coalesced(key, |d| {
            if let Some(p) =
                find_layer_mut(&mut d.pin_layers, &layer_id).and_then(|l| l.path_mut(&old.id))
            {
                *p = next;
            }
        });
    }

    /// Spec 21 Nearest-Pin Reattachment: replace `old` with `new` and remap every
    /// thread reference to the nearest new pin, as one step.
    pub fn commit_pin_path_with_reattach(
        &mut self,
        key: &str,
        layer_id: &str,
        old: &PinPath,
        new: PinPath,
    ) {
        let remap = build_nearest_pin_remap(old, &new);
        let path_id = old.id.clone();
        self.commit_coalesced(key, |d| {
            if let Some(p) =
                find_layer_mut(&mut d.pin_layers, layer_id).and_then(|l| l.path_mut(&path_id))
            {
                *p = new;
            }
            layers::remap_pins_in_all_threads_by_map(&mut d.thread_layers, &remap);
        });
    }

    /// Spec 11 Pin Eraser: removes a pin + cascades into thread segments.
    pub fn erase_pin(&mut self, layer_id: &str, path_id: &str, pin_id: &str) {
        if is_layer_locked(&self.doc.pin_layers, layer_id) {
            return;
        }
        self.commit(|d| {
            layers::erase_pin(&mut d.pin_layers, layer_id, path_id, pin_id);
            layers::remove_pin_from_all_threads(&mut d.thread_layers, pin_id);
        });
    }

    /// Spec 11 Path Eraser: removes a whole Pin Path + cascades.
    pub fn erase_pin_path(&mut self, layer_id: &str, path_id: &str) {
        if is_layer_locked(&self.doc.pin_layers, layer_id) {
            return;
        }
        let Some(path) = self.find_pin_path(layer_id, path_id) else {
            return;
        };
        let pin_ids: Vec<String> = path.pins.iter().map(|p| p.id.clone()).collect();
        self.commit(|d| {
            if let Some(l) = find_layer_mut(&mut d.pin_layers, layer_id) {
                l.pin_paths.retain(|p| p.id != path_id);
            }
            for id in &pin_ids {
                layers::remove_pin_from_all_threads(&mut d.thread_layers, id);
            }
        });
        self.drop_path_from_selection(path_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::ThreadPath;
    use crate::editor::PinTool;

    fn circle() -> PinPathGeometry {
        PinPathGeometry::Circle {
            center: Point::ORIGIN,
            radius: 5.0,
        }
    }

    #[test]
    fn add_respects_lock_and_defaults() {
        let mut s = Store::default();
        let layer = s.editor.active_pin_layer_id.clone();
        s.editor.pin_defaults.spacing = 2.0;
        let id = s.add_pin_path(&layer, circle()).unwrap();
        let p = s.find_pin_path(&layer, &id).unwrap();
        assert_eq!(p.requested_spacing, 2.0);
        s.toggle_pin_layer_locked(&layer);
        assert!(s.add_pin_path(&layer, circle()).is_none());
    }

    #[test]
    fn polygon_draft_lifecycle() {
        let mut s = Store::default();
        s.set_pin_tool(PinTool::Polygon);
        let layer = s.editor.active_pin_layer_id.clone();
        s.extend_polygon_draft(Point::new(0.0, 0.0));
        s.retract_polygon_draft();
        assert!(s.editor.polygon_draft.is_none());
        for p in [(0.0, 0.0), (4.0, 0.0)] {
            s.extend_polygon_draft(Point::new(p.0, p.1));
        }
        s.finish_polygon_draft(&layer); // < 3 → discarded
        assert!(s.doc.pin_layers[0].pin_paths.is_empty());
        for p in [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)] {
            s.extend_polygon_draft(Point::new(p.0, p.1));
        }
        s.finish_polygon_draft(&layer);
        assert_eq!(s.doc.pin_layers[0].pin_paths.len(), 1);
    }

    #[test]
    fn erase_pin_cascades_in_one_step() {
        let mut s = Store::default();
        let layer = s.editor.active_pin_layer_id.clone();
        let id = s.add_pin_path(&layer, circle()).unwrap();
        let pins: Vec<String> = s
            .find_pin_path(&layer, &id)
            .unwrap()
            .pins
            .iter()
            .map(|p| p.id.clone())
            .collect();
        let tl = s.editor.active_thread_layer_id.clone();
        s.commit(|d| {
            d.thread_layers[0].thread_paths.push(ThreadPath::new(
                pins[0..3].to_vec(),
                vec!["#f00".into()],
                1.0,
                6.0,
            ))
        });
        let _ = tl;
        s.erase_pin(&layer, &id, &pins[1]);
        assert!(s.doc.thread_layers[0].thread_paths.is_empty());
        s.undo();
        assert_eq!(s.doc.thread_layers[0].thread_paths.len(), 1);
        assert_eq!(s.find_pin_path(&layer, &id).unwrap().pins.len(), pins.len());
    }

    #[test]
    fn spacing_change_reattaches_threads() {
        let mut s = Store::default();
        let layer = s.editor.active_pin_layer_id.clone();
        let id = s.add_pin_path(&layer, circle()).unwrap();
        let pins: Vec<String> = s
            .find_pin_path(&layer, &id)
            .unwrap()
            .pins
            .iter()
            .map(|p| p.id.clone())
            .collect();
        s.commit(|d| {
            d.thread_layers[0].thread_paths.push(ThreadPath::new(
                vec![pins[0].clone(), pins[10].clone()],
                vec!["#f00".into()],
                1.0,
                6.0,
            ))
        });
        s.edit_pin_path(&layer, &id);
        s.set_pin_property(
            "spacing",
            PinPropertyPatch {
                spacing: Some(2.0),
                ..Default::default()
            },
        );
        let new_ids: Vec<String> = s
            .find_pin_path(&layer, &id)
            .unwrap()
            .pins
            .iter()
            .map(|p| p.id.clone())
            .collect();
        let t = &s.doc.thread_layers[0].thread_paths[0];
        assert!(
            t.pin_ids.iter().all(|pid| new_ids.contains(pid)),
            "{:?}",
            t.pin_ids
        );
    }

    #[test]
    fn symmetry_defaults_vs_selected() {
        let mut s = Store::default();
        s.set_symmetry_config(SymmetryConfig::Vertical {
            axis: Point::ORIGIN,
        });
        assert_eq!(
            s.editor.symmetry_defaults,
            SymmetryConfig::Vertical {
                axis: Point::ORIGIN
            }
        );
        let layer = s.editor.active_pin_layer_id.clone();
        let id = s.add_pin_path(&layer, circle()).unwrap();
        s.edit_pin_path(&layer, &id);
        s.set_symmetry_config(SymmetryConfig::None);
        assert_eq!(
            s.find_pin_path(&layer, &id).unwrap().symmetry,
            SymmetryConfig::None
        );
    }
}
