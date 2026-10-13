//! Edit mode side panel: granularity switch, tools, Merge, Selection panel
//! (specs 09, 21, 26, 29).

use egui::Ui;
use sa_app::document::{FontWeight, PinPathGeometry};
use sa_app::editor::{SelectGranularity, SelectTool, Selection, Store};
use sa_app::editor_pins::PinPropertyPatch;
use sa_domain::Point;
use sa_infra::fonts::{FONT_CATALOG, build_text_geometry, font_entry};

use crate::fonts;
use crate::icons::Icon;
use crate::ui::pin_panel::symmetry_panel;
use crate::widgets::{self, Kit};

pub const SELECT_TOOLS: [(SelectTool, Icon, &str, &str); 4] = [
    (SelectTool::Select, Icon::MousePointer2, "select", "S"),
    (SelectTool::Move, Icon::Move, "move", "M"),
    (SelectTool::Rotate, Icon::RotateCw, "rotate", "R"),
    (SelectTool::Scale, Icon::Scaling, "scale", "C"),
];

pub fn select_panel(ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
    ui.spacing_mut().item_spacing.y = 8.0;
    widgets::section_label(ui, kit, &kit.tr("toolbars:selectToolbar.sectionTitle"));
    let is_pins = store.editor.select_granularity == SelectGranularity::Pins;
    if let Some(right) = widgets::slide_switch(
        ui,
        kit,
        &kit.tr("toolbars:selectToolbar.granularity.pinPath"),
        &kit.tr("toolbars:selectToolbar.granularity.pins"),
        is_pins,
    ) {
        store.set_select_granularity(if right {
            SelectGranularity::Pins
        } else {
            SelectGranularity::Path
        });
    }

    let has_selection = store.has_transformable_selection();
    let gap = 6.0;
    let w = ((ui.available_width() - gap) / 2.0).floor();
    for row in SELECT_TOOLS.chunks(2) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (tool, icon, key, sc) in row {
                let label = format!(
                    "{} [{sc}]",
                    kit.tr(&format!("toolbars:selectToolbar.tools.{key}.label"))
                );
                let enabled = *tool == SelectTool::Select || has_selection;
                ui.add_enabled_ui(enabled, |ui| {
                    let r = widgets::tool_button(
                        ui,
                        kit,
                        *icon,
                        &label,
                        store.editor.select_tool == *tool,
                        egui::vec2(w, 56.0),
                    )
                    .on_hover_text(kit.tr(&format!("toolbars:selectToolbar.tools.{key}.tooltip")));
                    if r.clicked() && enabled {
                        store.set_select_tool(*tool);
                    }
                });
            }
        });
    }
    let merge = format!("{} [J]", kit.tr("toolbars:selectToolbar.tools.merge.label"));
    let can_merge = store.can_merge_selection();
    if widgets::button(
        ui,
        kit,
        Some(Icon::GitMerge),
        Some(&merge),
        false,
        can_merge,
    )
    .on_hover_text(kit.tr("toolbars:selectToolbar.tools.merge.tooltip"))
    .clicked()
    {
        store.commit_selection_merge();
    }
    ui.add_space(12.0);
    selection_panel(ui, kit, store);
}

fn stats_box(ui: &mut Ui, kit: &Kit, pins: usize, actual: Option<f64>, perimeter: f64) {
    widgets::section_label(ui, kit, &kit.tr("panels:selectionPanel.stats.title"));
    let mut row = |label: String, value: String| {
        widgets::property_row(ui, kit, &label, |ui| {
            ui.label(
                egui::RichText::new(value)
                    .font(fonts::mono(12.0))
                    .color(kit.t.text_primary),
            );
        });
    };
    row(kit.tr("panels:selectionPanel.stats.pins"), pins.to_string());
    if let Some(a) = actual {
        row(
            kit.tr("panels:selectionPanel.stats.actualGap"),
            format!("{a:.2} cm"),
        );
    }
    row(
        kit.tr("panels:selectionPanel.stats.perimeter"),
        format!("{perimeter:.2} cm"),
    );
}

