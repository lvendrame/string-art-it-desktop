//! Thread mode canvas interaction (specs 11, 12, 22, 27, 35, 37, 38). Port of
//! web `useThreadDrawing`, `useTwoPinSequenceDrawing`, `useRepeatDrawing`, the
//! draft layers and `PinHighlightOverlay`.

use egui::{Color32, CursorIcon, Key, Painter, Ui};
use sa_app::document::layers::pin_index;
use sa_app::editor::{Selection, Store, ThreadTool};
use sa_app::hit::{nearest_thread_insertion_pin, nearest_thread_path};
use sa_app::threads::MIN_REPEAT_PINS;
use sa_domain::Point;
use sa_render::Rgba;
use sa_render::scene::thread_strands;

use super::pin::{CANDIDATE, tool_cursor};
use super::{Pointer, dashed, hit_radius, ring};
use crate::canvas::CanvasInput;
use crate::canvas::paint::{Xf, stroke_polyline};
use crate::icons::Icon;
use crate::widgets::Kit;

#[derive(Default)]
pub struct ThreadToolState;

fn draw_thread(painter: &Painter, xf: &Xf, store: &Store, ids: &[String], colours: &[String]) {
    let index = pin_index(&store.doc.pin_layers);
    let pts: Vec<Point> = ids
        .iter()
        .filter_map(|id| index.get(id))
        .map(|p| p.point())
        .collect();
    if pts.len() < 2 {
        return;
    }
    let d = &store.editor.thread_defaults;
    for (points, stroke) in thread_strands(&pts, colours, d.width, d.twist_pitch) {
        stroke_polyline(painter, xf, &points, &stroke);
    }
}

fn preview_line(painter: &Painter, xf: &Xf, store: &Store, from: Point, to: Point, colour: &str) {
    let c = Rgba::parse(colour);
    let w = store.editor.thread_defaults.width * 0.1;
    dashed(
        painter,
        xf,
        &[from, to],
        w,
        [0.2, 0.15],
        Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a),
    );
}

impl ThreadToolState {
    pub fn frame(
        &mut self,
        ui: &mut Ui,
        kit: &mut Kit,
        store: &mut Store,
        input: &CanvasInput,
    ) -> Option<String> {
        let ptr = Pointer::read(ui, input);
        let xf = input.xf;
        let tool = store.editor.thread_tool;
        let layer = store.editor.active_thread_layer_id.clone();
        let max = hit_radius(store);
        let candidate = ptr.raw.and_then(|r| {
            nearest_thread_insertion_pin(
                &store.doc.pin_layers,
                &store.editor.active_pin_layer_id,
                r,
                max,
            )
            .map(|h| h.pin_id)
        });

        self.keyboard(ui, store, &layer);

        // ---- pointer
        if ptr.pressed
            && let Some(raw) = ptr.raw
        {
            match tool {
                ThreadTool::Eraser | ThreadTool::SegmentEraser | ThreadTool::Select => {
                    let hit = nearest_thread_path(
                        &store.doc.thread_layers,
                        &store.doc.pin_layers,
                        raw,
                        max,
                    );
                    match (tool, hit) {
                        (ThreadTool::Eraser, Some(h)) => {
                            store.delete_thread_path(&h.layer_id, &h.path_id)
                        }
                        (ThreadTool::SegmentEraser, Some(h)) => {
                            store.erase_thread_segment(&h.layer_id, &h.path_id, h.segment_index)
                        }
                        (ThreadTool::Select, h) => store.select(match h {
                            Some(h) => Selection::ThreadPath {
                                layer_id: h.layer_id,
                                path_id: h.path_id,
                            },
                            None => Selection::None,
                        }),
                        _ => {}
                    }
                }
                ThreadTool::Zigzag | ThreadTool::Parabolic | ThreadTool::Radial => {
                    let resolving = store
                        .editor
                        .two_pin_draft
                        .as_ref()
                        .is_some_and(|d| !d.candidates.is_empty());
                    if resolving {
                        store.resolve_two_pin_draft(&layer);
                    } else if let Some(pin) = &candidate {
                        if store.editor.two_pin_draft.is_none() {
                            store.start_two_pin_draft(tool, pin);
                        } else {
                            store.choose_second_pin(&layer, pin);
                        }
                    }
                }
                ThreadTool::Repeat => {
                    if let Some(pin) = &candidate {
                        store.extend_repeat_draft(pin);
                    }
                }
                ThreadTool::Draw => {
                    // A repeated click on the draft's own last pin adds nothing (it
                    // would be a zero-length segment); double-click finishes below.
                    if let Some(pin) = &candidate
                        && store.editor.thread_draft.as_ref().and_then(|d| d.last()) != Some(pin)
                    {
                        store.extend_thread_draft(pin);
                    }
                }
            }
        }
        // Double-click: the first click already added the pin; finish there.
        if tool == ThreadTool::Draw && ptr.double_clicked && store.editor.thread_draft.is_some() {
            store.finish_thread_draft(&layer);
        }

        // Two-pin: hovering picks the nearest candidate by far endpoint.
        if let (Some(raw), Some(d)) = (ptr.raw, &store.editor.two_pin_draft)
            && !d.candidates.is_empty()
        {
            let index = pin_index(&store.doc.pin_layers);
            let best = d
                .candidates
                .iter()
                .enumerate()
                .filter_map(|(i, s)| Some((i, index.get(s.last()?)?.point().distance(raw))))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i);
            if let Some(i) = best {
                store.set_two_pin_chosen_candidate(i);
            }
        }

