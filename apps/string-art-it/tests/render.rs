//! Renders key screens to PNG (GPU via wgpu) for visual review. Output goes to
//! `$STRINGARTIT_SNAPSHOT_DIR` (defaults to `target/ui-snapshots`).

use egui_kittest::Harness;
use string_art_it::app::StringArtApp;

fn out_dir() -> std::path::PathBuf {
    let dir = std::env::var("STRINGARTIT_SNAPSHOT_DIR")
        .map(Into::into)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/ui-snapshots")
        });
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn harness(setup: impl FnOnce(&mut StringArtApp)) -> Harness<'static, StringArtApp> {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .build_eframe(|cc| StringArtApp::with_context(&cc.egui_ctx));
    setup(h.state_mut());
    h.run_steps(4);
    h
}

fn save(h: &mut Harness<'_, StringArtApp>, name: &str) {
    match h.render() {
        Ok(img) => img.save(out_dir().join(format!("{name}.png"))).unwrap(),
        Err(e) => eprintln!("render skipped (no GPU adapter?): {e}"),
    }
}

#[test]
fn board_setup_screen() {
    let mut h = harness(|app| app.set_language_code("en"));
    save(&mut h, "board-setup");
}

#[test]
fn editor_wood_board() {
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.store_mut()
            .set_board_appearance(sa_app::document::BoardAppearance::WoodTexture {
                preset_id: "oak".into(),
            });
        app.enter_editor();
    });
    save(&mut h, "editor-wood");
}

#[test]
fn editor_pins_with_symmetry() {
    use sa_app::document::{PinPathGeometry, SymmetryConfig};
    use sa_domain::Point;
    let mut h = harness(|app| {
        app.set_language_code("en");
        let store = app.store_mut();
        store.set_symmetry_config(SymmetryConfig::Radial {
            centre: Point::ORIGIN,
            interval_degrees: 60.0,
        });
        let layer = store.editor.active_pin_layer_id.clone();
        store.add_pin_path(
            &layer,
            PinPathGeometry::Star {
                center: Point::new(14.0, 0.0),
                outer_radius: 8.0,
                inner_radius: 4.0,
                points: 5,
                rotation: 0.0,
            },
        );
        app.enter_editor();
    });
    save(&mut h, "editor-pins");
}

#[test]
fn editor_edit_mode_selection() {
    use sa_app::document::PinPathGeometry;
    use sa_app::editor::{EditorMode, PinPathRef, Selection};
    use sa_domain::Point;
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.enter_editor();
        let store = app.store_mut();
        let layer = store.editor.active_pin_layer_id.clone();
        let id = store
            .add_pin_path(
                &layer,
                PinPathGeometry::Star {
                    center: Point::new(0.0, 0.0),
                    outer_radius: 20.0,
                    inner_radius: 9.0,
                    points: 6,
                    rotation: 0.0,
                },
            )
            .unwrap();
        store.set_mode(EditorMode::Select);
        store.select(Selection::PinPaths(vec![PinPathRef {
            layer_id: layer,
            path_id: id,
        }]));
    });
    save(&mut h, "editor-edit");
}

#[test]
fn editor_thread_mode() {
    use sa_app::document::{PinPathGeometry, ThreadPath};
    use sa_app::editor::EditorMode;
    use sa_domain::Point;
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.enter_editor();
        let store = app.store_mut();
        let layer = store.editor.active_pin_layer_id.clone();
        store.editor.pin_defaults.spacing = 3.0;
        let id = store
            .add_pin_path(
                &layer,
                PinPathGeometry::Circle {
                    center: Point::ORIGIN,
                    radius: 25.0,
                },
            )
            .unwrap();
        let pins: Vec<String> = store
            .find_pin_path(&layer, &id)
            .unwrap()
            .pins
            .iter()
            .map(|p| p.id.clone())
            .collect();
        let n = pins.len();
        let ids: Vec<String> = (0..60).map(|k| pins[(k * 17) % n].clone()).collect();
        store.commit(|d| {
            d.thread_layers[0].thread_paths.push(ThreadPath::new(
                ids,
                vec!["#5b8def".into(), "#edeff7".into()],
                1.5,
                6.0,
            ))
        });
        store.set_mode(EditorMode::Thread);
    });
    save(&mut h, "editor-thread");
}

#[test]
fn editor_layers_panel() {
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.enter_editor();
        let store = app.store_mut();
        store.add_pin_layer();
        store.add_pin_layer();
        let id = store.doc.pin_layers[1].id.clone();
        store.toggle_pin_layer_locked(&id);
        store.toggle_pin_layer_visible(&id);
    });
    save(&mut h, "editor-layers");
}

