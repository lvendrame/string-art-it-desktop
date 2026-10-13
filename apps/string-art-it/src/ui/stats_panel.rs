//! Statistics overlay (spec 17): summary by thread type + per-path details.

use egui::{Align2, CornerRadius, Ui, pos2, vec2};
use sa_app::editor::Store;
use sa_app::statistics::{
    pin_path_statistics, project_total_pins, thread_path_statistics, thread_statistics_by_type,
};
use sa_render::Rgba;

use crate::canvas::paint::c32;
use crate::fonts;
use crate::theme;
use crate::widgets::Kit;

enum Value {
    Text(String),
    Colours(Vec<String>),
}

pub fn show(ui: &mut Ui, kit: &mut Kit, store: &Store) -> bool {
    let title = kit.tr("panels:statisticsPanel.title");
    super::overlay::show(ui, kit, &title, |ui, kit| body(ui, kit, store))
}

fn tr(kit: &Kit, k: &str) -> String {
    kit.tr(&format!("panels:statisticsPanel.{k}"))
}

enum Card {
    Total(usize),
    Stat(Value, Vec<(String, Value)>),
}

fn body(ui: &mut Ui, kit: &mut Kit, store: &Store) {
    let doc = &store.doc;
    ui.set_max_width(1000.0);
    ui.spacing_mut().item_spacing.y = 16.0;
    let mut summary = vec![Card::Total(project_total_pins(&doc.pin_layers))];
    for s in thread_statistics_by_type(&doc.thread_layers, &doc.pin_layers) {
        summary.push(Card::Stat(
            Value::Colours(s.colours.clone()),
            vec![
                (tr(kit, "width"), Value::Text(fmt_num(s.width))),
                (
                    tr(kit, "threadCount"),
                    Value::Text(s.thread_count.to_string()),
                ),
                (
                    tr(kit, "segments"),
                    Value::Text(s.total_segments.to_string()),
                ),
                (
                    tr(kit, "threadLength"),
                    Value::Text(format!("{:.2} m", s.total_length_cm / 100.0)),
                ),
                (
                    tr(kit, "pinsVisited"),
                    Value::Text(s.total_pins_visited.to_string()),
                ),
            ],
        ));
    }
    section(ui, kit, "summary", &tr(kit, "summaryTitle"), true, &summary);

    let mut details = Vec::new();
    for layer in &doc.pin_layers {
        for (i, path) in layer.pin_paths.iter().enumerate() {
            let s = pin_path_statistics(path);
            let title = kit.trf(
                "panels:statisticsPanel.pathTitle",
                &[("layerName", &layer.name), ("index", &(i + 1))],
            );
            details.push(Card::Stat(
                Value::Text(title),
                vec![
                    (tr(kit, "pins"), Value::Text(s.pins.to_string())),
                    (
                        tr(kit, "requestedGap"),
                        Value::Text(format!("{:.2} cm", s.requested_spacing)),
                    ),
                    (
                        tr(kit, "actualGap"),
                        Value::Text(format!("{:.2} cm", s.actual_spacing)),
                    ),
                    (
                        tr(kit, "pinDiameter"),
                        Value::Text(format!("{} mm", fmt_num(s.diameter))),
                    ),
                ],
            ));
        }
    }
    for layer in &doc.thread_layers {
        for (i, th) in layer.thread_paths.iter().enumerate() {
            let s = thread_path_statistics(th, &doc.pin_layers);
            let title = kit.trf(
                "panels:statisticsPanel.threadTitle",
                &[("layerName", &layer.name), ("index", &(i + 1))],
            );
            details.push(Card::Stat(
                Value::Text(title),
                vec![
                    (tr(kit, "segments"), Value::Text(s.segments.to_string())),
                    (
                        tr(kit, "threadLength"),
                        Value::Text(format!("{:.2} m", s.length_cm / 100.0)),
                    ),
                    (
                        tr(kit, "pinsVisited"),
                        Value::Text(s.pins_visited.to_string()),
                    ),
                    (tr(kit, "colours"), Value::Colours(s.colours)),
                ],
            ));
        }
    }
    section(
        ui,
        kit,
        "details",
        &tr(kit, "detailsTitle"),
        false,
        &details,
    );
}

/// JS `String(number)`-style formatting (no trailing zeros).
fn fmt_num(v: f64) -> String {
    let s = format!("{v}");
    s.strip_suffix(".0").map(str::to_owned).unwrap_or(s)
}

