//! Thread-mode interaction through real egui input events.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use sa_app::document::PinPathGeometry;
use sa_app::editor::{EditorMode, ThreadTool};
use sa_domain::Point;
use sa_domain::transforms::to_screen;
use string_art_it::app::StringArtApp;

fn editor_with_circle() -> (Harness<'static, StringArtApp>, Vec<Point>) {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_eframe(|cc| StringArtApp::with_context(&cc.egui_ctx));
    h.state_mut().set_language_code("en");
    h.state_mut().enter_editor();
    let store = h.state_mut().store_mut();
    let layer = store.editor.active_pin_layer_id.clone();
    store.editor.pin_defaults.spacing = 5.0;
    store.add_pin_path(
        &layer,
        PinPathGeometry::Circle {
            center: Point::ORIGIN,
            radius: 20.0,
        },
    );
    store.set_mode(EditorMode::Thread);
    h.run_steps(3);
    let pts = h.state_mut().store_mut().doc.pin_layers[0].pin_paths[0]
        .pins
        .iter()
        .map(|p| p.point())
        .collect();
    (h, pts)
}

/// Screen position of a document point (canvas origin found via the viewport).
fn screen(h: &mut Harness<'_, StringArtApp>, p: Point) -> Pos2 {
    let (vp, origin) = h.state_mut().canvas_origin();
    let s = to_screen(p, &vp);
    Pos2::new(origin.x + s.x as f32, origin.y + s.y as f32)
}

fn click(h: &mut Harness<'_, StringArtApp>, at: Pos2) {
    for e in [
        Event::PointerMoved(at),
        Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
}

fn key(h: &mut Harness<'_, StringArtApp>, k: Key) {
    h.input_mut().events.push(Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.step();
    h.input_mut().events.push(Event::Key {
        key: k,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    h.step();
}

#[test]
fn click_pins_then_escape_commits_a_thread() {
    let (mut h, pins) = editor_with_circle();
    for i in [0, 8, 3] {
        let p = screen(&mut h, pins[i]);
        click(&mut h, p);
        h.step();
    }
    assert_eq!(
        h.state_mut()
            .store_mut()
            .editor
            .thread_draft
            .as_ref()
            .map(Vec::len),
        Some(3)
    );
    key(&mut h, Key::Escape);
    let store = h.state_mut().store_mut();
    assert_eq!(store.doc.thread_layers[0].thread_paths.len(), 1);
    assert_eq!(store.doc.thread_layers[0].thread_paths[0].pin_ids.len(), 3);
}

#[test]
fn zigzag_two_clicks_on_closed_path_then_resolve() {
    let (mut h, pins) = editor_with_circle();
    h.state_mut()
        .store_mut()
        .set_thread_tool(ThreadTool::Zigzag);
    let a = screen(&mut h, pins[0]);
    let b = screen(&mut h, pins[6]);
    click(&mut h, a);
    h.step();
    click(&mut h, b);
    h.step();
    let cands = h
        .state_mut()
        .store_mut()
        .editor
        .two_pin_draft
        .as_ref()
        .map(|d| d.candidates.len());
    assert_eq!(cands, Some(2), "closed path → two arc candidates");
    click(&mut h, b);
    let store = h.state_mut().store_mut();
    assert!(store.editor.two_pin_draft.is_none());
    assert_eq!(store.doc.thread_layers[0].thread_paths.len(), 1);
}
