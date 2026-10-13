//! Edit-mode canvas interaction: click / Alt-Cmd-click / rubber-band select,
//! Move/Rotate/Scale drags, arrow-key nudges with hold-to-repeat (specs 09,
//! 21, 23, 26). Port of web `useSelectTool`, `useMoveTool`, `useRotateTool`,
//! `useScaleTool`, `useKeyboardTransform`.

use egui::{CursorIcon, Key, Ui};
use sa_app::editor::{PinPathRef, PinRef, SelectGranularity, SelectTool, Selection, Store};
use sa_app::editor_select::{Transform, TransformSession};
use sa_app::hit::{
    DocRect, nearest_select_path_hit, nearest_select_pin_hit, selectable_pin_paths_touching_rect,
    selectable_pins_touching_rect,
};
use sa_domain::Point;
use sa_render::scene::guide_path;

use super::{Pointer, dashed, hit_radius, ring, snap, snap_pins};
use crate::canvas::CanvasInput;
use crate::icons::Icon;
use crate::widgets::Kit;

const SELECT_DRAG_THRESHOLD_PX: f32 = 4.0;
pub const ROTATION_SENSITIVITY_RAD_PER_PX: f64 = 0.3 * std::f64::consts::PI / 180.0;
pub const SCALE_SENSITIVITY_PER_PX: f64 = 0.004;
pub const MIN_SCALE_FACTOR: f64 = 0.05;
const REPEAT_DELAY_S: f64 = 0.5;
const REPEAT_INTERVAL_S: f64 = 0.1;

enum Drag {
    Marquee {
        origin: Point,
        origin_px: egui::Pos2,
        current: Point,
        dragging: bool,
    },
    Move {
        origin: Point,
        session: TransformSession,
    },
    Rotate {
        pivot: Point,
        start_x: f32,
        session: TransformSession,
    },
    Scale {
        pivot: Point,
        start_x: f32,
        session: TransformSession,
    },
}

#[derive(Default)]
pub struct SelectToolState {
    drag: Option<Drag>,
    held: Option<(Key, bool, f64)>,
}

fn scale_factor(dx: f32) -> f64 {
    (1.0 + dx as f64 * SCALE_SENSITIVITY_PER_PX).max(MIN_SCALE_FACTOR)
}

