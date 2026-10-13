//! Thread mode store operations (specs 11, 12, 22, 27, 35, 37, 38). Port of
//! the thread half of web `EditorStore.ts`.

use crate::document::ThreadPath;
use crate::document::layers::{self, find_layer, find_layer_mut, is_layer_locked};
use crate::editor::{Selection, Store, ThreadTool, TwoPinDraft};
use crate::threads::{
    TwoPinFillSettings, TwoPinTool, build_repeat_thread_pins, compute_cross_path_candidates,
    compute_next_pattern_pin_id, compute_radial_sequence, compute_repeat_groups,
    compute_same_path_candidates, max_repeat_colours,
};

/// Partial thread-property edit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThreadPropertyPatch {
    pub colours: Option<Vec<String>>,
    pub width: Option<f64>,
    pub twist_pitch: Option<f64>,
}

impl Store {
    pub fn set_thread_tool(&mut self, tool: ThreadTool) {
        let e = &mut self.editor;
        if matches!(e.selection, Selection::ThreadPath { .. }) {
            e.selection = Selection::None;
        }
        if e.two_pin_draft.as_ref().is_some_and(|d| d.tool != tool) {
            e.two_pin_draft = None;
        }
        if tool != ThreadTool::Repeat {
            e.repeat_draft = None;
        }
        e.thread_tool = tool;
    }

    pub fn selected_thread_path(&self) -> Option<&ThreadPath> {
        let Selection::ThreadPath { layer_id, path_id } = &self.editor.selection else {
            return None;
        };
        find_layer(&self.doc.thread_layers, layer_id)?.path(path_id)
    }

    /// Spec 27: edits the selected Thread Path, else the next-draw defaults.
    pub fn set_thread_property(&mut self, key: &str, patch: ThreadPropertyPatch) {
        if let Selection::ThreadPath { layer_id, path_id } = self.editor.selection.clone() {
            if is_layer_locked(&self.doc.thread_layers, &layer_id) {
                return;
            }
            self.commit_coalesced(key, |d| {
                if let Some(t) = find_layer_mut(&mut d.thread_layers, &layer_id)
                    .and_then(|l| l.path_mut(&path_id))
                {
                    if let Some(c) = patch.colours {
                        t.colours = c;
                    }
                    if let Some(w) = patch.width {
                        t.width = w;
                    }
                    if let Some(p) = patch.twist_pitch {
                        t.twist_pitch = p;
                    }
                }
            });
            return;
        }
        let d = &mut self.editor.thread_defaults;
        if let Some(c) = patch.colours {
            d.colours = c;
        }
        if let Some(w) = patch.width {
            d.width = w;
        }
        if let Some(p) = patch.twist_pitch {
            d.twist_pitch = p;
        }
    }

    fn new_thread(&self, pin_ids: Vec<String>) -> ThreadPath {
        let d = &self.editor.thread_defaults;
        ThreadPath::new(pin_ids, d.colours.clone(), d.width, d.twist_pitch)
    }

    /// Adds every path with ≥ 2 pins as one step; a locked layer discards all.
    fn commit_thread_paths(&mut self, layer_id: &str, paths: Vec<ThreadPath>) {
        let valid: Vec<ThreadPath> = paths.into_iter().filter(|p| p.pin_ids.len() >= 2).collect();
        if valid.is_empty() || is_layer_locked(&self.doc.thread_layers, layer_id) {
            return;
        }
        self.commit(|d| {
            if let Some(l) = find_layer_mut(&mut d.thread_layers, layer_id) {
                l.thread_paths.extend(valid);
            }
        });
    }

    // ---- free-draw thread draft (spec 12)

    pub fn extend_thread_draft(&mut self, pin_id: &str) {
        self.editor
            .thread_draft
            .get_or_insert_with(Vec::new)
            .push(pin_id.to_owned());
    }

    fn commit_thread_draft(&mut self, layer_id: &str) {
        if let Some(ids) = self.editor.thread_draft.take() {
            let t = self.new_thread(ids);
            self.commit_thread_paths(layer_id, vec![t]);
        }
    }