fn perimeter(g: &PinPathGeometry) -> f64 {
    g.contour_paths().iter().map(|p| p.length()).sum()
}

fn selection_panel(ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
    let muted = |ui: &mut Ui, kit: &Kit, text: String| {
        ui.label(
            egui::RichText::new(text)
                .size(12.5)
                .color(kit.t.text_secondary),
        );
    };
    match store.editor.selection.clone() {
        Selection::Pins(refs) => {
            let mut paths: Vec<&str> = refs.iter().map(|r| r.path_id.as_str()).collect();
            paths.sort_unstable();
            paths.dedup();
            muted(
                ui,
                kit,
                kit.trf(
                    "panels:selectionPanel.multiPinsSelected",
                    &[("count", &refs.len()), ("paths", &paths.len())],
                ),
            );
            return;
        }
        Selection::PinPaths(refs) if refs.len() > 1 => {
            muted(
                ui,
                kit,
                kit.trf(
                    "panels:selectionPanel.multiPathsSelected",
                    &[("count", &refs.len())],
                ),
            );
            let paths = store.selected_pin_paths();
            let pins = paths.iter().map(|p| p.pins.len()).sum();
            let per = paths.iter().map(|p| perimeter(&p.geometry)).sum();
            stats_box(ui, kit, pins, None, per);
            return;
        }
        Selection::PinPaths(_) => {}
        _ => {
            muted(ui, kit, kit.tr("panels:selectionPanel.emptyMessage"));
            return;
        }
    }
    let Some((layer_id, path)) = store
        .selected_pin_path()
        .map(|(l, p)| (l.to_owned(), p.clone()))
    else {
        return;
    };
    let g = path.geometry.clone();
    let kind_key = match &g {
        PinPathGeometry::RegularPolygon { .. } => "regular-polygon",
        PinPathGeometry::Polygon { .. } => "freehand",
        other => kind_name(other),
    };
    let ty = kit.tr(&format!("panels:selectionPanel.shapeTypes.{kind_key}"));
    widgets::section_label(
        ui,
        kit,
        &kit.trf("panels:selectionPanel.title", &[("type", &ty)]),
    );

    let pid = path.id.clone();
    let mut set = |store: &mut Store, field: &str, next: PinPathGeometry| {
        store.update_pin_path_geometry(&format!("geom:{pid}:{field}"), &layer_id, &pid, next);
    };
    let f = |k: &str| kit.tr(&format!("panels:selectionPanel.fields.{k}"));
    let num = |ui: &mut Ui, kit: &Kit, label: &str, v: f64, min: f64| -> Option<f64> {
        widgets::property_row(ui, kit, label, |ui| {
            widgets::number(ui, kit, v, min..=100_000.0, 0.05, 3)
        })
    };
    const ANY: f64 = -100_000.0;
    const POS: f64 = 0.01;

    if let Some(v) = num(
        ui,
        kit,
        &kit.tr("panels:selectionPanel.pinDistance"),
        path.requested_spacing,
        POS,
    ) {
        store.set_pin_property(
            &format!("spacing:{}", path.id),
            PinPropertyPatch {
                spacing: Some(v.max(0.01)),
                ..Default::default()
            },
        );
    }

    use PinPathGeometry as G;
    match g.clone() {
        G::Line { start, end } => {
            if let Some(v) = num(ui, kit, &f("startX"), start.x, ANY) {
                set(
                    store,
                    "sx",
                    G::Line {
                        start: Point::new(v, start.y),
                        end,
                    },
                );
            }
            if let Some(v) = num(ui, kit, &f("startY"), start.y, ANY) {
                set(
                    store,
                    "sy",
                    G::Line {
                        start: Point::new(start.x, v),
                        end,
                    },
                );
            }
            if let Some(v) = num(ui, kit, &f("endX"), end.x, ANY) {
                set(
                    store,
                    "ex",
                    G::Line {
                        start,
                        end: Point::new(v, end.y),
                    },
                );
            }
            if let Some(v) = num(ui, kit, &f("endY"), end.y, ANY) {
                set(
                    store,
                    "ey",
                    G::Line {
                        start,
                        end: Point::new(end.x, v),
                    },
                );
            }
        }
        G::Circle { center, radius } => {
            if let Some(v) = num(ui, kit, &f("centreX"), center.x, ANY) {
                set(
                    store,
                    "cx",
                    G::Circle {
                        center: Point::new(v, center.y),
                        radius,
                    },
                );
            }
            if let Some(v) = num(ui, kit, &f("centreY"), center.y, ANY) {
                set(
                    store,
                    "cy",
                    G::Circle {
                        center: Point::new(center.x, v),
                        radius,
                    },
                );
            }
            if let Some(v) = num(ui, kit, &f("radius"), radius, POS) {
                set(
                    store,
                    "r",
                    G::Circle {
                        center,
                        radius: v.max(POS),
                    },
                );
            }
        }
        G::Ellipse {
            center,
            radius_x,
            radius_y,
            rotation,
        } => {
            let e = |c: Point, rx: f64, ry: f64, rot: f64| G::Ellipse {
                center: c,
                radius_x: rx,
                radius_y: ry,
                rotation: rot,
            };
            if let Some(v) = num(ui, kit, &f("centreX"), center.x, ANY) {
                set(
                    store,
                    "cx",
                    e(Point::new(v, center.y), radius_x, radius_y, rotation),
                );
            }
            if let Some(v) = num(ui, kit, &f("centreY"), center.y, ANY) {
                set(
                    store,
                    "cy",
                    e(Point::new(center.x, v), radius_x, radius_y, rotation),
                );
            }
            if let Some(v) = num(ui, kit, &f("radiusX"), radius_x, POS) {
                set(store, "rx", e(center, v.max(POS), radius_y, rotation));
            }
            if let Some(v) = num(ui, kit, &f("radiusY"), radius_y, POS) {
                set(store, "ry", e(center, radius_x, v.max(POS), rotation));
            }
            if let Some(v) = num(ui, kit, &f("rotation"), rotation, ANY) {
                set(store, "rot", e(center, radius_x, radius_y, v));
            }
        }
        G::Rectangle {
            position,
            width,
            height,
            rotation,
        } => {
            let r = |p: Point, w: f64, h: f64, rot: f64| G::Rectangle {
                position: p,
                width: w,
                height: h,
                rotation: rot,
            };
            if let Some(v) = num(ui, kit, &f("positionX"), position.x, ANY) {
                set(
                    store,
                    "px",
                    r(Point::new(v, position.y), width, height, rotation),
                );
            }
            if let Some(v) = num(ui, kit, &f("positionY"), position.y, ANY) {
                set(
                    store,
                    "py",
                    r(Point::new(position.x, v), width, height, rotation),
                );
            }
            if let Some(v) = num(ui, kit, &f("width"), width, POS) {
                set(store, "w", r(position, v.max(POS), height, rotation));
            }
            if let Some(v) = num(ui, kit, &f("height"), height, POS) {
                set(store, "h", r(position, width, v.max(POS), rotation));
            }
            if let Some(v) = num(ui, kit, &f("rotation"), rotation, ANY) {
                set(store, "rot", r(position, width, height, v));
            }
        }
        G::Square {
            position,
            side,
            rotation,
        } => {
            let s = |p: Point, sd: f64, rot: f64| G::Square {
                position: p,
                side: sd,
                rotation: rot,
            };
            if let Some(v) = num(ui, kit, &f("positionX"), position.x, ANY) {
                set(store, "px", s(Point::new(v, position.y), side, rotation));
            }
            if let Some(v) = num(ui, kit, &f("positionY"), position.y, ANY) {
                set(store, "py", s(Point::new(position.x, v), side, rotation));
            }
            if let Some(v) = num(ui, kit, &f("side"), side, POS) {
                set(store, "side", s(position, v.max(POS), rotation));
            }
            if let Some(v) = num(ui, kit, &f("rotation"), rotation, ANY) {
                set(store, "rot", s(position, side, v));
            }
        }
        G::RegularPolygon {
            center,
            radius,
            sides,
            rotation,
        } => {
            let p = |c: Point, r: f64, rot: f64| G::RegularPolygon {
                center: c,
                radius: r,
                sides,
                rotation: rot,
            };
            centre_rotation(ui, kit, store, &mut set, center, rotation, |c, rot| {
                p(c, radius, rot)
            });
            if let Some(v) = num(ui, kit, &f("radius"), radius, POS) {
                set(store, "r", p(center, v.max(POS), rotation));
            }
        }
        G::Star {
            center,
            outer_radius,
            inner_radius,
            points,
            rotation,
        } => {
            let s = |c: Point, o: f64, i: f64, rot: f64| G::Star {
                center: c,
                outer_radius: o,
                inner_radius: i,
                points,
                rotation: rot,
            };
            centre_rotation(ui, kit, store, &mut set, center, rotation, |c, rot| {
                s(c, outer_radius, inner_radius, rot)
            });
            if let Some(v) = num(ui, kit, &f("outerRadius"), outer_radius, POS) {
                set(store, "or", s(center, v.max(POS), inner_radius, rotation));
            }
            if let Some(v) = num(ui, kit, &f("innerRadius"), inner_radius, POS) {
                set(store, "ir", s(center, outer_radius, v.max(POS), rotation));
            }
        }
        G::Polygram {
            center,
            radius,
            points,
            skip,
            rotation,
        } => {
            let p = |c: Point, r: f64, rot: f64| G::Polygram {
                center: c,
                radius: r,
                points,
                skip,
                rotation: rot,
            };
            centre_rotation(ui, kit, store, &mut set, center, rotation, |c, rot| {
                p(c, radius, rot)
            });
            if let Some(v) = num(ui, kit, &f("radius"), radius, POS) {
                set(store, "r", p(center, v.max(POS), rotation));
            }
        }
        G::Text {
            origin,
            text,
            font_id,
            weight,
            italic,
            size,
            letter_spacing,
            ..
        } => {
            let regen = |store: &mut Store,
                         field: &str,
                         text: &str,
                         font: &str,
                         w: FontWeight,
                         it: bool,
                         sz: f64,
                         ls: f64| {
                set(
                    store,
                    field,
                    build_text_geometry(origin, text, font, w, it, sz, ls),
                );
            };
            widgets::property_row(ui, kit, &f("text"), |ui| {
                let mut t = text.clone();
                let resp = ui.add(egui::TextEdit::singleline(&mut t).desired_width(130.0));
                if resp.changed() {
                    regen(
                        store,
                        "text",
                        &t,
                        &font_id,
                        weight,
                        italic,
                        size,
                        letter_spacing,
                    );
                }
                if text.is_empty() && !resp.has_focus() && ui.memory(|m| m.focused().is_none()) {
                    resp.request_focus();
                }
            });
            widgets::property_row(ui, kit, &f("font"), |ui| {
                let family = font_entry(&font_id).map(|e| e.family).unwrap_or("?");
                egui::ComboBox::from_id_salt("text-font")
                    .selected_text(family)
                    .show_ui(ui, |ui| {
                        for e in &FONT_CATALOG {
                            if ui.selectable_label(e.id == font_id, e.family).clicked()
                                && e.id != font_id
                            {
                                regen(
                                    store,
                                    "font",
                                    &text,
                                    e.id,
                                    weight,
                                    italic,
                                    size,
                                    letter_spacing,
                                );
                            }
                        }
                    });
            });
            let entry = font_entry(&font_id);
            widgets::property_row(ui, kit, &f("weight"), |ui| {
                let label = |w: FontWeight| {
                    kit.tr(&format!(
                        "panels:selectionPanel.weightOptions.{}",
                        if w == FontWeight::Bold {
                            "bold"
                        } else {
                            "regular"
                        }
                    ))
                };
                egui::ComboBox::from_id_salt("text-weight")
                    .selected_text(label(weight))
                    .show_ui(ui, |ui| {
                        let mut options = vec![FontWeight::Regular];
                        if entry.is_some_and(|e| e.has_bold()) {
                            options.push(FontWeight::Bold);
                        }
                        for w in options {
                            if ui.selectable_label(w == weight, label(w)).clicked() && w != weight {
                                regen(
                                    store,
                                    "weight",
                                    &text,
                                    &font_id,
                                    w,
                                    italic,
                                    size,
                                    letter_spacing,
                                );
                            }
                        }
                    });
            });
            widgets::property_row(ui, kit, &f("italic"), |ui| {
                let mut it = italic;
                ui.add_enabled_ui(entry.is_some_and(|e| e.has_italic()), |ui| {
                    if ui.checkbox(&mut it, "").changed() {
                        regen(
                            store,
                            "italic",
                            &text,
                            &font_id,
                            weight,
                            it,
                            size,
                            letter_spacing,
                        );
                    }
                });
            });
            if let Some(v) = num(ui, kit, &f("size"), size, 0.1) {
                regen(
                    store,
                    "size",
                    &text,
                    &font_id,
                    weight,
                    italic,
                    v.max(0.1),
                    letter_spacing,
                );
            }
            if let Some(v) = num(ui, kit, &f("letterSpacing"), letter_spacing, ANY) {
                regen(store, "ls", &text, &font_id, weight, italic, size, v);
            }
        }
        G::Arc { .. } | G::Freehand { .. } | G::Polygon { .. } => {}
    }

    ui.add_space(8.0);
    symmetry_panel(ui, kit, store);
    ui.add_space(8.0);
    if let Some((_, p)) = store.selected_pin_path() {
        let (n, a, per) = (p.pins.len(), p.actual_spacing, perimeter(&p.geometry));
        stats_box(ui, kit, n, Some(a), per);
    }
    ui.add_space(8.0);
    let label = kit.tr("panels:selectionPanel.deletePinPath");
    if widgets::button(ui, kit, Some(Icon::Trash2), Some(&label), false, true).clicked() {
        let lid = store.selected_pin_path().map(|(l, _)| l.to_owned());
        if let Some(lid) = lid {
            store.delete_pin_path(&lid, &path.id);
        }
    }
}

