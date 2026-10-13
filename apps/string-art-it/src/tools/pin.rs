//! Pin mode canvas interaction (specs 05, 06, 08, 11, 29, 33). Port of web
//! `usePinDrawing`, `useFreehandDrawing`, `usePolygonDrawing`, `useCenterSnap`,
//! `useEraserHover` and the Pin-mode overlays in `Canvas.tsx`.

use egui::{Color32, CursorIcon, Key, Painter, Ui};
use sa_app::document::board::BOARD_CENTER;
use sa_app::document::{FontWeight, PinPathGeometry, SymmetryConfig};
use sa_app::editor::{PinTool, Store};
use sa_app::hit::{PinHit, nearest_pin_owner};
use sa_app::pin_tools::{curvature_from_cursor, geometry_from_drag, is_drag_tool};
use sa_domain::Point;
use sa_domain::paths::{distribute_closed_path, distribute_open_path, distribute_path_per_vertex};
use sa_infra::fonts::{DEFAULT_FONT_ID, DEFAULT_TEXT_SIZE, build_text_geometry};
use sa_render::path::PathData;
use sa_render::scene::guide_path;

use super::{Pointer, SnapSource, dashed, hit_radius, ring, snap, snap_pins};
use crate::canvas::paint::Xf;
use crate::canvas::{CanvasInput, draw_snap_indicator};
use crate::icons::Icon;
use crate::widgets::Kit;

const MIN_FREEHAND_POINT_DISTANCE_PX: f64 = 3.0;
pub const CANDIDATE: Color32 = Color32::from_rgb(0xe8, 0xb4, 0x49);

#[derive(Default)]
pub struct PinToolState {
    drag_start: Option<Point>,
    /// Arc: start, then end; the third click commits the curvature.
    arc: Option<(Point, Option<Point>)>,
    freehand: Vec<Point>,
    /// Ctrl/Cmd centre snap already consumed within the current gesture.
    center_used: bool,
}