    /// Double-click: add the final segment, then finish.
    pub fn finish_thread_draft_with_segment(&mut self, layer_id: &str, pin_id: &str) {
        self.extend_thread_draft(pin_id);
        self.commit_thread_draft(layer_id);
    }

    /// Cut: finish at the last confirmed pin.
    pub fn finish_thread_draft(&mut self, layer_id: &str) {
        self.commit_thread_draft(layer_id);
    }

    /// Esc: 0 segments cancels; ≥ 1 segment finishes as-is.
    pub fn escape_thread_draft(&mut self, layer_id: &str) {
        if self
            .editor
            .thread_draft
            .as_ref()
            .is_some_and(|d| d.len() >= 2)
        {
            self.commit_thread_draft(layer_id);
        } else {
            self.editor.thread_draft = None;
        }
    }

    pub fn cancel_thread_draft(&mut self) {
        self.editor.thread_draft = None;
    }

    pub fn retract_thread_draft(&mut self) {
        if let Some(d) = &mut self.editor.thread_draft {
            d.pop();
            if d.is_empty() {
                self.editor.thread_draft = None;
            }
        }
    }

    /// Right Arrow (spec 22).
    pub fn advance_thread_draft_by_pattern(&mut self) {
        let Some(d) = &self.editor.thread_draft else {
            return;
        };
        if let Some(next) = compute_next_pattern_pin_id(&self.doc.pin_layers, d) {
            self.extend_thread_draft(&next);
        }
    }

    // ---- two-pin tools (spec 35/37)

    pub fn start_two_pin_draft(&mut self, tool: ThreadTool, pin_id: &str) {
        self.editor.two_pin_draft = Some(TwoPinDraft {
            tool,
            first_pin_id: pin_id.to_owned(),
            candidates: Vec::new(),
            chosen_index: 0,
        });
    }

    pub fn choose_second_pin(&mut self, layer_id: &str, pin_id: &str) {
        let Some(draft) = self.editor.two_pin_draft.clone() else {
            return;
        };
        if pin_id == draft.first_pin_id {
            return;
        }
        if draft.tool == ThreadTool::Radial {
            let seq = compute_radial_sequence(
                &self.doc.pin_layers,
                &draft.first_pin_id,
                pin_id,
                self.editor.radial_step,
            );
            self.commit_two_pin_sequences(layer_id, vec![seq]);
            return;
        }
        let (tool, st) = if draft.tool == ThreadTool::Zigzag {
            (TwoPinTool::Zigzag, self.editor.zigzag_settings)
        } else {
            (TwoPinTool::Parabolic, self.editor.parabolic_settings)
        };
        let settings = TwoPinFillSettings {
            step_a: st.step_a,
            step_b: st.step_b,
            full_fill: st.full_fill,
            cycles: (draft.tool == ThreadTool::Parabolic).then_some(st.cycles),
        };
        let layers = &self.doc.pin_layers;
        let mut candidates =
            compute_same_path_candidates(layers, &draft.first_pin_id, pin_id, tool, &settings);
        if candidates.is_empty() {
            candidates =
                compute_cross_path_candidates(layers, &draft.first_pin_id, pin_id, tool, &settings);
        }
        match candidates.len() {
            0 => {}
            1 => {
                let c = candidates.remove(0);
                let mut all = vec![c.sequence];
                all.extend(c.extra_sequences);
                self.commit_two_pin_sequences(layer_id, all);
            }
            _ => {
                self.editor.two_pin_draft = Some(TwoPinDraft {
                    candidates: candidates.into_iter().map(|c| c.sequence).collect(),
                    chosen_index: 0,
                    ..draft
                });
            }
        }
    }

    pub fn set_two_pin_chosen_candidate(&mut self, index: usize) {
        if let Some(d) = &mut self.editor.two_pin_draft
            && index < d.candidates.len()
        {
            d.chosen_index = index;
        }
    }

