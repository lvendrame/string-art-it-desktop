//! Radial context menu (spec 25): right-click on the canvas opens an
//! icon-only ring whose slices depend on the mode and draft state. Escape or a
//! click outside closes it without acting.

use std::f32::consts::{PI, TAU};

use egui::{Color32, Pos2, Sense, Shape, Stroke, Ui, epaint::PathShape, pos2, vec2};
use sa_app::editor::{EditorMode, PinTool, SelectTool, Store, ThreadTool};
use sa_app::threads::MIN_REPEAT_PINS;
use sa_domain::Point;

use crate::canvas::{CanvasView, zoom_in_step, zoom_out_step};
use crate::icons::Icon;
use crate::widgets::Kit;

const INNER: f32 = 32.0;
const OUTER: f32 = 100.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slice {
    Select,
    Move,
    Rotate,
    Scale,
    Merge,
    Pin(PinTool),
    PolygonCut,
    PolygonBack,
    PolygonCancel,
    Thread(ThreadTool),
    ThreadCut,
    ThreadBack,
    ThreadNext,
    TwoPinResolve,
    TwoPinBack,
    TwoPinCancel,
    RepeatGenerate,
    RepeatBack,
    RepeatCancel,
    PanFit,
    PanZoomIn,
    PanZoomOut,
    PlayFirst,
    PlayPrevious,
    PlayToggle,
    PlayNext,
    PlayLast,
}

/// Web `getRadialMenuSliceIds`. Play slices come from the transport (D13).
pub fn slices(store: &Store) -> Vec<Slice> {
    let e = &store.editor;
    match e.mode {
        EditorMode::Select => {
            let mut v = vec![Slice::Select, Slice::Move, Slice::Rotate, Slice::Scale];
            if store.can_merge_selection() {
                v.push(Slice::Merge);
            }
            v
        }
        EditorMode::Pin if e.polygon_draft.is_some() => {
            vec![Slice::PolygonCut, Slice::PolygonBack, Slice::PolygonCancel]
        }
        EditorMode::Pin => [
            PinTool::Line,
            PinTool::Arc,
            PinTool::Ellipse,
            PinTool::Circle,
            PinTool::Rectangle,
            PinTool::Square,
            PinTool::Freehand,
            PinTool::Polygon,
            PinTool::Eraser,
            PinTool::PathEraser,
        ]
        .into_iter()
        .map(Slice::Pin)
        .collect(),
        EditorMode::Thread => {
            if let Some(d) = &e.two_pin_draft {
                return if d.candidates.is_empty() {
                    vec![Slice::TwoPinBack, Slice::TwoPinCancel]
                } else {
                    vec![Slice::TwoPinResolve, Slice::TwoPinBack, Slice::TwoPinCancel]
                };
            }
            if let Some(d) = &e.repeat_draft {
                return if d.len() >= MIN_REPEAT_PINS {
                    vec![
                        Slice::RepeatGenerate,
                        Slice::RepeatBack,
                        Slice::RepeatCancel,
                    ]
                } else {
                    vec![Slice::RepeatBack, Slice::RepeatCancel]
                };
            }
            if e.thread_draft.is_some() {
                return vec![Slice::ThreadCut, Slice::ThreadBack, Slice::ThreadNext];
            }
            [
                ThreadTool::Draw,
                ThreadTool::Zigzag,
                ThreadTool::Parabolic,
                ThreadTool::Radial,
                ThreadTool::Repeat,
                ThreadTool::Eraser,
                ThreadTool::SegmentEraser,
            ]
            .into_iter()
            .map(Slice::Thread)
            .collect()
        }
        EditorMode::Pan => vec![Slice::PanFit, Slice::PanZoomIn, Slice::PanZoomOut],
        EditorMode::Play => vec![
            Slice::PlayFirst,
            Slice::PlayPrevious,
            Slice::PlayToggle,
            Slice::PlayNext,
            Slice::PlayLast,
        ],
        EditorMode::Generate => Vec::new(),
    }
}