impl PinToolState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn gesture_in_progress(&self, store: &Store) -> bool {
        self.drag_start.is_some()
            || self.arc.is_some()
            || !self.freehand.is_empty()
            || store.editor.polygon_draft.is_some()
    }

    /// Handles one frame of pin-mode input; returns a status-bar override.
    pub fn frame(
        &mut self,
        ui: &mut Ui,
        kit: &mut Kit,
        store: &mut Store,
        input: &CanvasInput,
    ) -> Option<String> {
        let ptr = Pointer::read(ui, input);
        let xf = input.xf;
        let tool = store.editor.pin_tool;
        let layer = store.editor.active_pin_layer_id.clone();
        let max_dist = hit_radius(store);
        let pins = snap_pins(store);

        let suppressed = self.center_used && self.gesture_in_progress(store);
        let center = (ptr.command && !suppressed).then_some(BOARD_CENTER);
        let snapped = ptr.raw.map(|raw| snap(store, &pins, raw, center));
        let cursor = snapped.map(|s| s.0);

        // Keyboard: Path draft Esc finishes, ArrowLeft retracts (spec 33).
        if !ui.ctx().egui_wants_keyboard_input() {
            let (esc, left) =
                ui.input(|i| (i.key_pressed(Key::Escape), i.key_pressed(Key::ArrowLeft)));
            if tool == PinTool::Polygon && store.editor.polygon_draft.is_some() {
                if esc {
                    store.finish_polygon_draft(&layer);
                } else if left {
                    store.retract_polygon_draft();
                }
            } else if esc {
                self.arc = None;
                self.drag_start = None;
                self.freehand.clear();
            }
        }

        // ---- pointer press
        if ptr.pressed
            && let (Some(raw), Some(point)) = (ptr.raw, cursor)
        {
            match tool {
                PinTool::Eraser => {
                    if let Some(h) = nearest_pin_owner(&store.doc.pin_layers, raw, max_dist) {
                        store.erase_pin(&h.layer_id, &h.path_id, &h.pin_id);
                    }
                }
                PinTool::PathEraser => {
                    if let Some(h) = nearest_pin_owner(&store.doc.pin_layers, raw, max_dist) {
                        store.erase_pin_path(&h.layer_id, &h.path_id);
                    }
                }
                _ => {
                    // Centre-snap bookkeeping: Path tool is per click, others per gesture.
                    self.center_used = if tool == PinTool::Polygon {
                        center.is_some()
                    } else {
                        center.is_some() || suppressed
                    };
                    match tool {
                        PinTool::Freehand => self.freehand = vec![point],
                        PinTool::Polygon => {
                            let close = store
                                .editor
                                .polygon_draft
                                .as_ref()
                                .filter(|d| d.len() >= 3)
                                .is_some_and(|d| d[0].distance(point) <= max_dist);
                            if close {
                                store.finish_polygon_draft(&layer);
                            } else {
                                store.extend_polygon_draft(point);
                            }
                        }
                        PinTool::Text => {
                            let g = build_text_geometry(
                                point,
                                "",
                                DEFAULT_FONT_ID,
                                FontWeight::Regular,
                                false,
                                DEFAULT_TEXT_SIZE,
                                0.0,
                            );
                            if let Some(id) = store.add_pin_path(&layer, g) {
                                store.edit_pin_path(&layer, &id);
                            }
                        }
                        PinTool::Arc => match self.arc {
                            None => self.arc = Some((point, None)),
                            Some((start, None)) => self.arc = Some((start, Some(point))),
                            Some((start, Some(end))) => {
                                let curvature = curvature_from_cursor(start, end, point);
                                store.add_pin_path(
                                    &layer,
                                    PinPathGeometry::Arc {
                                        start,
                                        end,
                                        curvature,
                                    },
                                );
                                self.arc = None;
                            }
                        },
                        t if is_drag_tool(t) => self.drag_start = Some(point),
                        _ => {}
                    }
                }
            }
        }

        // ---- drag / release
        if tool == PinTool::Freehand && !self.freehand.is_empty() {
            if ptr.down
                && let Some(raw) = ptr.raw
            {
                let min = MIN_FREEHAND_POINT_DISTANCE_PX / xf.vp.zoom;
                if self.freehand.last().is_none_or(|l| l.distance(raw) >= min) {
                    self.freehand.push(raw);
                }
            }
            if ptr.released {
                let points = std::mem::take(&mut self.freehand);
                store.add_pin_path(&layer, PinPathGeometry::Freehand { points });
            }
        }
        if ptr.released
            && let Some(start) = self.drag_start.take()
            && is_drag_tool(tool)
            && let Some(end) = cursor
            && let Some(g) = geometry_from_drag(tool, start, end, ptr.alt)
        {
            store.add_pin_path(&layer, g);
        }

        // ---- preview geometry
        let preview: Option<PinPathGeometry> = match (tool, cursor) {
            (PinTool::Arc, Some(c)) => match self.arc {
                Some((start, Some(end))) => Some(PinPathGeometry::Arc {
                    start,
                    end,
                    curvature: curvature_from_cursor(start, end, c),
                }),
                Some((start, None)) => Some(PinPathGeometry::Line { start, end: c }),
                None => None,
            },
            (PinTool::Freehand, _) if self.freehand.len() >= 2 => Some(PinPathGeometry::Freehand {
                points: self.freehand.clone(),
            }),
            (PinTool::Polygon, c) => store.editor.polygon_draft.as_ref().map(|d| {
                let mut points = d.clone();
                points.extend(c);
                PinPathGeometry::Freehand { points }
            }),
            (t, Some(c)) if is_drag_tool(t) => self
                .drag_start
                .and_then(|s| geometry_from_drag(t, s, c, ptr.alt)),
            _ => None,
        };

        // ---- overlays
        let painter = ui.painter_at(input.response.rect);
        let accent = kit.t.accent;
        let symmetry = store.active_symmetry().clone();
        symmetry_overlay(&painter, &xf, &symmetry, accent);
        if let Some((p, src)) = snapped
            && matches!(src, SnapSource::Grid | SnapSource::Center)
        {
            draw_snap_indicator(&painter, &xf, p, accent);
        }
        if let (Some(draft), Some(c)) = (&store.editor.polygon_draft, cursor)
            && draft.len() >= 3
            && draft[0].distance(c) <= max_dist
        {
            ring(&painter, &xf, draft[0], 0.35, 0.07, CANDIDATE);
        }
        if let Some(g) = &preview {
            let flat = guide_path(g).flatten(0.3 / xf.vp.zoom);
            for (pts, closed) in &flat {
                let mut pts = pts.clone();
                if *closed && let Some(f) = pts.first().copied() {
                    pts.push(f);
                }
                dashed(&painter, &xf, &pts, 0.1, [0.3, 0.2], accent);
                for copy in symmetry.copies(&pts) {
                    dashed(
                        &painter,
                        &xf,
                        &copy,
                        0.1,
                        [0.3, 0.2],
                        accent.gamma_multiply(0.55),
                    );
                }
            }
        }
        if let Some(raw) = ptr.raw {
            eraser_hover(&painter, &xf, store, tool, raw, max_dist, kit.t.danger);
        }

        // ---- cursor
        if input.response.hovered() {
            match tool {
                PinTool::Text => ui.ctx().set_cursor_icon(CursorIcon::Text),
                PinTool::Eraser | PinTool::PathEraser => {
                    ui.ctx().set_cursor_icon(CursorIcon::None);
                    if let Some(p) = input.response.hover_pos() {
                        let icon = if tool == PinTool::Eraser {
                            Icon::Eraser
                        } else {
                            Icon::Trash2
                        };
                        tool_cursor(ui, kit, icon, p);
                    }
                }
                _ => ui.ctx().set_cursor_icon(CursorIcon::Crosshair),
            }
        }

        preview.map(|g| preview_status(kit, &g, store.editor.pin_defaults.spacing))
    }
}

