//! Pin-mode interaction through real egui input events.

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
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
    let ev = |e: Event, h: &mut Harness<'_, StringArtApp>| {
        h.input_mut().events.push(e);
        h.step();
    };
    ev(Event::PointerMoved(from), h);
    ev(
        Event::PointerButton {
            pos: from,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        h,
    );
    for i in 1..=5 {
        let t = i as f32 / 5.0;
        ev(Event::PointerMoved(from + (to - from) * t), h);
    }
    ev(
        Event::PointerButton {
            pos: to,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
        h,
    );
    h.run_steps(2);
}

#[test]
fn dragging_with_circle_tool_creates_a_pin_path() {
    let mut h = editor();
    drag(&mut h, Pos2::new(694.0, 488.0), Pos2::new(780.0, 488.0));
    let store = h.state_mut().store_mut();
    let paths = &store.doc.pin_layers[0].pin_paths;
    assert_eq!(paths.len(), 1, "one circle drawn");
    assert!(!paths[0].pins.is_empty());
    assert!(store.can_undo());
}

#[test]
fn eraser_removes_clicked_pin() {
    let mut h = editor();
    drag(&mut h, Pos2::new(694.0, 488.0), Pos2::new(780.0, 488.0));
    let before = h.state_mut().store_mut().doc.pin_layers[0].pin_paths[0]
        .pins
        .len();
    h.state_mut()
        .store_mut()
        .set_pin_tool(sa_app::editor::PinTool::Eraser);
    // first pin of a circle sits at angle 0 → right-most point, at the drag end
    drag(&mut h, Pos2::new(780.0, 488.0), Pos2::new(780.0, 488.0));
    let after = h.state_mut().store_mut().doc.pin_layers[0].pin_paths[0]
        .pins
        .len();
    assert_eq!(after, before - 1);
}
