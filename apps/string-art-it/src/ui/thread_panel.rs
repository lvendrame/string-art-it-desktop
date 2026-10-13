//! Thread mode side panel: tools, pin-state legend, thread properties and
//! per-tool settings (specs 12, 27, 35, 37, 38).

use egui::Ui;
use sa_app::document::layers::pin_index;
use sa_app::editor::{Store, ThreadTool};
use sa_app::editor_threads::ThreadPropertyPatch;
use sa_app::threads::max_repeat_colours;

use crate::fonts;
use crate::icons::Icon;
use crate::tools::pin::CANDIDATE;
use crate::widgets::{self, Kit};

pub const PALETTE: [&str; 5] = ["#5b8def", "#edeff7", "#e8b449", "#d96c6c", "#8fd6c8"];

pub fn colours_for_count(colours: &[String], n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            colours
                .get(i)
                .cloned()
                .unwrap_or_else(|| PALETTE[i % PALETTE.len()].to_owned())
        })
        .collect()
}

pub const THREAD_TOOLS: [(ThreadTool, Icon, &str, &str); 8] = [
    (ThreadTool::Draw, Icon::PenLine, "draw", "D"),
    (ThreadTool::Zigzag, Icon::Waypoints, "zigzag", "Z"),
    (ThreadTool::Parabolic, Icon::Croissant, "parabolic", "P"),
    (ThreadTool::Radial, Icon::LoaderPinwheel, "radial", "R"),
    (ThreadTool::Repeat, Icon::Galaxy, "repeat", "T"),
    (ThreadTool::Select, Icon::MousePointer2, "select", "S"),
    (ThreadTool::Eraser, Icon::Eraser, "eraser", "E"),
    (ThreadTool::SegmentEraser, Icon::Scissors, "segment", "C"),
];

pub fn thread_panel(ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
    ui.spacing_mut().item_spacing.y = 8.0;
    widgets::section_label(ui, kit, &kit.tr("toolbars:threadToolbar.sectionTitle"));
    let items: Vec<(Icon, String)> = THREAD_TOOLS
        .iter()
        .map(|(_, icon, key, sc)| {
            (
                *icon,
                format!(
                    "{} [{sc}]",
                    kit.tr(&format!("toolbars:threadToolbar.{key}"))
                ),
            )
        })
        .collect();
    let selected = THREAD_TOOLS
        .iter()
        .position(|(t, ..)| *t == store.editor.thread_tool);
    if let Some(i) = widgets::tool_grid(ui, kit, &items, selected) {
        store.set_thread_tool(THREAD_TOOLS[i].0);
    }
    legend(ui, kit);
    ui.add_space(10.0);
    properties(ui, kit, store);
}

fn legend(ui: &mut Ui, kit: &Kit) {
    let rows: [(&str, u8); 4] = [
        ("toolbars:threadToolbar.legend.normal", 0),
        ("toolbars:threadToolbar.legend.nearestCandidate", 1),
        ("toolbars:threadToolbar.legend.activeOrigin", 2),
        ("toolbars:threadToolbar.legend.usedInThisThread", 3),
    ];
    for (key, kind) in rows {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            let c = r.center();
            let p = ui.painter();
            match kind {
                0 => {
                    p.circle_filled(c, 4.0, kit.t.text_secondary);
                }
                1 => {
                    p.circle_stroke(c, 7.0, egui::Stroke::new(1.5, CANDIDATE));
                    p.circle_filled(c, 3.0, CANDIDATE);
                }
                2 => {
                    p.circle_filled(c, 4.0, kit.t.accent);
                }
                _ => {
                    p.circle_filled(c, 4.0, kit.t.accent.gamma_multiply(0.5));
                }
            }
            ui.label(
                egui::RichText::new(kit.tr(key))
                    .size(11.5)
                    .color(kit.t.text_secondary),
            );
        });
    }
}