impl SelectToolState {
    pub fn frame(&mut self, ui: &mut Ui, kit: &mut Kit, store: &mut Store, input: &CanvasInput) {
        let ptr = Pointer::read(ui, input);
        let xf = input.xf;
        let max = hit_radius(store);
        let pins = snap_pins(store);
        let screen = ui.input(|i| i.pointer.interact_pos());
        let tool = store.editor.select_tool;

        // Escape aborts any drag (marquee included) with no store mutation.
        if ui.input(|i| i.key_pressed(Key::Escape))
            && let Some(
                Drag::Move { session, .. }
                | Drag::Rotate { session, .. }
                | Drag::Scale { session, .. },
            ) = self.drag.take()
        {
            store.abort_transform(session);
        }

        if ptr.pressed
            && let (Some(raw), Some(px)) = (ptr.raw, screen)
        {
            let point = snap(store, &pins, raw, None).0;
            self.drag =
                match tool {
                    SelectTool::Select => Some(Drag::Marquee {
                        origin: raw,
                        origin_px: px,
                        current: raw,
                        dragging: false,
                    }),
                    SelectTool::Move => store.begin_transform().map(|session| Drag::Move {
                        origin: point,
                        session,
                    }),
                    SelectTool::Rotate => store.begin_transform().map(|session| Drag::Rotate {
                        pivot: point,
                        start_x: px.x,
                        session,
                    }),
                    SelectTool::Scale => store.selection_pivot().zip(store.begin_transform()).map(
                        |(pivot, session)| Drag::Scale {
                            pivot,
                            start_x: px.x,
                            session,
                        },
                    ),
                };
        }

        // Live preview
        if let (Some(drag), Some(raw), Some(px)) = (&mut self.drag, ptr.raw, screen) {
            match drag {
                Drag::Marquee {
                    origin_px,
                    current,
                    dragging,
                    ..
                } => {
                    *current = raw;
                    *dragging |= px.distance(*origin_px) > SELECT_DRAG_THRESHOLD_PX;
                }
                Drag::Move { origin, session } => {
                    let p = snap(store, &pins, raw, None).0;
                    store.preview_transform(
                        session,
                        Transform::Move(Point::new(p.x - origin.x, p.y - origin.y)),
                    );
                }
                Drag::Rotate {
                    pivot,
                    start_x,
                    session,
                } => {
                    let theta = (px.x - *start_x) as f64 * ROTATION_SENSITIVITY_RAD_PER_PX;
                    store.preview_transform(
                        session,
                        Transform::Rotate {
                            pivot: *pivot,
                            theta,
                        },
                    );
                }
                Drag::Scale {
                    pivot,
                    start_x,
                    session,
                } => {
                    store.preview_transform(
                        session,
                        Transform::Scale {
                            pivot: *pivot,
                            factor: scale_factor(px.x - *start_x),
                        },
                    );
                }
            }
        }

        if ptr.released
            && let Some(drag) = self.drag.take()
        {
            let alt = ptr.alt || ptr.command;
            let raw = ptr.raw;
            let px = screen;
            match drag {
                Drag::Marquee {
                    origin,
                    current,
                    dragging,
                    ..
                } => {
                    let end = raw.unwrap_or(current);
                    if dragging {
                        rubber_band(
                            store,
                            DocRect {
                                x0: origin.x,
                                y0: origin.y,
                                x1: end.x,
                                y1: end.y,
                            },
                            alt,
                        );
                    } else {
                        click(store, end, max, alt);
                    }
                }
                Drag::Move { origin, session } => {
                    let p = raw.map(|r| snap(store, &pins, r, None).0).unwrap_or(origin);
                    store.commit_transform(
                        session,
                        Transform::Move(Point::new(p.x - origin.x, p.y - origin.y)),
                    );
                }
                Drag::Rotate {
                    pivot,
                    start_x,
                    session,
                } => {
                    let dx = px.map(|p| p.x - start_x).unwrap_or(0.0);
                    store.commit_transform(
                        session,
                        Transform::Rotate {
                            pivot,
                            theta: dx as f64 * ROTATION_SENSITIVITY_RAD_PER_PX,
                        },
                    );
                }
                Drag::Scale {
                    pivot,
                    start_x,
                    session,
                } => {
                    let dx = px.map(|p| p.x - start_x).unwrap_or(0.0);
                    store.commit_transform(
                        session,
                        Transform::Scale {
                            pivot,
                            factor: scale_factor(dx),
                        },
                    );
                }
            }
        }

        self.keyboard(ui, store);

        // ---- overlays
        let painter = ui.painter_at(input.response.rect);
        let accent = kit.t.accent;
        for p in store.selected_pin_paths() {
            for (mut pts, closed) in guide_path(&p.geometry).flatten(0.3 / xf.vp.zoom) {
                if closed && let Some(f) = pts.first().copied() {
                    pts.push(f);
                }
                dashed(&painter, &xf, &pts, 0.08, [0.2, 0.15], accent);
            }
        }
        if store.editor.select_granularity == SelectGranularity::Pins {
            for pin in store.selected_pins() {
                ring(&painter, &xf, pin.point(), 0.35, 0.08, accent);
            }
        }
        if let Some(Drag::Marquee {
            origin,
            current,
            dragging: true,
            ..
        }) = &self.drag
        {
            let r = egui::Rect::from_two_pos(xf.pos(*origin), xf.pos(*current));
            painter.rect_filled(r, 0.0, kit.t.accent_soft.gamma_multiply(0.25));
            let corners = [
                Point::new(origin.x, origin.y),
                Point::new(current.x, origin.y),
                Point::new(current.x, current.y),
                Point::new(origin.x, current.y),
                Point::new(origin.x, origin.y),
            ];
            dashed(&painter, &xf, &corners, 0.03, [0.15, 0.1], accent);
        }

        if input.response.hovered() {
            match tool {
                SelectTool::Select => {}
                SelectTool::Move => ui.ctx().set_cursor_icon(CursorIcon::Move),
                SelectTool::Scale => ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal),
                SelectTool::Rotate => {
                    ui.ctx().set_cursor_icon(CursorIcon::None);
                    if let Some(p) = input.response.hover_pos() {
                        super::pin::tool_cursor(ui, kit, Icon::RotateCw, p);
                    }
                }
            }
        }
    }

    /// Arrow keys drive the active Move/Rotate/Scale (1 px / 1° / 1%, ×10 with
    /// Shift); hold repeats after 500 ms, then every 100 ms.
    fn keyboard(&mut self, ui: &mut Ui, store: &mut Store) {
        if ui.ctx().egui_wants_keyboard_input() || store.editor.select_tool == SelectTool::Select {
            self.held = None;
            return;
        }
        let now = ui.input(|i| i.time);
        let arrows = [
            Key::ArrowUp,
            Key::ArrowDown,
            Key::ArrowLeft,
            Key::ArrowRight,
        ];
        let pressed = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if arrows.contains(key) => Some((*key, modifiers.shift)),
                _ => None,
            })
        });
        if let Some((key, big)) = pressed {
            if nudge(store, key, big) {
                self.held = Some((key, big, now + REPEAT_DELAY_S));
            }
        } else if let Some((key, big, next)) = self.held {
            if !ui.input(|i| i.key_down(key)) {
                self.held = None;
                store.end_coalescing();
            } else if now >= next {
                if nudge(store, key, big) {
                    self.held = Some((key, big, next + REPEAT_INTERVAL_S));
                } else {
                    self.held = None;
                }
            }
        }
        if self.held.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}