fn frame(kit: &Kit) -> egui::Frame {
    egui::Frame::new()
        .fill(kit.t.bg_panel)
        .stroke(egui::Stroke::new(1.0, kit.t.border))
        .corner_radius(CornerRadius::same(theme::RADIUS_MD))
        .inner_margin(16)
}

const GAP: f32 = 16.0;

/// `<details>`-style collapsible section with an auto-fill (min 260 px) card grid.
fn section(ui: &mut Ui, kit: &Kit, id: &str, title: &str, default_open: bool, cards: &[Card]) {
    frame(kit).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            ui.id().with(id),
            default_open,
        )
        .show_header(ui, |ui| {
            ui.label(
                egui::RichText::new(title)
                    .font(fonts::extrabold(13.0))
                    .color(kit.t.text_primary),
            );
        })
        .body_unindented(|ui| {
            ui.add_space(8.0);
            let w = ui.available_width();
            let cols = ((w + GAP) / (260.0 + GAP)).floor().max(1.0) as usize;
            let card_w = (w - GAP * (cols as f32 - 1.0)) / cols as f32;
            ui.spacing_mut().item_spacing = vec2(GAP, GAP);
            for row in cards.chunks(cols) {
                ui.horizontal_top(|ui| {
                    for c in row {
                        ui.allocate_ui_with_layout(
                            vec2(card_w, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                frame(kit).show(ui, |ui| {
                                    ui.set_width(card_w - 34.0);
                                    ui.spacing_mut().item_spacing.y = 8.0;
                                    card(ui, kit, c);
                                });
                            },
                        );
                    }
                });
            }
        });
    });
}

fn card(ui: &mut Ui, kit: &Kit, c: &Card) {
    match c {
        Card::Total(n) => {
            ui.label(
                egui::RichText::new(n.to_string())
                    .font(egui::FontId::monospace(24.0))
                    .color(kit.t.accent),
            );
            ui.label(
                egui::RichText::new(tr(kit, "totalPins"))
                    .size(12.0)
                    .color(kit.t.text_secondary),
            );
        }
        Card::Stat(title, rows) => {
            match title {
                Value::Text(t) => {
                    ui.label(
                        egui::RichText::new(t)
                            .font(fonts::extrabold(12.5))
                            .color(kit.t.text_primary),
                    );
                }
                Value::Colours(c) => {
                    let (r, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), 16.0),
                        egui::Sense::hover(),
                    );
                    paint_colours(ui, kit, c, r.left_center(), fonts::extrabold(12.5), false);
                }
            }
            for (label, value) in rows {
                let (r, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 16.0), egui::Sense::hover());
                let p = ui.painter();
                p.text(
                    r.left_center(),
                    Align2::LEFT_CENTER,
                    label,
                    fonts::mono(12.0),
                    kit.t.text_secondary,
                );
                match value {
                    Value::Text(t) => {
                        p.text(
                            r.right_center(),
                            Align2::RIGHT_CENTER,
                            t,
                            fonts::mono(12.0),
                            kit.t.text_primary,
                        );
                    }
                    Value::Colours(c) => {
                        paint_colours(ui, kit, c, r.right_center(), fonts::mono(12.0), true)
                    }
                }
            }
        }
    }
}

/// Swatch + hex per colour, laid out from `anchor` (rightwards, or leftwards when `from_right`).
fn paint_colours(
    ui: &Ui,
    kit: &Kit,
    list: &[String],
    anchor: egui::Pos2,
    font: egui::FontId,
    from_right: bool,
) {
    let p = ui.painter();
    let items: Vec<_> = list
        .iter()
        .map(|c| {
            (
                c,
                ui.fonts_mut(|f| f.layout_no_wrap(c.clone(), font.clone(), kit.t.text_primary)),
            )
        })
        .collect();
    let total: f32 = items
        .iter()
        .map(|(_, g)| 14.0 + 4.0 + g.size().x)
        .sum::<f32>()
        + 6.0 * items.len().saturating_sub(1) as f32;
    let mut x = if from_right {
        anchor.x - total
    } else {
        anchor.x
    };
    for (c, g) in items {
        let sw = egui::Rect::from_min_size(pos2(x, anchor.y - 5.0), vec2(14.0, 10.0));
        p.rect(
            sw,
            2.0,
            c32(Rgba::parse(c)),
            egui::Stroke::new(1.0, kit.t.border),
            egui::StrokeKind::Inside,
        );
        x += 18.0;
        let w = g.size().x;
        p.galley(pos2(x, anchor.y - g.size().y / 2.0), g, kit.t.text_primary);
        x += w + 6.0;
    }
}
