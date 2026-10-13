//! Edit-mode interaction through real egui input events.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use sa_app::editor::{EditorMode, SelectTool, Selection};
use string_art_it::app::StringArtApp;

fn editor() -> Harness<'static, StringArtApp> {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_eframe(|cc| StringArtApp::with_context(&cc.egui_ctx));
    h.state_mut().set_language_code("en");
    h.state_mut().enter_editor();
    h.run_steps(3);
    h
}

fn drag(h: &mut Harness<'_, StringArtApp>, from: Pos2, to: Pos2) {
    let mut ev = |e: Event| {
        h.input_mut().events.push(e);
        h.step();
    };
    ev(Event::PointerMoved(from));
    ev(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    for i in 1..=5 {
        ev(Event::PointerMoved(from + (to - from) * (i as f32 / 5.0)));
    }
    ev(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
}

#[test]
fn click_selects_then_move_drag_translates_and_undo_restores() {
    let mut h = editor();
    drag(&mut h, Pos2::new(694.0, 488.0), Pos2::new(780.0, 488.0));
    h.state_mut().store_mut().set_mode(EditorMode::Select);
    h.run_steps(2);
    // click on the right-most pin (start of the circle)
    drag(&mut h, Pos2::new(780.0, 488.0), Pos2::new(780.0, 488.0));
    assert!(
        matches!(h.state_mut().store_mut().editor.selection, Selection::PinPaths(ref r) if r.len() == 1)
    );

    h.state_mut().store_mut().set_select_tool(SelectTool::Move);
    let before = h.state_mut().store_mut().doc.pin_layers[0].pin_paths[0].pins[0].x;
    drag(&mut h, Pos2::new(700.0, 400.0), Pos2::new(760.0, 400.0));
    let after = h.state_mut().store_mut().doc.pin_layers[0].pin_paths[0].pins[0].x;
    assert!(after > before + 1.0, "moved right: {before} → {after}");
    h.state_mut().store_mut().undo();
    let restored = h.state_mut().store_mut().doc.pin_layers[0].pin_paths[0].pins[0].x;
    assert!((restored - before).abs() < 1e-9);
}

#[test]
fn rubber_band_selects_and_arrow_nudges() {
    let mut h = editor();
    drag(&mut h, Pos2::new(694.0, 488.0), Pos2::new(780.0, 488.0));
    h.state_mut().store_mut().set_mode(EditorMode::Select);
    h.run_steps(2);
    drag(&mut h, Pos2::new(560.0, 360.0), Pos2::new(820.0, 620.0));
    assert!(
        matches!(h.state_mut().store_mut().editor.selection, Selection::PinPaths(ref r) if r.len() == 1)
    );
    h.state_mut().store_mut().set_select_tool(SelectTool::Move);
    let before = h.state_mut().store_mut().doc.pin_layers[0].pin_paths[0].pins[0].x;
    h.input_mut().events.push(Event::Key {
        key: Key::ArrowRight,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::SHIFT,
    });
    h.step();
    h.input_mut().events.push(Event::Key {
        key: Key::ArrowRight,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::SHIFT,
    });
    h.run_steps(2);
    let store = h.state_mut().store_mut();
    let after = store.doc.pin_layers[0].pin_paths[0].pins[0].x;
    let expected = 10.0 / store.editor.viewport.zoom;
    assert!(
        (after - before - expected).abs() < 1e-9,
        "{before} → {after}, expected +{expected}"
    );
}