/// One nudge step (spec 23). Each press is its own undo step; holds coalesce.
fn nudge(store: &mut Store, key: Key, big: bool) -> bool {
    if !store.has_transformable_selection() {
        return false;
    }
    let step = if big { 10.0 } else { 1.0 };
    let t = match store.editor.select_tool {
        SelectTool::Move => {
            let d = step / store.editor.viewport.zoom;
            Transform::Move(match key {
                Key::ArrowLeft => Point::new(-d, 0.0),
                Key::ArrowRight => Point::new(d, 0.0),
                Key::ArrowUp => Point::new(0.0, -d),
                _ => Point::new(0.0, d),
            })
        }
        SelectTool::Rotate => {
            let sign = if matches!(key, Key::ArrowUp | Key::ArrowLeft) {
                -1.0
            } else {
                1.0
            };
            let Some(pivot) = store.selection_pivot() else {
                return false;
            };
            Transform::Rotate {
                pivot,
                theta: (sign * step).to_radians(),
            }
        }
        SelectTool::Scale => {
            let sign = if matches!(key, Key::ArrowUp | Key::ArrowRight) {
                1.0
            } else {
                -1.0
            };
            let Some(pivot) = store.selection_pivot() else {
                return false;
            };
            Transform::Scale {
                pivot,
                factor: (1.0 + sign * step / 100.0).max(MIN_SCALE_FACTOR),
            }
        }
        SelectTool::Select => return false,
    };
    store.apply_transform("keyboard-transform", t);
    true
}

fn click(store: &mut Store, p: Point, max: f64, toggle: bool) {
    let e = &store.editor;
    match e.select_granularity {
        SelectGranularity::Path => {
            let Some(hit) =
                nearest_select_path_hit(&store.doc.pin_layers, &e.active_pin_layer_id, p, max)
            else {
                if !toggle {
                    store.select(Selection::None);
                }
                return;
            };
            let r = PinPathRef {
                layer_id: hit.layer_id,
                path_id: hit.path_id,
            };
            if !toggle {
                store.select(Selection::PinPaths(vec![r]));
                return;
            }
            let mut cur = match &e.selection {
                Selection::PinPaths(v) => v.clone(),
                _ => Vec::new(),
            };
            if let Some(i) = cur.iter().position(|x| *x == r) {
                cur.remove(i);
            } else {
                cur.push(r);
            }
            store.select(if cur.is_empty() {
                Selection::None
            } else {
                Selection::PinPaths(cur)
            });
        }
        SelectGranularity::Pins => {
            let Some(hit) =
                nearest_select_pin_hit(&store.doc.pin_layers, &e.active_pin_layer_id, p, max)
            else {
                if !toggle {
                    store.select(Selection::None);
                }
                return;
            };
            let r = PinRef {
                layer_id: hit.layer_id,
                path_id: hit.path_id,
                pin_id: hit.pin_id,
            };
            if !toggle {
                store.select(Selection::Pins(vec![r]));
                return;
            }
            let mut cur = match &e.selection {
                Selection::Pins(v) => v.clone(),
                _ => Vec::new(),
            };
            if let Some(i) = cur.iter().position(|x| x.pin_id == r.pin_id) {
                cur.remove(i);
            } else {
                cur.push(r);
            }
            store.select(if cur.is_empty() {
                Selection::None
            } else {
                Selection::Pins(cur)
            });
        }
    }
}

fn rubber_band(store: &mut Store, rect: DocRect, union: bool) {
    let e = &store.editor;
    match e.select_granularity {
        SelectGranularity::Path => {
            let touched: Vec<PinPathRef> = selectable_pin_paths_touching_rect(
                &store.doc.pin_layers,
                &e.active_pin_layer_id,
                rect,
            )
            .into_iter()
            .map(|(layer_id, path_id)| PinPathRef { layer_id, path_id })
            .collect();
            let mut out = if union {
                match &e.selection {
                    Selection::PinPaths(v) => v.clone(),
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            for t in touched {
                if !out.contains(&t) {
                    out.push(t);
                }
            }
            store.select(if out.is_empty() {
                Selection::None
            } else {
                Selection::PinPaths(out)
            });
        }
        SelectGranularity::Pins => {
            let touched: Vec<PinRef> =
                selectable_pins_touching_rect(&store.doc.pin_layers, &e.active_pin_layer_id, rect)
                    .into_iter()
                    .map(|h| PinRef {
                        layer_id: h.layer_id,
                        path_id: h.path_id,
                        pin_id: h.pin_id,
                    })
                    .collect();
            let mut out = if union {
                match &e.selection {
                    Selection::Pins(v) => v.clone(),
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            for t in touched {
                if !out.iter().any(|x| x.pin_id == t.pin_id) {
                    out.push(t);
                }
            }
            store.select(if out.is_empty() {
                Selection::None
            } else {
                Selection::Pins(out)
            });
        }
    }
}
