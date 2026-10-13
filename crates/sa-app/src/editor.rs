//! Editor state + the document store (spec 01 §Command Architecture). Port of
//! web `EditorState.ts` / `EditorStore.ts`. Every document mutation goes through
//! [`Store::commit`], which snapshots the document so cascades undo as one step.
//! Mode, grid, snap, viewport, tools and drafts are transient, never undoable.

use sa_domain::Point;
use sa_domain::transforms::Viewport;

use crate::document::board::{
    BoardAppearance, BoardDimensions, BoardShape, TriangleType, clamp_dimension,
    default_dimensions_for,
};
use crate::document::layers::{self, Direction, Layer, PinLayer, ThreadLayer};
use crate::document::project::{GridFile, ProjectFile};
use crate::document::{Document, PinStyle, SymmetryConfig};
use crate::history::History;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditorMode {
    Pin,
    /// Labelled "Edit" in the UI.
    Select,
    Thread,
    Generate,
    Pan,
    Play,
}

impl EditorMode {
    /// Tab order (spec 36: Pin first); keys `1`–`6` follow it.
    pub const ORDER: [EditorMode; 6] = [
        EditorMode::Pin,
        EditorMode::Select,
        EditorMode::Thread,
        EditorMode::Generate,
        EditorMode::Pan,
        EditorMode::Play,
    ];