        // ---- overlays
        let painter = ui.painter_at(input.response.rect);
        let index = pin_index(&store.doc.pin_layers);
        let cursor_target = candidate
            .as_ref()
            .and_then(|c| index.get(c))
            .map(|p| p.point())
            .or(ptr.raw);
        let colours = store.editor.thread_defaults.colours.clone();

        // Selected thread halo
        if let Some(t) = store.selected_thread_path() {
            let pts: Vec<Point> = t
                .pin_ids
                .iter()
                .filter_map(|id| index.get(id))
                .map(|p| p.point())
                .collect();
            let halo = sa_render::scene::Stroke {
                color: Rgba {
                    r: kit.t.accent.r(),
                    g: kit.t.accent.g(),
                    b: kit.t.accent.b(),
                    a: 153,
                },
                width: t.width * 0.1 + 0.08,
                dash: None,
            };
            stroke_polyline(&painter, &xf, &pts, &halo);
        }

        let mut origin: Option<String> = None;
        let mut used: Vec<String> = Vec::new();
        match tool {
            ThreadTool::Draw => {
                if let Some(d) = &store.editor.thread_draft {
                    draw_thread(&painter, &xf, store, d, &colours);
                    if let (Some(last), Some(to)) =
                        (d.last().and_then(|l| index.get(l)), cursor_target)
                    {
                        preview_line(&painter, &xf, store, last.point(), to, &colours[0]);
                    }
                    origin = d.last().cloned();
                    used = d[..d.len().saturating_sub(1)].to_vec();
                }
            }
            ThreadTool::Repeat => {
                if let Some(d) = &store.editor.repeat_draft {
                    let first = store
                        .editor
                        .repeat_settings
                        .colours
                        .first()
                        .cloned()
                        .unwrap_or_else(|| colours[0].clone());
                    draw_thread(&painter, &xf, store, d, std::slice::from_ref(&first));
                    if let (Some(last), Some(to)) =
                        (d.last().and_then(|l| index.get(l)), cursor_target)
                    {
                        preview_line(&painter, &xf, store, last.point(), to, &first);
                    }
                    origin = d.last().cloned();
                    used = d[..d.len().saturating_sub(1)].to_vec();
                }
            }
            ThreadTool::Zigzag | ThreadTool::Parabolic | ThreadTool::Radial => {
                if let Some(d) = &store.editor.two_pin_draft {
                    if d.candidates.is_empty() {
                        if let (Some(from), Some(to)) = (index.get(&d.first_pin_id), cursor_target)
                        {
                            preview_line(&painter, &xf, store, from.point(), to, &colours[0]);
                        }
                    } else {
                        draw_thread(
                            &painter,
                            &xf,
                            store,
                            &d.candidates[d.chosen_index],
                            &colours,
                        );
                    }
                    origin = Some(d.first_pin_id.clone());
                }
            }
            _ => {}
        }
        if !matches!(
            tool,
            ThreadTool::Select | ThreadTool::Eraser | ThreadTool::SegmentEraser
        ) {
            let accent = kit.t.accent;
            for id in &used {
                if Some(id) != origin.as_ref()
                    && Some(id) != candidate.as_ref()
                    && let Some(p) = index.get(id)
                {
                    painter.circle_filled(
                        xf.pos(p.point()),
                        xf.len(0.16).max(2.0),
                        accent.gamma_multiply(0.5),
                    );
                }
            }
            if let Some(p) = origin.as_ref().and_then(|o| index.get(o)) {
                painter.circle_filled(xf.pos(p.point()), xf.len(0.16).max(2.0), accent);
            }
            if let Some(c) = candidate
                .as_ref()
                .filter(|c| Some(*c) != origin.as_ref())
                .and_then(|c| index.get(c))
            {
                ring(&painter, &xf, c.point(), 0.35, 0.07, CANDIDATE);
                painter.circle_filled(xf.pos(c.point()), xf.len(0.16).max(2.0), CANDIDATE);
            }
        }