#[test]
fn radial_menu_open() {
    use egui::{Event, Modifiers, PointerButton, Pos2};
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.enter_editor();
    });
    let at = Pos2::new(700.0, 480.0);
    for e in [
        Event::PointerMoved(at),
        Event::PointerButton {
            pos: at,
            button: PointerButton::Secondary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos: at,
            button: PointerButton::Secondary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
        Event::PointerMoved(Pos2::new(700.0, 414.0)),
    ] {
        h.input_mut().events.push(e);
        h.step();
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
    h.run_steps(3);
    save(&mut h, "radial-menu");
}

#[test]
fn print_preview_tiled() {
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.enter_editor();
        let store = app.store_mut();
        store.set_board_appearance(sa_app::document::BoardAppearance::WoodTexture {
            preset_id: "oak".into(),
        });
        let layer = store.editor.active_pin_layer_id.clone();
        store.editor.pin_defaults.spacing = 4.0;
        store.add_pin_path(
            &layer,
            sa_app::document::PinPathGeometry::Circle {
                center: sa_domain::Point::ORIGIN,
                radius: 25.0,
            },
        );
        store.editor.print.scale_mode = sa_app::print::ScaleMode::Actual;
        store.editor.print.tiling.enabled = true;
        store.editor.print.elements.pin_numbers = true;
        app.open_print_preview();
    });
    save(&mut h, "print-preview");
}

#[test]
fn play_mode_mid_frame() {
    use sa_app::document::{PinPathGeometry, ThreadPath};
    use sa_app::editor::EditorMode;
    use sa_domain::Point;
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.enter_editor();
        let store = app.store_mut();
        let layer = store.editor.active_pin_layer_id.clone();
        store.editor.pin_defaults.spacing = 3.0;
        let id = store
            .add_pin_path(
                &layer,
                PinPathGeometry::Circle {
                    center: Point::ORIGIN,
                    radius: 25.0,
                },
            )
            .unwrap();
        let pins: Vec<String> = store
            .find_pin_path(&layer, &id)
            .unwrap()
            .pins
            .iter()
            .map(|p| p.id.clone())
            .collect();
        let n = pins.len();
        let ids: Vec<String> = (0..60).map(|k| pins[(k * 17) % n].clone()).collect();
        store.commit(|d| {
            d.thread_layers[0].thread_paths.push(ThreadPath::new(
                ids,
                vec!["#e8b449".into()],
                1.5,
                6.0,
            ))
        });
        store.set_mode(EditorMode::Play);
        app.set_play_frame(25);
    });
    save(&mut h, "play-mode");
}

#[test]
fn generator_preview() {
    use sa_app::editor::EditorMode;
    use sa_app::generator::GeneratorParams;
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.enter_editor();
        let store = app.store_mut();
        store.set_mode(EditorMode::Generate);
        store.generate_pattern(
            GeneratorParams::Mandala {
                n: 180.0,
                base: 3.0,
                layers: 2.0,
            },
            vec!["#e8b449".into(), "#5b8def".into()],
        );
    });
    save(&mut h, "generator");
}

#[test]
fn generator_lotus_confirmed() {
    use sa_app::editor::EditorMode;
    use sa_app::generator::PatternId;
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.enter_editor();
        let store = app.store_mut();
        store.set_mode(EditorMode::Generate);
        store.generate_pattern(
            PatternId::Lotus.default_params(),
            vec!["#e8b449".into(), "#d9534f".into(), "#5b8def".into()],
        );
        store.confirm_generated_pattern();
        store.set_mode(EditorMode::Pin);
    });
    save(&mut h, "generator-lotus");
}

fn generated_doc(app: &mut StringArtApp) {
    use sa_app::editor::EditorMode;
    use sa_app::generator::PatternId;
    app.set_language_code("en");
    app.enter_editor();
    let store = app.store_mut();
    store.set_mode(EditorMode::Generate);
    store.generate_pattern(
        PatternId::Star.default_params(),
        vec!["#e8b449".into(), "#5b8def".into()],
    );
    store.confirm_generated_pattern();
    store.set_mode(EditorMode::Pin);
}

#[test]
fn statistics_overlay() {
    let mut h = harness(|app| {
        generated_doc(app);
        app.open_overlay_for_test("stats");
    });
    save(&mut h, "stats");
}

#[test]
fn help_overlay() {
    let mut h = harness(|app| {
        generated_doc(app);
        app.open_overlay_for_test("help");
    });
    save(&mut h, "help");
    let mut h = harness(|app| {
        generated_doc(app);
        app.open_overlay_for_test("help:about");
    });
    save(&mut h, "help-about");
}

#[test]
fn cjk_locales() {
    for code in ["ja", "zh-CN", "ko"] {
        let mut h = harness(|app| {
            generated_doc(app);
            app.set_language_code(code);
            app.open_overlay_for_test("help");
        });
        save(&mut h, &format!("cjk-{code}"));
    }
}

#[test]
fn agent_popover() {
    let mut h = harness(|app| {
        app.set_language_code("en");
        app.enter_editor();
    });
    let ctx = h.ctx.clone();
    h.state_mut().start_agent_for_test(&ctx, 0).unwrap();
    h.run_steps(2);
    // Bot button sits left of "Stats" in the top bar.
    let pos = egui::pos2(1077.0, 27.0);
    h.event(egui::Event::PointerMoved(pos));
    h.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    h.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    h.run_steps(3);
    save(&mut h, "agent-popover");
}