fn icon_and_label(s: Slice, playing: bool) -> (Icon, String) {
    let pin = |t: PinTool| -> (Icon, &str) {
        match t {
            PinTool::Line => (Icon::Minus, "toolbars:pinToolbar.basicTools.line"),
            PinTool::Arc => (Icon::Spline, "toolbars:pinToolbar.basicTools.arc"),
            PinTool::Ellipse => (Icon::Ellipse, "toolbars:pinToolbar.basicTools.ellipse"),
            PinTool::Circle => (Icon::Circle, "toolbars:pinToolbar.basicTools.circle"),
            PinTool::Rectangle => (
                Icon::RectangleHorizontal,
                "toolbars:pinToolbar.basicTools.rectangle",
            ),
            PinTool::Square => (Icon::Square, "toolbars:pinToolbar.basicTools.square"),
            PinTool::Freehand => (Icon::PenTool, "toolbars:pinToolbar.basicTools.freehand"),
            PinTool::Polygon => (Icon::Waypoints, "toolbars:pinToolbar.basicTools.path"),
            PinTool::Eraser => (Icon::Eraser, "toolbars:pinToolbar.eraser"),
            _ => (Icon::Trash2, "toolbars:pinToolbar.pathEraser"),
        }
    };
    let thread = |t: ThreadTool| -> (Icon, &str) {
        match t {
            ThreadTool::Draw => (Icon::PenLine, "toolbars:threadToolbar.draw"),
            ThreadTool::Zigzag => (Icon::Waypoints, "toolbars:threadToolbar.zigzag"),
            ThreadTool::Parabolic => (Icon::Croissant, "toolbars:threadToolbar.parabolic"),
            ThreadTool::Radial => (Icon::LoaderPinwheel, "toolbars:threadToolbar.radial"),
            ThreadTool::Repeat => (Icon::Galaxy, "toolbars:threadToolbar.repeat"),
            ThreadTool::Eraser => (Icon::Eraser, "toolbars:threadToolbar.eraser"),
            _ => (Icon::Scissors, "toolbars:threadToolbar.segment"),
        }
    };
    let (icon, key) = match s {
        Slice::Select => (
            Icon::MousePointer2,
            "toolbars:selectToolbar.tools.select.label",
        ),
        Slice::Move => (Icon::Move, "toolbars:selectToolbar.tools.move.label"),
        Slice::Rotate => (Icon::RotateCw, "toolbars:selectToolbar.tools.rotate.label"),
        Slice::Scale => (Icon::Scaling, "toolbars:selectToolbar.tools.scale.label"),
        Slice::Merge => (Icon::GitMerge, "toolbars:selectToolbar.tools.merge.label"),
        Slice::Pin(t) => pin(t),
        Slice::Thread(t) => thread(t),
        Slice::PolygonCut | Slice::ThreadCut | Slice::TwoPinResolve => {
            (Icon::Scissors, "toolbars:radialMenu.cut")
        }
        Slice::PolygonBack | Slice::ThreadBack | Slice::TwoPinBack | Slice::RepeatBack => {
            (Icon::ArrowLeft, "toolbars:radialMenu.back")
        }
        Slice::PolygonCancel | Slice::TwoPinCancel | Slice::RepeatCancel => {
            (Icon::X, "toolbars:radialMenu.cancel")
        }
        Slice::ThreadNext => (Icon::ArrowRight, "toolbars:radialMenu.next"),
        Slice::RepeatGenerate => (Icon::Sparkles, "toolbars:radialMenu.generate"),
        Slice::PanFit => (Icon::Maximize, "toolbars:canvasToolbar.fit"),
        Slice::PanZoomIn => (Icon::ZoomIn, "toolbars:canvasToolbar.zoomIn"),
        Slice::PanZoomOut => (Icon::ZoomOut, "toolbars:canvasToolbar.zoomOut"),
        Slice::PlayFirst => (Icon::SkipBack, "toolbars:playToolbar.firstFrame"),
        Slice::PlayPrevious => (Icon::ChevronLeft, "toolbars:playToolbar.previousFrame"),
        Slice::PlayToggle if playing => (Icon::Pause, "toolbars:playToolbar.pause"),
        Slice::PlayToggle => (Icon::Play, "toolbars:playToolbar.play"),
        Slice::PlayNext => (Icon::ChevronRight, "toolbars:playToolbar.nextFrame"),
        Slice::PlayLast => (Icon::SkipForward, "toolbars:playToolbar.lastFrame"),
    };
    (icon, key.to_owned())
}

/// What a play slice asks the play controller to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayCommand {
    First,
    Previous,
    Toggle,
    Next,
    Last,
}

#[derive(Default)]
pub struct RadialMenu {
    open: Option<(Pos2, f64, EditorMode)>,
}