    /// Third click / Cut: commit the previewed candidate.
    pub fn resolve_two_pin_draft(&mut self, layer_id: &str) {
        let Some(d) = &self.editor.two_pin_draft else {
            return;
        };
        if d.candidates.is_empty() {
            return;
        }
        let seq = d.candidates[d.chosen_index].clone();
        self.commit_two_pin_sequences(layer_id, vec![seq]);
    }

    fn commit_two_pin_sequences(&mut self, layer_id: &str, seqs: Vec<Vec<String>>) {
        self.editor.two_pin_draft = None;
        let paths = seqs.into_iter().map(|s| self.new_thread(s)).collect();
        self.commit_thread_paths(layer_id, paths);
    }

    pub fn cancel_two_pin_draft(&mut self) {
        self.editor.two_pin_draft = None;
    }

    /// Back: with candidates, step back to awaiting the second pin; else cancel.
    pub fn retract_two_pin_draft(&mut self) {
        match &mut self.editor.two_pin_draft {
            Some(d) if !d.candidates.is_empty() => {
                d.candidates.clear();
                d.chosen_index = 0;
            }
            _ => self.editor.two_pin_draft = None,
        }
    }

    // ---- repeat tool (spec 38)

    pub fn extend_repeat_draft(&mut self, pin_id: &str) {
        let d = self.editor.repeat_draft.get_or_insert_with(Vec::new);
        if d.last().map(String::as_str) != Some(pin_id) {
            d.push(pin_id.to_owned());
        }
    }

    pub fn retract_repeat_draft(&mut self) {
        if let Some(d) = &mut self.editor.repeat_draft {
            d.pop();
            if d.is_empty() {
                self.editor.repeat_draft = None;
            }
        }
    }

    pub fn cancel_repeat_draft(&mut self) {
        self.editor.repeat_draft = None;
    }

    /// Enter / Generate: no-op until enough pins are picked.
    pub fn generate_repeat_draft(&mut self, layer_id: &str) {
        let Some(draft) = self.editor.repeat_draft.clone() else {
            return;
        };
        let rs = self.editor.repeat_settings.clone();
        let cycles = rs.cycles.floor().max(0.0) as usize;
        let Some(groups) =
            compute_repeat_groups(&self.doc.pin_layers, &draft, cycles, rs.full_fill)
        else {
            return;
        };
        self.editor.repeat_draft = None;
        let used: Vec<String> = rs
            .colours
            .into_iter()
            .take(max_repeat_colours(cycles))
            .collect();
        let used = if used.is_empty() {
            self.editor.thread_defaults.colours.clone()
        } else {
            used
        };
        let (w, tp) = (
            self.editor.thread_defaults.width,
            self.editor.thread_defaults.twist_pitch,
        );
        let paths = build_repeat_thread_pins(&groups, used.len())
            .into_iter()
            .enumerate()
            .map(|(g, ids)| ThreadPath::new(ids, vec![used[g % used.len()].clone()], w, tp))
            .collect();
        self.commit_thread_paths(layer_id, paths);
    }

    // ---- erasers (spec 11)

    pub fn delete_thread_path(&mut self, layer_id: &str, path_id: &str) {
        if is_layer_locked(&self.doc.thread_layers, layer_id) {
            return;
        }
        self.commit(|d| {
            if let Some(l) = find_layer_mut(&mut d.thread_layers, layer_id) {
                l.thread_paths.retain(|t| t.id != path_id);
            }
        });
    }