    pub fn key(self) -> &'static str {
        match self {
            EditorMode::Pin => "pin",
            EditorMode::Select => "select",
            EditorMode::Thread => "thread",
            EditorMode::Generate => "generate",
            EditorMode::Pan => "pan",
            EditorMode::Play => "play",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridSettings {
    pub gap_x: f64,
    pub gap_y: f64,
    pub visible: bool,
    pub snap_enabled: bool,
    pub colour: String,
    pub opacity: f64,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            gap_x: 1.0,
            gap_y: 1.0,
            visible: true,
            snap_enabled: true,
            colour: "#6d5ef7".into(),
            opacity: 0.6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapSettings {
    pub pin_snap_enabled: bool,
    pub radius_px: f64,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            pin_snap_enabled: true,
            radius_px: 12.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PinTool {
    Line,
    Arc,
    Ellipse,
    Circle,
    Rectangle,
    Square,
    Pentagon,
    Hexagon,
    Octagon,
    Star5,
    Star6,
    Star8,
    Pentagram,
    Heptagram,
    Octagram,
    Freehand,
    /// The "Path" tool (spec 33).
    Polygon,
    Text,
    Eraser,
    PathEraser,
}

impl PinTool {
    pub fn is_draw_tool(self) -> bool {
        !matches!(self, PinTool::Eraser | PinTool::PathEraser)
    }

    pub fn key(self) -> &'static str {
        use PinTool::*;
        match self {
            Line => "line",
            Arc => "arc",
            Ellipse => "ellipse",
            Circle => "circle",
            Rectangle => "rectangle",
            Square => "square",
            Pentagon => "pentagon",
            Hexagon => "hexagon",
            Octagon => "octagon",
            Star5 => "star-5",
            Star6 => "star-6",
            Star8 => "star-8",
            Pentagram => "pentagram",
            Heptagram => "heptagram",
            Octagram => "octagram",
            Freehand => "freehand",
            Polygon => "polygon",
            Text => "text",
            Eraser => "eraser",
            PathEraser => "path-eraser",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PinPathRef {
    pub layer_id: String,
    pub path_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PinRef {
    pub layer_id: String,
    pub path_id: String,
    pub pin_id: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum Selection {
    #[default]
    None,
    PinPaths(Vec<PinPathRef>),
    Pins(Vec<PinRef>),
    ThreadPath {
        layer_id: String,
        path_id: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectTool {
    Select,
    Move,
    Rotate,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectGranularity {
    Path,
    Pins,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThreadTool {
    Draw,
    Eraser,
    SegmentEraser,
    Select,
    Zigzag,
    Parabolic,
    Radial,
    Repeat,
}

impl ThreadTool {
    pub fn is_two_pin_draft_tool(self) -> bool {
        matches!(
            self,
            ThreadTool::Zigzag | ThreadTool::Parabolic | ThreadTool::Radial
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinDefaults {
    pub spacing: f64,
    pub colour: String,
    /// Millimetres.
    pub diameter: f64,
    pub guide_visible: bool,
}

impl Default for PinDefaults {
    fn default() -> Self {
        Self {
            spacing: 1.0,
            colour: "#f2ede4".into(),
            diameter: 2.0,
            guide_visible: true,
        }
    }
}

impl PinDefaults {
    pub fn style(&self) -> PinStyle {
        PinStyle {
            colour: self.colour.clone(),
            diameter: self.diameter,
            guide_visible: self.guide_visible,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThreadDefaults {
    pub colours: Vec<String>,
    pub width: f64,
    pub twist_pitch: f64,
}

impl Default for ThreadDefaults {
    fn default() -> Self {
        Self {
            colours: vec!["#5b8def".into()],
            width: 1.5,
            twist_pitch: 6.0,
        }
    }
}

/// Zig-zag/Parabolic/Radial draft (spec 35/37): first pin, then 2+ candidate
/// sequences awaiting the disambiguating third click.
#[derive(Clone, Debug, PartialEq)]
pub struct TwoPinDraft {
    pub tool: ThreadTool,
    pub first_pin_id: String,
    pub candidates: Vec<Vec<String>>,
    pub chosen_index: usize,
}

/// Next-draw settings for Zig-zag / Parabolic (spec 35 §Configuration).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoPinSettings {
    pub step_a: f64,
    pub step_b: f64,
    pub full_fill: bool,
    pub cycles: f64,
}

impl Default for TwoPinSettings {
    fn default() -> Self {
        Self {
            step_a: 0.0,
            step_b: 0.0,
            full_fill: false,
            cycles: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepeatSettings {
    pub cycles: f64,
    pub full_fill: bool,
    pub colours: Vec<String>,
}

impl Default for RepeatSettings {
    fn default() -> Self {
        Self {
            cycles: 1.0,
            full_fill: false,
            colours: vec![ThreadDefaults::default().colours[0].clone()],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerPanelTab {
    Pin,
    Thread,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditorState {
    pub mode: EditorMode,
    pub grid: GridSettings,
    pub snap: SnapSettings,
    pub viewport: Viewport,
    pub active_pin_layer_id: String,
    pub active_thread_layer_id: String,
    pub pin_tool: PinTool,
    pub pin_defaults: PinDefaults,
    pub symmetry_defaults: SymmetryConfig,
    pub selection: Selection,
    pub select_tool: SelectTool,
    pub select_granularity: SelectGranularity,
    pub thread_tool: ThreadTool,
    pub thread_defaults: ThreadDefaults,
    pub thread_draft: Option<Vec<String>>,
    pub polygon_draft: Option<Vec<Point>>,
    pub two_pin_draft: Option<TwoPinDraft>,
    pub repeat_draft: Option<Vec<String>>,
    pub zigzag_settings: TwoPinSettings,
    pub parabolic_settings: TwoPinSettings,
    /// Radial tool stride over the target path's pins (1 = every pin).
    pub radial_step: f64,
    pub repeat_settings: RepeatSettings,
    pub layer_panel_tab: LayerPanelTab,
    pub print: crate::print::PrintSettings,
    /// Generate/Re-generate result awaiting Confirm (transient, spec 32).
    pub generator_draft: Option<(
        crate::generator::GeneratorParams,
        crate::generator::BuildResult,
    )>,
}

/// Document + editor state + undo history.
#[derive(Clone, Debug)]
pub struct Store {
    pub doc: Document,
    pub editor: EditorState,
    history: History<Document>,
    /// Key of the coalescing edit in progress (a drag, a colour picker): further
    /// commits with the same key amend the newest undo entry.
    coalesce_key: Option<String>,
    /// Identifies this open board for paste placement (spec 39).
    pub(crate) board_instance_id: String,
    pub(crate) last_paste: Option<(String, usize)>,
    /// Bumped on every document change (edits, undo, redo, load): cache key and
    /// dirty tracking.
    revision: u64,
}

impl Default for Store {
    fn default() -> Self {
        Self::new(Document::empty())
    }
}

impl Store {
    pub fn new(doc: Document) -> Self {
        let editor = EditorState {
            mode: EditorMode::Pin,
            grid: GridSettings::default(),
            snap: SnapSettings::default(),
            viewport: Viewport {
                zoom: 4.0,
                pan_origin: Point::new(-40.0, -40.0),
            },
            active_pin_layer_id: doc
                .pin_layers
                .first()
                .map(|l| l.id.clone())
                .unwrap_or_default(),
            active_thread_layer_id: doc
                .thread_layers
                .first()
                .map(|l| l.id.clone())
                .unwrap_or_default(),
            pin_tool: PinTool::Circle,
            pin_defaults: PinDefaults::default(),
            symmetry_defaults: SymmetryConfig::None,
            selection: Selection::None,
            select_tool: SelectTool::Select,
            select_granularity: SelectGranularity::Path,
            thread_tool: ThreadTool::Draw,
            thread_defaults: ThreadDefaults::default(),
            thread_draft: None,
            polygon_draft: None,
            two_pin_draft: None,
            repeat_draft: None,
            zigzag_settings: TwoPinSettings::default(),
            parabolic_settings: TwoPinSettings::default(),
            radial_step: 1.0,
            repeat_settings: RepeatSettings::default(),
            layer_panel_tab: LayerPanelTab::Pin,
            print: crate::print::PrintSettings::default(),
            generator_draft: None,
        };
        Self {
            doc,
            editor,
            history: History::default(),
            coalesce_key: None,
            board_instance_id: crate::clipboard::instance_token(),
            last_paste: None,
            revision: 0,
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Applies `f` to the document as one undoable step. No-op changes are not
    /// recorded. Returns whether the document changed.
    pub fn commit(&mut self, f: impl FnOnce(&mut Document)) -> bool {
        self.coalesce_key = None;
        let before = self.doc.clone();
        f(&mut self.doc);
        if self.doc == before {
            return false;
        }
        self.history.record(before, self.doc.clone());
        self.revision += 1;
        true
    }

    /// Like [`Store::commit`], but consecutive calls with the same `key` merge
    /// into one undo step until [`Store::end_coalescing`] or another commit.
    pub fn commit_coalesced(&mut self, key: &str, f: impl FnOnce(&mut Document)) -> bool {
        if self.coalesce_key.as_deref() != Some(key) {
            let changed = self.commit(f);
            if changed {
                self.coalesce_key = Some(key.to_owned());
            }
            return changed;
        }
        let before = self.doc.clone();
        f(&mut self.doc);
        if self.doc == before {
            return false;
        }
        self.history.amend_last(self.doc.clone());
        self.revision += 1;
        true
    }

    /// Ends the current coalescing run (call on pointer release / focus loss).
    pub fn end_coalescing(&mut self) {
        self.coalesce_key = None;
    }

    /// Replaces the document without recording history (live previews during a
    /// drag). Pair with [`Store::commit_from`] on release.
    pub fn preview(&mut self, f: impl FnOnce(&mut Document)) {
        f(&mut self.doc);
        self.revision += 1;
    }

    /// Records `before` → current document as one step (after previews).
    pub fn commit_from(&mut self, before: Document) -> bool {
        if self.doc == before {
            return false;
        }
        self.history.record(before, self.doc.clone());
        self.revision += 1;
        true
    }

    pub fn undo(&mut self) {
        self.coalesce_key = None;
        if let Some(doc) = self.history.undo() {
            self.doc = doc;
            self.after_document_replaced();
        }
    }

    pub fn redo(&mut self) {
        self.coalesce_key = None;
        if let Some(doc) = self.history.redo() {
            self.doc = doc;
            self.after_document_replaced();
        }
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    fn after_document_replaced(&mut self) {
        self.revision += 1;
        self.ensure_active_layers();
    }

    fn ensure_active_layers(&mut self) {
        let e = &mut self.editor;
        if layers::find_layer(&self.doc.pin_layers, &e.active_pin_layer_id).is_none()
            && let Some(l) = self.doc.pin_layers.first()
        {
            e.active_pin_layer_id = l.id.clone();
        }
        if layers::find_layer(&self.doc.thread_layers, &e.active_thread_layer_id).is_none()
            && let Some(l) = self.doc.thread_layers.first()
        {
            e.active_thread_layer_id = l.id.clone();
        }
    }

    // ---- board (spec 03/04, undoable)

    pub fn set_board_shape(&mut self, shape: BoardShape, triangle_type: Option<TriangleType>) {
        self.commit(|d| {
            d.board.shape = shape;
            d.board.triangle_type = triangle_type;
            d.board.dimensions = default_dimensions_for(shape, triangle_type);
        });
    }

    pub fn set_triangle_type(&mut self, triangle_type: TriangleType) {
        self.set_board_shape(BoardShape::Triangle, Some(triangle_type));
    }

    pub fn set_board_dimensions(&mut self, patch: BoardDimensions) {
        self.commit(|d| {
            let dims = &mut d.board.dimensions;
            let apply = |slot: &mut Option<f64>, v: Option<f64>| {
                if let Some(v) = v {
                    *slot = Some(clamp_dimension(v));
                }
            };
            apply(&mut dims.diameter, patch.diameter);
            apply(&mut dims.width, patch.width);
            apply(&mut dims.height, patch.height);
            apply(&mut dims.side, patch.side);
            apply(&mut dims.base, patch.base);
        });
    }

    pub fn set_board_appearance(&mut self, appearance: BoardAppearance) {
        self.commit(|d| d.board.appearance = appearance);
    }

    /// Continuous appearance edits (colour pickers, number drags) coalesce.
    pub fn edit_board_appearance(&mut self, key: &str, appearance: BoardAppearance) {
        self.commit_coalesced(key, |d| d.board.appearance = appearance);
    }

    pub fn edit_board_dimensions(&mut self, key: &str, patch: BoardDimensions) {
        self.commit_coalesced(key, |d| {
            let dims = &mut d.board.dimensions;
            for (slot, v) in [
                (&mut dims.diameter, patch.diameter),
                (&mut dims.width, patch.width),
                (&mut dims.height, patch.height),
                (&mut dims.side, patch.side),
                (&mut dims.base, patch.base),
            ] {
                if let Some(v) = v {
                    *slot = Some(clamp_dimension(v));
                }
            }
        });
    }

    // ---- transient state

    /// Spec 05/13: entering Pin/Thread syncs the Layers tab; selection kinds that
    /// don't belong to the new mode are dropped; the entered tab resets its tool
    /// state; drafts belonging to other modes are discarded.
    pub fn set_mode(&mut self, mode: EditorMode) {
        let e = &mut self.editor;
        match mode {
            EditorMode::Pin => e.layer_panel_tab = LayerPanelTab::Pin,
            EditorMode::Thread => e.layer_panel_tab = LayerPanelTab::Thread,
            _ => {}
        }
        if matches!(mode, EditorMode::Pin | EditorMode::Thread)
            && matches!(e.selection, Selection::PinPaths(_) | Selection::Pins(_))
        {
            e.selection = Selection::None;
        }
        if matches!(mode, EditorMode::Pin | EditorMode::Select)
            && matches!(e.selection, Selection::ThreadPath { .. })
        {
            e.selection = Selection::None;
        }
        match mode {
            EditorMode::Pin => {
                e.pin_tool = PinTool::Circle;
                e.pin_defaults = PinDefaults::default();
                e.symmetry_defaults = SymmetryConfig::None;
            }
            EditorMode::Select => {
                e.select_tool = SelectTool::Select;
                e.select_granularity = SelectGranularity::Path;
            }
            EditorMode::Thread => {
                e.thread_tool = ThreadTool::Draw;
                e.thread_defaults = ThreadDefaults::default();
            }
            _ => {}
        }
        if mode != EditorMode::Pin {
            e.polygon_draft = None;
        }
        if mode != EditorMode::Generate {
            e.generator_draft = None;
        }
        if mode != EditorMode::Thread {
            e.thread_draft = None;
            e.two_pin_draft = None;
            e.repeat_draft = None;
        }
        e.mode = mode;
    }

    pub fn set_pin_tool(&mut self, tool: PinTool) {
        let e = &mut self.editor;
        if matches!(e.selection, Selection::PinPaths(_) | Selection::Pins(_)) {
            e.selection = Selection::None;
        }
        if tool.is_draw_tool() {
            e.pin_defaults = PinDefaults::default();
            e.symmetry_defaults = SymmetryConfig::None;
        }
        if tool != PinTool::Polygon {
            e.polygon_draft = None;
        }
        e.pin_tool = tool;
    }

    // ---- layers (spec 13/36)

    pub fn add_pin_layer(&mut self) {
        let layer = PinLayer::new(format!("Layer {}", self.doc.pin_layers.len() + 1));
        let id = layer.id.clone();
        self.commit(|d| d.pin_layers.push(layer));
        self.editor.active_pin_layer_id = id;
    }

    pub fn add_thread_layer(&mut self) {
        let layer = ThreadLayer::new(format!("Layer {}", self.doc.thread_layers.len() + 1));
        let id = layer.id.clone();
        self.commit(|d| d.thread_layers.push(layer));
        self.editor.active_thread_layer_id = id;
    }

    pub fn rename_pin_layer(&mut self, id: &str, name: &str) {
        self.commit(|d| layers::rename_layer(&mut d.pin_layers, id, name));
    }

    pub fn rename_thread_layer(&mut self, id: &str, name: &str) {
        self.commit(|d| layers::rename_layer(&mut d.thread_layers, id, name));
    }

    pub fn toggle_pin_layer_visible(&mut self, id: &str) {
        self.commit(|d| layers::toggle_layer_visible(&mut d.pin_layers, id));
    }

    pub fn toggle_thread_layer_visible(&mut self, id: &str) {
        self.commit(|d| layers::toggle_layer_visible(&mut d.thread_layers, id));
    }

    pub fn toggle_pin_layer_locked(&mut self, id: &str) {
        self.commit(|d| layers::toggle_layer_locked(&mut d.pin_layers, id));
    }

    pub fn toggle_thread_layer_locked(&mut self, id: &str) {
        self.commit(|d| layers::toggle_layer_locked(&mut d.thread_layers, id));
    }

    pub fn reorder_pin_layer(&mut self, id: &str, dir: Direction) {
        self.commit(|d| layers::reorder_layer(&mut d.pin_layers, id, dir));
    }

    pub fn reorder_thread_layer(&mut self, id: &str, dir: Direction) {
        self.commit(|d| layers::reorder_layer(&mut d.thread_layers, id, dir));
    }

    pub fn duplicate_pin_layer(&mut self, id: &str) {
        self.commit(|d| {
            layers::duplicate_layer(&mut d.pin_layers, id);
        });
    }

    pub fn duplicate_thread_layer(&mut self, id: &str) {
        self.commit(|d| {
            layers::duplicate_layer(&mut d.thread_layers, id);
        });
    }

    /// No-op on the topmost layer or when either layer is locked.
    pub fn merge_pin_layer_above(&mut self, id: &str) {
        let Some(i) = self.doc.pin_layers.iter().position(|l| l.id == id) else {
            return;
        };
        if i == 0 || self.doc.pin_layers[i].locked || self.doc.pin_layers[i - 1].locked {
            return;
        }
        let target = self.doc.pin_layers[i - 1].id.clone();
        self.commit(|d| {
            layers::merge_layer_above(&mut d.pin_layers, id);
        });
        if self.editor.active_pin_layer_id == id {
            self.editor.active_pin_layer_id = target;
        }
    }

    pub fn merge_thread_layer_above(&mut self, id: &str) {
        let Some(i) = self.doc.thread_layers.iter().position(|l| l.id == id) else {
            return;
        };
        if i == 0 || self.doc.thread_layers[i].locked || self.doc.thread_layers[i - 1].locked {
            return;
        }
        let target = self.doc.thread_layers[i - 1].id.clone();
        self.commit(|d| {
            layers::merge_layer_above(&mut d.thread_layers, id);
        });
        if self.editor.active_thread_layer_id == id {
            self.editor.active_thread_layer_id = target;
        }
    }

    /// Deleting a pin layer cascades into thread segments using its pins — one step.
    pub fn delete_pin_layer(&mut self, id: &str) {
        let Some(layer) = layers::find_layer(&self.doc.pin_layers, id) else {
            return;
        };
        let pin_ids: Vec<String> = layer
            .pin_paths
            .iter()
            .flat_map(|p| p.pins.iter().map(|q| q.id.clone()))
            .collect();
        self.commit(|d| {
            layers::delete_layer(&mut d.pin_layers, id);
            for pin in &pin_ids {
                layers::remove_pin_from_all_threads(&mut d.thread_layers, pin);
            }
        });
        self.ensure_active_layers();
    }

    pub fn delete_thread_layer(&mut self, id: &str) {
        self.commit(|d| layers::delete_layer(&mut d.thread_layers, id));
        self.ensure_active_layers();
    }

    pub fn is_active_pin_layer_locked(&self) -> bool {
        layers::find_layer(&self.doc.pin_layers, &self.editor.active_pin_layer_id)
            .is_some_and(Layer::is_locked)
    }

    pub fn is_active_thread_layer_locked(&self) -> bool {
        layers::find_layer(&self.doc.thread_layers, &self.editor.active_thread_layer_id)
            .is_some_and(Layer::is_locked)
    }

    // ---- persistence (spec 16)

    pub fn to_project_file(&self) -> ProjectFile {
        let g = &self.editor.grid;
        ProjectFile::from_document(
            &self.doc,
            GridFile {
                gap_x: g.gap_x,
                gap_y: g.gap_y,
                colour: g.colour.clone(),
                opacity: g.opacity,
            },
        )
    }

    /// Loading is not undoable and starts a fresh history.
    pub fn load_project(&mut self, file: ProjectFile) {
        let (doc, grid) = file.into_parts();
        self.history.clear();
        self.board_instance_id = crate::clipboard::instance_token();
        self.last_paste = None;
        self.doc = doc;
        let e = &mut self.editor;
        e.grid.gap_x = grid.gap_x;
        e.grid.gap_y = grid.gap_y;
        e.grid.colour = grid.colour;
        e.grid.opacity = grid.opacity;
        e.active_pin_layer_id = self
            .doc
            .pin_layers
            .first()
            .map(|l| l.id.clone())
            .unwrap_or_default();
        e.active_thread_layer_id = self
            .doc
            .thread_layers
            .first()
            .map(|l| l.id.clone())
            .unwrap_or_default();
        e.selection = Selection::None;
        e.select_tool = SelectTool::Select;
        e.select_granularity = SelectGranularity::Path;
        e.thread_draft = None;
        e.polygon_draft = None;
        e.two_pin_draft = None;
        e.repeat_draft = None;
        e.generator_draft = None;
        self.revision += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_changes_are_undoable_and_clamped() {
        let mut s = Store::default();
        s.set_board_dimensions(BoardDimensions {
            diameter: Some(-5.0),
            ..Default::default()
        });
        assert_eq!(s.doc.board.dimensions.diameter, Some(0.1));
        s.undo();
        assert_eq!(s.doc.board.dimensions.diameter, Some(60.0));
        s.redo();
        assert_eq!(s.doc.board.dimensions.diameter, Some(0.1));
        s.set_triangle_type(TriangleType::RightAngled);
        assert_eq!(s.doc.board.dimensions.base, Some(40.0));
    }

    #[test]
    fn coalesced_edits_make_one_step() {
        let mut s = Store::default();
        for d in [61.0, 62.0, 63.0] {
            s.edit_board_dimensions(
                "dia",
                BoardDimensions {
                    diameter: Some(d),
                    ..Default::default()
                },
            );
        }
        s.end_coalescing();
        s.edit_board_dimensions(
            "dia",
            BoardDimensions {
                diameter: Some(70.0),
                ..Default::default()
            },
        );
        s.undo();
        assert_eq!(s.doc.board.dimensions.diameter, Some(63.0));
        s.undo();
        assert_eq!(s.doc.board.dimensions.diameter, Some(60.0));
        assert!(!s.can_undo());
    }

    #[test]
    fn no_op_changes_are_not_recorded() {
        let mut s = Store::default();
        s.set_board_appearance(s.doc.board.appearance.clone());
        assert!(!s.can_undo());
    }

    #[test]
    fn mode_switch_syncs_tab_and_resets_tools() {
        let mut s = Store::default();
        s.editor.pin_tool = PinTool::Line;
        s.set_mode(EditorMode::Thread);
        assert_eq!(s.editor.layer_panel_tab, LayerPanelTab::Thread);
        s.set_mode(EditorMode::Pan);
        assert_eq!(s.editor.layer_panel_tab, LayerPanelTab::Thread);
        s.set_mode(EditorMode::Pin);
        assert_eq!(s.editor.pin_tool, PinTool::Circle);
        assert_eq!(s.editor.layer_panel_tab, LayerPanelTab::Pin);
    }

    #[test]
    fn layers_add_merge_delete() {
        let mut s = Store::default();
        s.add_pin_layer();
        assert_eq!(s.doc.pin_layers.len(), 2);
        let second = s.doc.pin_layers[1].id.clone();
        assert_eq!(s.editor.active_pin_layer_id, second);
        s.merge_pin_layer_above(&second);
        assert_eq!(s.doc.pin_layers.len(), 1);
        assert_eq!(s.editor.active_pin_layer_id, s.doc.pin_layers[0].id);
        s.undo();
        assert_eq!(s.doc.pin_layers.len(), 2);
        s.delete_pin_layer(&second);
        assert_eq!(s.doc.pin_layers.len(), 1);
    }

    #[test]
    fn load_resets_history() {
        let mut s = Store::default();
        s.set_board_dimensions(BoardDimensions {
            diameter: Some(90.0),
            ..Default::default()
        });
        let file = s.to_project_file();
        s.load_project(file);
        assert!(!s.can_undo());
        assert_eq!(s.doc.board.dimensions.diameter, Some(90.0));
    }
}