fn kind_name(g: &PinPathGeometry) -> &'static str {
    use PinPathGeometry as G;
    match g {
        G::Line { .. } => "line",
        G::Arc { .. } => "arc",
        G::Ellipse { .. } => "ellipse",
        G::Circle { .. } => "circle",
        G::Rectangle { .. } => "rectangle",
        G::Square { .. } => "square",
        G::RegularPolygon { .. } => "regular-polygon",
        G::Star { .. } => "star",
        G::Polygram { .. } => "polygram",
        G::Freehand { .. } | G::Polygon { .. } => "freehand",
        G::Text { .. } => "text",
    }
}

fn centre_rotation(
    ui: &mut Ui,
    kit: &Kit,
    store: &mut Store,
    set: &mut impl FnMut(&mut Store, &str, PinPathGeometry),
    center: Point,
    rotation: f64,
    make: impl Fn(Point, f64) -> PinPathGeometry,
) {
    let f = |k: &str| kit.tr(&format!("panels:selectionPanel.fields.{k}"));
    let row = |ui: &mut Ui, label: &str, v: f64| {
        widgets::property_row(ui, kit, label, |ui| {
            widgets::number(ui, kit, v, -100_000.0..=100_000.0, 0.05, 3)
        })
    };
    if let Some(v) = row(ui, &f("centreX"), center.x) {
        set(store, "cx", make(Point::new(v, center.y), rotation));
    }
    if let Some(v) = row(ui, &f("centreY"), center.y) {
        set(store, "cy", make(Point::new(center.x, v), rotation));
    }
    if let Some(v) = row(ui, &f("rotation"), rotation) {
        set(store, "rot", make(center, v));
    }
}