        // Eraser hover previews (danger)
        if let Some(raw) = ptr.raw
            && matches!(tool, ThreadTool::Eraser | ThreadTool::SegmentEraser)
            && let Some(h) =
                nearest_thread_path(&store.doc.thread_layers, &store.doc.pin_layers, raw, max)
            && let Some(t) = store
                .doc
                .thread_layers
                .iter()
                .find(|l| l.id == h.layer_id)
                .and_then(|l| l.path(&h.path_id))
        {
            let danger = kit.t.danger;
            let pts: Vec<Point> = if tool == ThreadTool::Eraser {
                t.pin_ids
                    .iter()
                    .filter_map(|id| index.get(id))
                    .map(|p| p.point())
                    .collect()
            } else {
                t.pin_ids[h.segment_index..=h.segment_index + 1]
                    .iter()
                    .filter_map(|id| index.get(id))
                    .map(|p| p.point())
                    .collect()
            };
            let (w, alpha) = if tool == ThreadTool::Eraser {
                (t.width * 0.1 + 0.25, 191)
            } else {
                (0.35, 217)
            };
            let s = sa_render::scene::Stroke {
                color: Rgba {
                    r: danger.r(),
                    g: danger.g(),
                    b: danger.b(),
                    a: alpha,
                },
                width: w,
                dash: None,
            };
            stroke_polyline(&painter, &xf, &pts, &s);
        }

        if input.response.hovered() {
            match tool {
                ThreadTool::Select => {}
                ThreadTool::Eraser | ThreadTool::SegmentEraser => {
                    ui.ctx().set_cursor_icon(CursorIcon::None);
                    if let Some(p) = input.response.hover_pos() {
                        tool_cursor(
                            ui,
                            kit,
                            if tool == ThreadTool::Eraser {
                                Icon::Eraser
                            } else {
                                Icon::Scissors
                            },
                            p,
                        );
                    }
                }
                _ => ui.ctx().set_cursor_icon(CursorIcon::Crosshair),
            }
        }

        self.status(store, tool, candidate.as_deref(), cursor_target)
    }

    fn keyboard(&mut self, ui: &mut Ui, store: &mut Store, layer: &str) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let (esc, left, right, enter) = ui.input(|i| {
            (
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::Enter),
            )
        });
        match store.editor.thread_tool {
            ThreadTool::Zigzag | ThreadTool::Parabolic | ThreadTool::Radial => {
                if esc {
                    store.cancel_two_pin_draft();
                } else if left {
                    store.retract_two_pin_draft();
                }
            }
            ThreadTool::Repeat => {
                if enter {
                    store.generate_repeat_draft(layer);
                } else if esc {
                    store.cancel_repeat_draft();
                } else if left {
                    store.retract_repeat_draft();
                }
            }
            _ => {
                if esc {
                    store.escape_thread_draft(layer);
                } else if left {
                    store.retract_thread_draft();
                } else if right {
                    store.advance_thread_draft_by_pattern();
                }
            }
        }
    }

    /// Web status texts (these are English-only in the web app too).
    fn status(
        &self,
        store: &Store,
        tool: ThreadTool,
        candidate: Option<&str>,
        cursor: Option<Point>,
    ) -> Option<String> {
        let index = pin_index(&store.doc.pin_layers);
        match tool {
            ThreadTool::Zigzag | ThreadTool::Parabolic | ThreadTool::Radial => {
                match &store.editor.two_pin_draft {
                    None => candidate.map(|c| format!("Pin {c} — click to start.")),
                    Some(d) if d.candidates.is_empty() => Some(match candidate {
                        Some(c) => format!("From Pin {} → {c} — click to compute.", d.first_pin_id),
                        None => format!("From Pin {} — click the second pin.", d.first_pin_id),
                    }),
                    Some(d) => Some(format!(
                        "{} candidates — click anywhere to confirm the previewed one.",
                        d.candidates.len()
                    )),
                }
            }
            ThreadTool::Repeat => {
                let count = store.editor.repeat_draft.as_ref().map_or(0, Vec::len);
                if count == 0 {
                    return candidate.map(|c| format!("Pin {c} — click to start."));
                }
                let usable = count - count % 2;
                Some(if usable >= MIN_REPEAT_PINS {
                    format!("{count} pins — press Enter to generate.")
                } else {
                    format!("{count} pins — pick at least {MIN_REPEAT_PINS} (even count).")
                })
            }
            ThreadTool::Draw => {
                let Some(d) = &store.editor.thread_draft else {
                    return candidate.map(|c| format!("Pin {c} — click to start a Thread Path."));
                };
                let last = d.last()?;
                let from = index.get(last)?.point();
                let Some(to) = cursor else {
                    return Some(format!("From Pin {last} — click the next pin to extend."));
                };
                Some(format!(
                    "From Pin {last} → {} | Segment: {:.1} cm",
                    candidate.unwrap_or("?"),
                    from.distance(to)
                ))
            }
            _ => None,
        }
    }
}