impl RadialMenu {
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Opens on a right-press inside `canvas_rect` (except Ctrl+right-click in Pin
    /// mode, the macOS centre-snap gesture).
    pub fn maybe_open(&mut self, ui: &Ui, store: &Store, canvas_rect: egui::Rect) {
        let (pressed, pos, ctrl, time) = ui.input(|i| {
            (
                i.pointer.secondary_clicked(),
                i.pointer.interact_pos(),
                i.modifiers.ctrl,
                i.time,
            )
        });
        if !pressed {
            return;
        }
        let Some(pos) = pos.filter(|p| canvas_rect.contains(*p)) else {
            return;
        };
        if store.editor.mode == EditorMode::Pin && ctrl {
            return;
        }
        self.open = Some((pos, time, store.editor.mode));
    }

    /// Draws the open menu; applies the chosen slice. Returns a play command
    /// for the play controller when a Play slice was picked.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        kit: &mut Kit,
        store: &mut Store,
        canvas: &mut CanvasView,
        playing: bool,
    ) -> Option<PlayCommand> {
        let (center, opened_at, mode) = self.open?;
        if mode != store.editor.mode || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.open = None;
            return None;
        }
        let items = slices(store);
        let now = ui.input(|i| i.time);
        let t = (((now - opened_at) / 0.15) as f32).clamp(0.0, 1.0);
        if t < 1.0 {
            ui.ctx().request_repaint();
        }
        let scale = 0.85 + 0.15 * t;
        let (inner, outer) = (INNER * scale, OUTER * scale);
        let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("radial-menu"));
        let painter = ui.ctx().layer_painter(layer);
        let alpha = |c: Color32| c.gamma_multiply(t);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let n = items.len().max(1);
        let hovered = pointer.and_then(|p| {
            let d = p - center;
            let r = d.length();
            if r < inner || r > outer || items.is_empty() {
                return None;
            }
            // Slice 0 centred at the top, clockwise.
            let a = (d.y.atan2(d.x) + PI / 2.0 + PI / n as f32).rem_euclid(TAU);
            Some(((a / TAU) * n as f32) as usize % n)
        });

        // Shadow + ring
        painter.circle_filled(
            center + vec2(0.0, 8.0),
            outer + 4.0,
            alpha(Color32::from_black_alpha(90)),
        );
        painter.circle_filled(center, outer, alpha(kit.t.bg_panel));
        painter.circle_stroke(center, outer, Stroke::new(1.0, alpha(kit.t.border)));
        for (i, s) in items.iter().enumerate() {
            let a0 = -PI / 2.0 - PI / n as f32 + i as f32 * TAU / n as f32;
            let a1 = a0 + TAU / n as f32;
            if hovered == Some(i) {
                let steps = 24;
                let mut pts: Vec<Pos2> = (0..=steps)
                    .map(|k| {
                        let a = a0 + (a1 - a0) * k as f32 / steps as f32;
                        center + vec2(a.cos(), a.sin()) * outer
                    })
                    .collect();
                pts.extend((0..=steps).rev().map(|k| {
                    let a = a0 + (a1 - a0) * k as f32 / steps as f32;
                    center + vec2(a.cos(), a.sin()) * inner
                }));
                painter.add(Shape::Path(PathShape::closed_line(
                    pts.clone(),
                    Stroke::NONE,
                )));
                // filled sector via mesh (concave polygon) — draw as triangles
                let mut mesh = egui::Mesh::default();
                for k in 0..steps {
                    let o0 = pts[k];
                    let o1 = pts[k + 1];
                    let i0 = pts[2 * steps + 1 - k];
                    let i1 = pts[2 * steps - k];
                    let base = mesh.vertices.len() as u32;
                    for p in [o0, o1, i1, i0] {
                        mesh.colored_vertex(p, alpha(kit.t.accent_soft_2));
                    }
                    mesh.add_triangle(base, base + 1, base + 2);
                    mesh.add_triangle(base, base + 2, base + 3);
                }
                painter.add(Shape::mesh(mesh));
            }
            // separators
            let dir = vec2(a0.cos(), a0.sin());
            if n > 1 {
                painter.line_segment(
                    [center + dir * inner, center + dir * outer],
                    Stroke::new(1.0, alpha(kit.t.border)),
                );
            }
            let mid = (a0 + a1) / 2.0;
            let ic = center + vec2(mid.cos(), mid.sin()) * (inner + outer) / 2.0;
            let (icon, _) = icon_and_label(*s, playing);
            let color = if hovered == Some(i) {
                kit.t.accent
            } else {
                kit.t.text_primary
            };
            let tex = kit.icons.texture(ui.ctx(), icon, 16.0, None);
            painter.image(
                tex.id(),
                egui::Rect::from_center_size(ic, vec2(16.0, 16.0)),
                egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                alpha(color),
            );
        }
        painter.circle_filled(center, inner, alpha(kit.t.bg_app));
        painter.circle_stroke(center, inner, Stroke::new(1.0, alpha(kit.t.border)));

        if let Some(i) = hovered {
            let (_, key) = icon_and_label(items[i], playing);
            let label = kit.tr(&key);
            let galley =
                painter.layout_no_wrap(label, egui::FontId::proportional(12.0), kit.t.text_primary);
            let pos = center + vec2(-galley.size().x / 2.0, outer + 8.0);
            let bg =
                egui::Rect::from_min_size(pos - vec2(6.0, 3.0), galley.size() + vec2(12.0, 6.0));
            painter.rect_filled(bg, 6.0, kit.t.bg_panel_2);
            painter.galley(pos, galley, kit.t.text_primary);
        }

        // Block canvas interaction underneath while open.
        let area = egui::Area::new(egui::Id::new("radial-blocker"))
            .order(egui::Order::Middle)
            .fixed_pos(pos2(0.0, 0.0));
        area.show(ui.ctx(), |ui| {
            ui.allocate_rect(ui.ctx().content_rect(), Sense::click());
        });

        let clicked = ui.input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked());
        let pressed_now = ui.input(|i| {
            (i.pointer.primary_pressed() || i.pointer.secondary_pressed())
                && i.time > opened_at + 0.05
        });
        if !(clicked || pressed_now) {
            return None;
        }
        self.open = None;
        let i = hovered?;
        let anchor = Point::new(
            (center.x - canvas.origin().x) as f64,
            (center.y - canvas.origin().y) as f64,
        );
        apply(items[i], store, canvas, anchor)
    }
}