fn properties(ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
    let selected = store.selected_thread_path().cloned();
    let title = if selected.is_some() {
        "panels:threadPropertiesPanel.titleSelected"
    } else {
        "panels:threadPropertiesPanel.titleDefaults"
    };
    widgets::section_label(ui, kit, &kit.tr(title));
    let (colours, width, pitch) = match &selected {
        Some(t) => (t.colours.clone(), t.width, t.twist_pitch),
        None => {
            let d = &store.editor.thread_defaults;
            (d.colours.clone(), d.width, d.twist_pitch)
        }
    };
    let tool = store.editor.thread_tool;
    let show_strands = tool != ThreadTool::Repeat;
    if show_strands {
        widgets::section_label(
            ui,
            kit,
            &kit.tr("panels:threadPropertiesPanel.coloursSectionTitle"),
        );
        ui.horizontal(|ui| {
            for n in 1..=3 {
                if widgets::text_button(ui, kit, &n.to_string(), colours.len() == n, Some(40.0))
                    .clicked()
                {
                    store.set_thread_property(
                        "thread-colour-count",
                        ThreadPropertyPatch {
                            colours: Some(colours_for_count(&colours, n)),
                            ..Default::default()
                        },
                    );
                }
            }
        });
        ui.horizontal(|ui| {
            for (i, c) in colours.iter().enumerate() {
                if let Some(nc) = widgets::colour(ui, c) {
                    let mut v = colours.clone();
                    v[i] = nc;
                    store.set_thread_property(
                        "thread-colours",
                        ThreadPropertyPatch {
                            colours: Some(v),
                            ..Default::default()
                        },
                    );
                }
            }
        });
    }
    if let Some(w) = widgets::slider_field(
        ui,
        kit,
        &kit.tr("panels:threadPropertiesPanel.width"),
        width,
        0.5..=5.0,
        0.5,
        true,
        |v| format!("{v}"),
    ) {
        store.set_thread_property(
            "thread-width",
            ThreadPropertyPatch {
                width: Some(w),
                ..Default::default()
            },
        );
    }
    if show_strands && colours.len() > 1 {
        widgets::property_row(
            ui,
            kit,
            &kit.tr("panels:threadPropertiesPanel.twistPitch"),
            |ui| {
                if let Some(v) = widgets::number(ui, kit, pitch, 2.0..=20.0, 0.1, 0) {
                    store.set_thread_property(
                        "thread-pitch",
                        ThreadPropertyPatch {
                            twist_pitch: Some(v.round()),
                            ..Default::default()
                        },
                    );
                }
            },
        );
    }

    let e = &mut store.editor;
    let int = |v: f64| format!("{v:.0}");
    match tool {
        ThreadTool::Zigzag | ThreadTool::Parabolic => {
            let parabolic = tool == ThreadTool::Parabolic;
            let title = if parabolic {
                "panels:threadPropertiesPanel.parabolicSectionTitle"
            } else {
                "panels:threadPropertiesPanel.zigzagSectionTitle"
            };
            widgets::section_label(ui, kit, &kit.tr(title));
            let s = if parabolic {
                &mut e.parabolic_settings
            } else {
                &mut e.zigzag_settings
            };
            if let Some(v) = widgets::slider_field(
                ui,
                kit,
                &kit.tr("panels:threadPropertiesPanel.stepA"),
                s.step_a,
                0.0..=9.0,
                1.0,
                true,
                int,
            ) {
                s.step_a = v;
            }
            if let Some(v) = widgets::slider_field(
                ui,
                kit,
                &kit.tr("panels:threadPropertiesPanel.stepB"),
                s.step_b,
                0.0..=9.0,
                1.0,
                true,
                int,
            ) {
                s.step_b = v;
            }
            if let Some(v) = widgets::checkbox_field(
                ui,
                &kit.tr("panels:threadPropertiesPanel.fullFill"),
                s.full_fill,
                true,
            ) {
                s.full_fill = v;
            }
            if parabolic
                && let Some(v) = widgets::slider_field(
                    ui,
                    kit,
                    &kit.tr("panels:threadPropertiesPanel.cycles"),
                    s.cycles,
                    1.0..=20.0,
                    1.0,
                    s.full_fill,
                    int,
                )
            {
                s.cycles = v;
            }
        }
        ThreadTool::Radial => {
            widgets::section_label(
                ui,
                kit,
                &kit.tr("panels:threadPropertiesPanel.radialSectionTitle"),
            );
            if let Some(v) = widgets::slider_field(
                ui,
                kit,
                &kit.tr("panels:threadPropertiesPanel.step"),
                e.radial_step,
                1.0..=10.0,
                1.0,
                true,
                int,
            ) {
                e.radial_step = v;
            }
        }
        ThreadTool::Repeat => {
            widgets::section_label(
                ui,
                kit,
                &kit.tr("panels:threadPropertiesPanel.repeatSectionTitle"),
            );
            let s = &mut e.repeat_settings;
            widgets::property_row(
                ui,
                kit,
                &kit.tr("panels:threadPropertiesPanel.cycles"),
                |ui| {
                    ui.add_enabled_ui(!s.full_fill, |ui| {
                        if let Some(v) = widgets::number(ui, kit, s.cycles, 1.0..=20.0, 0.1, 0) {
                            s.cycles = v.round().clamp(1.0, 20.0);
                        }
                    });
                },
            );
            if let Some(v) = widgets::checkbox_field(
                ui,
                &kit.tr("panels:threadPropertiesPanel.fullFill"),
                s.full_fill,
                true,
            ) {
                s.full_fill = v;
            }
            let max = max_repeat_colours(s.cycles as usize);
            let shown: Vec<String> = s.colours.iter().take(max).cloned().collect();
            let note = kit.trf(
                "panels:threadPropertiesPanel.repeatColoursMax",
                &[("max", &max)],
            );
            if let Some(v) = widgets::colour_list_field(
                ui,
                kit,
                &kit.tr("panels:threadPropertiesPanel.coloursSectionTitle"),
                &shown,
                max,
                &PALETTE,
                Some(&note),
            ) {
                store.editor.repeat_settings.colours = v;
            }
        }
        _ => {}
    }

    if let Some(t) = selected {
        widgets::section_label(ui, kit, &kit.tr("panels:threadPropertiesPanel.stats.title"));
        let index = pin_index(&store.doc.pin_layers);
        let length: f64 = t
            .pin_ids
            .windows(2)
            .filter_map(|w| {
                Some(
                    index
                        .get(&w[0])?
                        .point()
                        .distance(index.get(&w[1])?.point()),
                )
            })
            .sum();
        for (k, v) in [
            ("segments", t.segment_count().to_string()),
            ("threadLength", format!("{:.2} m", length / 100.0)),
            ("pinsVisited", t.pin_ids.len().to_string()),
        ] {
            widgets::property_row(
                ui,
                kit,
                &kit.tr(&format!("panels:threadPropertiesPanel.stats.{k}")),
                |ui| {
                    ui.label(
                        egui::RichText::new(v)
                            .font(fonts::mono(12.0))
                            .color(kit.t.text_primary),
                    );
                },
            );
        }
    }
}
