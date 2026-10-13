//! Central keyboard shortcut dispatcher (spec 34). Port of web
//! `useKeyboardShortcuts.ts` + EditorShell's undo/redo listener. Mode-specific
//! bindings win over global ones. Tool controllers own Esc / arrow handling
//! for their drafts; arrow-pan only fires when no tool claims the key.

use egui::{Event, Key, Modifiers};
use sa_app::document::{SYMMETRY_TYPE_CYCLE_REF, SymmetryConfig};
use sa_app::editor::{EditorMode, PinTool, SelectGranularity, SelectTool, Store, ThreadTool};
use sa_app::editor_threads::ThreadPropertyPatch;
use sa_domain::Point;

use crate::canvas::CanvasView;
use crate::ui::shell::ShellAction;
use crate::ui::thread_panel::colours_for_count;

fn bare(m: &Modifiers) -> bool {
    !m.shift && !m.command && !m.ctrl && !m.alt && !m.mac_cmd
}

fn arrow_claimed(store: &Store, key: Key) -> bool {
    let e = &store.editor;
    let horizontal = matches!(key, Key::ArrowLeft | Key::ArrowRight);
    (e.mode == EditorMode::Select && e.select_tool != SelectTool::Select)
        || (e.mode == EditorMode::Thread && e.thread_draft.is_some() && horizontal)
        || (e.mode == EditorMode::Pin
            && e.pin_tool == PinTool::Polygon
            && e.polygon_draft.is_some()
            && key == Key::ArrowLeft)
        || (e.mode == EditorMode::Thread && e.two_pin_draft.is_some() && key == Key::ArrowLeft)
        || (e.mode == EditorMode::Thread && e.repeat_draft.is_some() && key == Key::ArrowLeft)
}

fn handle_pin(store: &mut Store, key: Key, m: &Modifiers) -> bool {
    if m.shift && !m.command && !m.alt && key == Key::S {
        let current = store.active_symmetry().kind();
        let cycle = SYMMETRY_TYPE_CYCLE_REF;
        let i = cycle.iter().position(|t| *t == current).unwrap_or(0);
        store.set_symmetry_config(SymmetryConfig::for_type(cycle[(i + 1) % cycle.len()]));
        return true;
    }
    if !bare(m) {
        return false;
    }
    let tool = match key {
        Key::L => PinTool::Line,
        Key::A => PinTool::Arc,
        Key::E => PinTool::Ellipse,
        Key::C => PinTool::Circle,
        Key::R => PinTool::Rectangle,
        Key::S => PinTool::Square,
        Key::F => PinTool::Freehand,
        Key::P => PinTool::Polygon,
        Key::T => PinTool::Text,
        Key::D => PinTool::Eraser,
        Key::Q => PinTool::PathEraser,
        _ => return false,
    };
    store.set_pin_tool(tool);
    true
}

fn handle_edit(store: &mut Store, key: Key, m: &Modifiers) -> bool {
    if m.shift && !m.command && !m.alt && key == Key::P {
        let g = if store.editor.select_granularity == SelectGranularity::Path {
            SelectGranularity::Pins
        } else {
            SelectGranularity::Path
        };
        store.set_select_granularity(g);
        return true;
    }
    if !bare(m) {
        return false;
    }
    let tool = match key {
        Key::S => SelectTool::Select,
        Key::M => SelectTool::Move,
        Key::R => SelectTool::Rotate,
        Key::C => SelectTool::Scale,
        Key::J => {
            store.commit_selection_merge();
            return true;
        }
        _ => return false,
    };
    store.set_select_tool(tool);
    true
}