    pub fn erase_thread_segment(&mut self, layer_id: &str, path_id: &str, segment: usize) {
        if is_layer_locked(&self.doc.thread_layers, layer_id) {
            return;
        }
        self.commit(|d| {
            layers::split_thread_in_layer(&mut d.thread_layers, layer_id, path_id, segment)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::PinPathGeometry;
    use crate::editor::EditorMode;
    use sa_domain::Point;

    fn store_with_line(n: f64) -> (Store, Vec<String>) {
        let mut s = Store::default();
        let layer = s.editor.active_pin_layer_id.clone();
        let id = s
            .add_pin_path(
                &layer,
                PinPathGeometry::Line {
                    start: Point::ORIGIN,
                    end: Point::new(n, 0.0),
                },
            )
            .unwrap();
        let pins = s
            .find_pin_path(&layer, &id)
            .unwrap()
            .pins
            .iter()
            .map(|p| p.id.clone())
            .collect();
        s.set_mode(EditorMode::Thread);
        (s, pins)
    }

    #[test]
    fn draw_and_finish() {
        let (mut s, pins) = store_with_line(9.0);
        let tl = s.editor.active_thread_layer_id.clone();
        s.extend_thread_draft(&pins[0]);
        s.escape_thread_draft(&tl);
        assert!(s.doc.thread_layers[0].thread_paths.is_empty());
        s.extend_thread_draft(&pins[0]);
        s.finish_thread_draft_with_segment(&tl, &pins[3]);
        assert_eq!(
            s.doc.thread_layers[0].thread_paths[0].pin_ids,
            vec![pins[0].clone(), pins[3].clone()]
        );
        assert_eq!(
            s.doc.thread_layers[0].thread_paths[0].colours,
            vec!["#5b8def".to_owned()]
        );
    }

    #[test]
    fn pattern_advance() {
        let (mut s, pins) = store_with_line(19.0);
        for i in [2, 0, 5, 1] {
            s.extend_thread_draft(&pins[i]);
        }
        s.advance_thread_draft_by_pattern();
        assert_eq!(
            s.editor.thread_draft.as_ref().unwrap().last().unwrap(),
            &pins[8]
        );
    }

    #[test]
    fn zigzag_open_commits_immediately() {
        let (mut s, pins) = store_with_line(9.0);
        let tl = s.editor.active_thread_layer_id.clone();
        s.set_thread_tool(ThreadTool::Zigzag);
        s.start_two_pin_draft(ThreadTool::Zigzag, &pins[0]);
        s.choose_second_pin(&tl, &pins[9]);
        assert!(s.editor.two_pin_draft.is_none());
        assert_eq!(s.doc.thread_layers[0].thread_paths[0].pin_ids.len(), 10);
    }

    #[test]
    fn repeat_generates_coloured_groups() {
        let (mut s, pins) = store_with_line(19.0);
        let tl = s.editor.active_thread_layer_id.clone();
        s.set_thread_tool(ThreadTool::Repeat);
        s.editor.repeat_settings.colours = vec!["#f00".into(), "#0f0".into()];
        for i in [0, 1, 3, 4] {
            s.extend_repeat_draft(&pins[i]);
        }
        s.extend_repeat_draft(&pins[4]); // duplicate last ignored
        assert_eq!(s.editor.repeat_draft.as_ref().unwrap().len(), 4);
        s.generate_repeat_draft(&tl);
        let paths = &s.doc.thread_layers[0].thread_paths;
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[1].colours, vec!["#0f0".to_owned()]);
        assert_eq!(paths[2].colours, vec!["#f00".to_owned()]);
    }

    #[test]
    fn segment_eraser_and_property_edit() {
        let (mut s, pins) = store_with_line(9.0);
        let tl = s.editor.active_thread_layer_id.clone();
        for p in &pins[..4] {
            s.extend_thread_draft(p);
        }
        s.finish_thread_draft(&tl);
        let id = s.doc.thread_layers[0].thread_paths[0].id.clone();
        s.select(Selection::ThreadPath {
            layer_id: tl.clone(),
            path_id: id.clone(),
        });
        s.set_thread_property(
            "w",
            ThreadPropertyPatch {
                width: Some(3.0),
                ..Default::default()
            },
        );
        assert_eq!(s.selected_thread_path().unwrap().width, 3.0);
        s.erase_thread_segment(&tl, &id, 1);
        assert_eq!(s.doc.thread_layers[0].thread_paths.len(), 2);
    }
}