/// Draws a toolbar glyph as the pointer (web data-URI cursors: black glyph,
/// white outline, hotspot centred).
pub fn tool_cursor(ui: &Ui, kit: &mut Kit, icon: Icon, at: egui::Pos2) {
    let r = egui::Rect::from_center_size(at, egui::vec2(24.0, 24.0));
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("tool-cursor"),
    ));
    let tex_outline = kit.icons.texture(ui.ctx(), icon, 24.0, Some(4.0));
    let tex = kit.icons.texture(ui.ctx(), icon, 24.0, Some(2.0));
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    painter.image(tex_outline.id(), r, uv, Color32::WHITE);
    painter.image(tex.id(), r, uv, Color32::BLACK);
}

fn symmetry_overlay(painter: &Painter, xf: &Xf, config: &SymmetryConfig, accent: Color32) {
    let c = accent.gamma_multiply(0.5);
    let far = 1000.0;
    match config {
        SymmetryConfig::None => {}
        SymmetryConfig::Radial {
            centre,
            interval_degrees,
        } => {
            let mut a = 0.0;
            while a < 360.0 && *interval_degrees > 0.0 {
                let r = f64::to_radians(a);
                let end = Point::new(centre.x + r.cos() * far, centre.y + r.sin() * far);
                dashed(painter, xf, &[*centre, end], 0.03, [0.15, 0.25], c);
                a += interval_degrees;
            }
            ring(painter, xf, *centre, 0.4, 0.06, c);
            painter.circle_filled(xf.pos(*centre), xf.len(0.1).max(1.5), c);
        }
        SymmetryConfig::Vertical { axis }
        | SymmetryConfig::Horizontal { axis }
        | SymmetryConfig::Both { axis } => {
            if matches!(
                config,
                SymmetryConfig::Vertical { .. } | SymmetryConfig::Both { .. }
            ) {
                dashed(
                    painter,
                    xf,
                    &[Point::new(axis.x, -far), Point::new(axis.x, far)],
                    0.03,
                    [0.15, 0.25],
                    c,
                );
            }
            if matches!(
                config,
                SymmetryConfig::Horizontal { .. } | SymmetryConfig::Both { .. }
            ) {
                dashed(
                    painter,
                    xf,
                    &[Point::new(-far, axis.y), Point::new(far, axis.y)],
                    0.03,
                    [0.15, 0.25],
                    c,
                );
            }
        }
    }
}

fn eraser_hover(
    painter: &Painter,
    xf: &Xf,
    store: &Store,
    tool: PinTool,
    raw: Point,
    max: f64,
    danger: Color32,
) {
    let hit: Option<PinHit> = match tool {
        PinTool::Eraser | PinTool::PathEraser => nearest_pin_owner(&store.doc.pin_layers, raw, max),
        _ => None,
    };
    let Some(h) = hit else { return };
    let Some(path) = store.find_pin_path(&h.layer_id, &h.path_id) else {
        return;
    };
    if tool == PinTool::Eraser {
        if let Some(pin) = path.pins.iter().find(|p| p.id == h.pin_id) {
            ring(painter, xf, pin.point(), 0.35, 0.08, danger);
        }
        return;
    }
    let data: PathData = guide_path(&path.geometry);
    for (mut pts, closed) in data.flatten(0.3 / xf.vp.zoom) {
        if closed && let Some(f) = pts.first().copied() {
            pts.push(f);
        }
        dashed(painter, xf, &pts, 0.12, [0.2, 0.15], danger);
    }
    for pin in &path.pins {
        ring(painter, xf, pin.point(), 0.3, 0.07, danger);
    }
}

/// Web StatusBar geometry preview text.
fn preview_status(kit: &Kit, g: &PinPathGeometry, spacing: f64) -> String {
    let Some(path) = g.contour_paths().into_iter().next() else {
        return String::new();
    };
    let length = path.length();
    let spacing = spacing.max(0.1);
    if g.kind().is_vertex_anchored() || path.closed {
        let d = if g.kind().is_vertex_anchored() {
            distribute_path_per_vertex(&path, spacing)
        } else {
            distribute_closed_path(&path, spacing)
        };
        let Ok(d) = d else { return String::new() };
        let label = kit.tr(if path.closed {
            "canvas:status.perimeter"
        } else {
            "canvas:status.length"
        });
        return kit.trf(
            "canvas:status.geometryPreview",
            &[
                ("label", &label),
                ("length", &format!("{length:.1}")),
                ("requested", &format!("{spacing:.1}")),
                ("actual", &format!("{:.2}", d.actual_spacing)),
                ("pins", &d.n),
            ],
        );
    }
    let n = distribute_open_path(&path, spacing)
        .map(|p| p.len())
        .unwrap_or(0);
    kit.trf(
        "canvas:status.openPathPreview",
        &[
            ("label", &kit.tr("canvas:status.length")),
            ("length", &format!("{length:.1}")),
            ("pins", &n),
            ("gap", &format!("{spacing:.1}")),
        ],
    )
}