fn handle_thread(store: &mut Store, key: Key, m: &Modifiers) -> bool {
    if bare(m) {
        let tool = match key {
            Key::D => ThreadTool::Draw,
            Key::S => ThreadTool::Select,
            Key::E => ThreadTool::Eraser,
            Key::C => ThreadTool::SegmentEraser,
            Key::Z => ThreadTool::Zigzag,
            Key::P => ThreadTool::Parabolic,
            Key::R => ThreadTool::Radial,
            Key::T => ThreadTool::Repeat,
            _ => return false,
        };
        store.set_thread_tool(tool);
        return true;
    }
    if m.shift && !m.command && !m.alt {
        let (colours, width) = match store.selected_thread_path() {
            Some(t) => (t.colours.clone(), t.width),
            None => (
                store.editor.thread_defaults.colours.clone(),
                store.editor.thread_defaults.width,
            ),
        };
        let n = match key {
            Key::Num1 => Some(1),
            Key::Num2 => Some(2),
            Key::Num3 => Some(3),
            _ => None,
        };
        if let Some(n) = n {
            store.set_thread_property(
                "kbd-colours",
                ThreadPropertyPatch {
                    colours: Some(colours_for_count(&colours, n)),
                    ..Default::default()
                },
            );
            return true;
        }
        match key {
            Key::Plus | Key::Equals => {
                store.set_thread_property(
                    "kbd-width",
                    ThreadPropertyPatch {
                        width: Some((width + 0.5).min(5.0)),
                        ..Default::default()
                    },
                );
                return true;
            }
            Key::Minus => {
                store.set_thread_property(
                    "kbd-width",
                    ThreadPropertyPatch {
                        width: Some((width - 0.5).max(0.5)),
                        ..Default::default()
                    },
                );
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Processes this frame's key presses. Returns a shell action (Save, Open,
/// New, Help) for the app to run.
pub fn dispatch(
    ctx: &egui::Context,
    store: &mut Store,
    canvas: &mut CanvasView,
) -> Option<ShellAction> {
    if ctx.egui_wants_keyboard_input() {
        return None;
    }
    let keys: Vec<(Key, Modifiers)> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    });
    // Copy / paste (spec 39): the clipboard carries the web-compatible envelope.
    let (copy, paste) = ctx.input(|i| {
        let copy = i.events.iter().any(|e| matches!(e, Event::Copy));
        let paste = i.events.iter().find_map(|e| match e {
            Event::Paste(t) => Some(t.clone()),
            _ => None,
        });
        (copy, paste)
    });
    if copy && let Some(p) = store.copy_selection() {
        ctx.copy_text(p.to_json());
    }
    if let Some(text) = paste
        && let Some(p) = sa_app::clipboard::parse_clipboard(&text)
    {
        store.paste_clipboard(&p);
    }
    let mut action = None;
    for (key, m) in keys {
        let primary = m.command;
        // Undo/redo (spec 10); with a thread draft, Cmd+Z retracts its last vertex.
        if primary && key == Key::Z {
            if m.shift {
                store.redo();
            } else if store.editor.thread_draft.is_some() {
                store.retract_thread_draft();
            } else {
                store.undo();
            }
            continue;
        }
        let handled = match store.editor.mode {
            EditorMode::Pin => handle_pin(store, key, &m),
            EditorMode::Select => handle_edit(store, key, &m),
            EditorMode::Thread => handle_thread(store, key, &m),
            _ => false,
        };
        if handled {
            continue;
        }
        // Arrow pan fallback: content follows the key (20 px, Shift 100 px).
        if matches!(
            key,
            Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight
        ) && !primary
            && !m.alt
            && !arrow_claimed(store, key)
        {
            let step = if m.shift { 100.0 } else { 20.0 };
            let (dx, dy) = match key {
                Key::ArrowLeft => (step, 0.0),
                Key::ArrowRight => (-step, 0.0),
                Key::ArrowUp => (0.0, step),
                _ => (0.0, -step),
            };
            let v = &mut store.editor.viewport;
            v.pan_origin = Point::new(v.pan_origin.x + dx / v.zoom, v.pan_origin.y + dy / v.zoom);
            continue;
        }
        match key {
            Key::N if primary && m.alt => action = Some(ShellAction::New),
            Key::S if primary && !m.shift && !m.alt => action = Some(ShellAction::Save),
            Key::O if primary && !m.shift && !m.alt => action = Some(ShellAction::Open),
            Key::Delete | Key::Backspace if bare(&m) => store.delete_selection(),
            Key::Questionmark if !primary => action = Some(ShellAction::Help),
            Key::G if primary && m.shift => store.editor.grid.visible = !store.editor.grid.visible,
            Key::A if primary && m.shift => {
                store.editor.grid.snap_enabled = !store.editor.grid.snap_enabled
            }
            Key::Plus | Key::Equals if !primary && !m.alt => canvas.zoom_in(store),
            Key::Minus if !primary && !m.alt => canvas.zoom_out(store),
            Key::Num0 if bare(&m) => canvas.request_fit(),
            Key::Num1 | Key::Num2 | Key::Num3 | Key::Num4 | Key::Num5 | Key::Num6 if bare(&m) => {
                let i = match key {
                    Key::Num1 => 0,
                    Key::Num2 => 1,
                    Key::Num3 => 2,
                    Key::Num4 => 3,
                    Key::Num5 => 4,
                    _ => 5,
                };
                store.set_mode(EditorMode::ORDER[i]);
            }
            _ => {}
        }
    }
    action
}