fn apply(
    s: Slice,
    store: &mut Store,
    canvas: &mut CanvasView,
    anchor: Point,
) -> Option<PlayCommand> {
    let pl = store.editor.active_pin_layer_id.clone();
    let tl = store.editor.active_thread_layer_id.clone();
    match s {
        Slice::Select => store.set_select_tool(SelectTool::Select),
        Slice::Move => store.set_select_tool(SelectTool::Move),
        Slice::Rotate => store.set_select_tool(SelectTool::Rotate),
        Slice::Scale => store.set_select_tool(SelectTool::Scale),
        Slice::Merge => store.commit_selection_merge(),
        Slice::Pin(t) => store.set_pin_tool(t),
        Slice::PolygonCut => store.finish_polygon_draft(&pl),
        Slice::PolygonBack => store.retract_polygon_draft(),
        Slice::PolygonCancel => store.cancel_polygon_draft(),
        Slice::Thread(t) => store.set_thread_tool(t),
        Slice::ThreadCut => store.finish_thread_draft(&tl),
        Slice::ThreadBack => store.retract_thread_draft(),
        Slice::ThreadNext => store.advance_thread_draft_by_pattern(),
        Slice::TwoPinResolve => store.resolve_two_pin_draft(&tl),
        Slice::TwoPinBack => store.retract_two_pin_draft(),
        Slice::TwoPinCancel => store.cancel_two_pin_draft(),
        Slice::RepeatGenerate => store.generate_repeat_draft(&tl),
        Slice::RepeatBack => store.retract_repeat_draft(),
        Slice::RepeatCancel => store.cancel_repeat_draft(),
        Slice::PanFit => canvas.request_fit(),
        Slice::PanZoomIn => store.editor.viewport = zoom_in_step(&store.editor.viewport, anchor),
        Slice::PanZoomOut => store.editor.viewport = zoom_out_step(&store.editor.viewport, anchor),
        Slice::PlayFirst => return Some(PlayCommand::First),
        Slice::PlayPrevious => return Some(PlayCommand::Previous),
        Slice::PlayToggle => return Some(PlayCommand::Toggle),
        Slice::PlayNext => return Some(PlayCommand::Next),
        Slice::PlayLast => return Some(PlayCommand::Last),
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_sets_follow_mode_and_drafts() {
        let mut s = Store::default();
        assert_eq!(slices(&s).len(), 10);
        s.extend_polygon_draft(Point::ORIGIN);
        assert_eq!(
            slices(&s),
            vec![Slice::PolygonCut, Slice::PolygonBack, Slice::PolygonCancel]
        );
        s.set_mode(EditorMode::Select);
        assert_eq!(slices(&s).len(), 4);
        s.set_mode(EditorMode::Thread);
        assert_eq!(slices(&s).len(), 7);
        s.extend_thread_draft("x");
        assert_eq!(
            slices(&s),
            vec![Slice::ThreadCut, Slice::ThreadBack, Slice::ThreadNext]
        );
        s.set_mode(EditorMode::Generate);
        assert!(slices(&s).is_empty());
    }
}
