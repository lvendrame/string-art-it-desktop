//! Pin mode side panel: tool grid, polygon/star picker, pin properties,
//! symmetry (specs 06, 08).

use egui::Ui;
use sa_app::document::{SymmetryConfig, SymmetryType};
use sa_app::editor::{PinTool, Store};
use sa_app::editor_pins::PinPropertyPatch;
use sa_app::pin_tools::POLYGON_FAMILY;
use sa_domain::Point;

use crate::fonts;
use crate::icons::Icon;
use crate::widgets::{self, Kit};

/// (tool, icon, label key, shortcut)
pub const BASIC_TOOLS: [(PinTool, Icon, &str, &str); 11] = [
    (
        PinTool::Line,
        Icon::Minus,
        "toolbars:pinToolbar.basicTools.line",
        "L",
    ),
    (
        PinTool::Arc,
        Icon::Spline,
        "toolbars:pinToolbar.basicTools.arc",
        "A",
    ),
    (
        PinTool::Ellipse,
        Icon::Ellipse,
        "toolbars:pinToolbar.basicTools.ellipse",
        "E",
    ),
    (
        PinTool::Circle,
        Icon::Circle,
        "toolbars:pinToolbar.basicTools.circle",
        "C",
    ),
    (
        PinTool::Rectangle,
        Icon::RectangleHorizontal,
        "toolbars:pinToolbar.basicTools.rectangle",
        "R",
    ),
    (
        PinTool::Square,
        Icon::Square,
        "toolbars:pinToolbar.basicTools.square",
        "S",
    ),
    (
        PinTool::Freehand,
        Icon::PenTool,
        "toolbars:pinToolbar.basicTools.freehand",
        "F",
    ),
    (
        PinTool::Polygon,
        Icon::Waypoints,
        "toolbars:pinToolbar.basicTools.path",
        "P",
    ),
    (
        PinTool::Text,
        Icon::Type,
        "toolbars:pinToolbar.basicTools.text",
        "T",
    ),
    (
        PinTool::Eraser,
        Icon::Eraser,
        "toolbars:pinToolbar.eraser",
        "D",
    ),
    (
        PinTool::PathEraser,
        Icon::Trash2,
        "toolbars:pinToolbar.pathEraser",
        "Q",
    ),
];

pub fn polygon_family_label(kit: &Kit, tool: PinTool) -> String {
    kit.tr(&format!("toolbars:pinToolbar.polygonFamily.{}", tool.key()))
}

pub fn pin_panel(ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
    ui.spacing_mut().item_spacing.y = 8.0;
    widgets::section_label(ui, kit, &kit.tr("toolbars:pinToolbar.sectionTitle"));
    let items: Vec<(Icon, String)> = BASIC_TOOLS
        .iter()
        .map(|(_, icon, key, sc)| (*icon, format!("{} [{sc}]", kit.tr(key))))
        .collect();
    let selected = BASIC_TOOLS
        .iter()
        .position(|(t, ..)| *t == store.editor.pin_tool);
    if let Some(i) = widgets::tool_grid(ui, kit, &items, selected) {
        store.set_pin_tool(BASIC_TOOLS[i].0);
    }

    // Polygon / Star picker
    let in_family = POLYGON_FAMILY.contains(&store.editor.pin_tool);
    ui.horizontal(|ui| {
        let tint = if in_family {
            kit.t.accent
        } else {
            kit.t.text_secondary
        };
        let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        kit.paint_icon(ui, Icon::Shapes, r, 14.0, tint);
        ui.label(
            egui::RichText::new(kit.tr("toolbars:pinToolbar.polygonStarLabel"))
                .font(fonts::semibold(12.0))
                .color(tint),
        );
    });
    let current = if in_family {
        polygon_family_label(kit, store.editor.pin_tool)
    } else {
        kit.tr("toolbars:pinToolbar.choosePlaceholder")
    };
    egui::ComboBox::from_id_salt("polygon-family")
        .width(ui.available_width())
        .selected_text(current)
        .show_ui(ui, |ui| {
            for t in POLYGON_FAMILY {
                if ui
                    .selectable_label(store.editor.pin_tool == t, polygon_family_label(kit, t))
                    .clicked()
                {
                    store.set_pin_tool(t);
                }
            }
        });

    ui.add_space(12.0);
    pin_properties(ui, kit, store);
    ui.add_space(12.0);
    symmetry_panel(ui, kit, store);
}

pub fn pin_properties(ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
    let selected = store.selected_pin_path().map(|(_, p)| p.clone());
    let title = if selected.is_some() {
        "panels:pinPropertiesPanel.titleSelected"
    } else {
        "panels:pinPropertiesPanel.titleDefaults"
    };
    widgets::section_label(ui, kit, &kit.tr(title));
    let (spacing, colour, diameter, guide) = match &selected {
        Some(p) => (
            p.requested_spacing,
            p.colour.clone(),
            p.diameter,
            p.guide_visible,
        ),
        None => {
            let d = &store.editor.pin_defaults;
            (d.spacing, d.colour.clone(), d.diameter, d.guide_visible)
        }
    };
    widgets::property_row(
        ui,
        kit,
        &kit.tr("panels:pinPropertiesPanel.spacing"),
        |ui| {
            if let Some(v) = widgets::number(ui, kit, spacing, 0.1..=1000.0, 0.05, 2) {
                store.set_pin_property(
                    "pin-spacing",
                    PinPropertyPatch {
                        spacing: Some(v.max(0.1)),
                        ..Default::default()
                    },
                );
            }
        },
    );
    if let Some(p) = &selected {
        widgets::property_row(
            ui,
            kit,
            &kit.tr("panels:pinPropertiesPanel.actualGap"),
            |ui| {
                ui.label(
                    egui::RichText::new(format!("{:.2} cm", p.actual_spacing))
                        .font(fonts::mono(12.0))
                        .color(kit.t.text_secondary),
                );
            },
        );
    }
    widgets::property_row(
        ui,
        kit,
        &kit.tr("panels:pinPropertiesPanel.diameter"),
        |ui| {
            if let Some(v) = widgets::number(ui, kit, diameter, 0.1..=100.0, 0.05, 2) {
                store.set_pin_property(
                    "pin-diameter",
                    PinPropertyPatch {
                        diameter: Some(v.max(0.1)),
                        ..Default::default()
                    },
                );
            }
        },
    );
    widgets::property_row(ui, kit, &kit.tr("panels:pinPropertiesPanel.colour"), |ui| {
        if let Some(c) = widgets::colour(ui, &colour) {
            store.set_pin_property(
                "pin-colour",
                PinPropertyPatch {
                    colour: Some(c),
                    ..Default::default()
                },
            );
        }
    });
    widgets::property_row(
        ui,
        kit,
        &kit.tr("panels:pinPropertiesPanel.guideVisible"),
        |ui| {
            let mut g = guide;
            if ui.checkbox(&mut g, "").changed() {
                store.set_pin_property(
                    "pin-guide",
                    PinPropertyPatch {
                        guide_visible: Some(g),
                        ..Default::default()
                    },
                );
            }
        },
    );
}

const SYMMETRY_OPTIONS: [(SymmetryType, &str); 5] = [
    (SymmetryType::None, "panels:symmetryPanel.options.none"),
    (
        SymmetryType::Horizontal,
        "panels:symmetryPanel.options.horizontal",
    ),
    (
        SymmetryType::Vertical,
        "panels:symmetryPanel.options.vertical",
    ),
    (SymmetryType::Both, "panels:symmetryPanel.options.both"),
    (SymmetryType::Radial, "panels:symmetryPanel.options.radial"),
];

pub fn symmetry_panel(ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
    widgets::section_label(ui, kit, &kit.tr("panels:symmetryPanel.sectionTitle"));
    let config = store.active_symmetry().clone();
    let gap = 6.0;
    let w = ((ui.available_width() - gap * 2.0) / 3.0).floor();
    for row in SYMMETRY_OPTIONS.chunks(3) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (kind, key) in row {
                if widgets::text_button(ui, kit, &kit.tr(key), config.kind() == *kind, Some(w))
                    .clicked()
                {
                    store.set_symmetry_config(SymmetryConfig::for_type(*kind));
                }
            }
        });
    }
    if let SymmetryConfig::Radial {
        centre,
        interval_degrees,
    } = config
    {
        widgets::property_row(ui, kit, &kit.tr("panels:symmetryPanel.interval"), |ui| {
            if let Some(v) = widgets::number(ui, kit, interval_degrees, 1.0..=359.0, 1.0, 1) {
                store.set_symmetry_config(SymmetryConfig::Radial {
                    centre,
                    interval_degrees: v,
                });
            }
        });
        widgets::property_row(ui, kit, &kit.tr("panels:symmetryPanel.centreX"), |ui| {
            if let Some(v) = widgets::number(ui, kit, centre.x, -10_000.0..=10_000.0, 0.1, 2) {
                store.set_symmetry_config(SymmetryConfig::Radial {
                    centre: Point::new(v, centre.y),
                    interval_degrees,
                });
            }
        });
        widgets::property_row(ui, kit, &kit.tr("panels:symmetryPanel.centreY"), |ui| {
            if let Some(v) = widgets::number(ui, kit, centre.y, -10_000.0..=10_000.0, 0.1, 2) {
                store.set_symmetry_config(SymmetryConfig::Radial {
                    centre: Point::new(centre.x, v),
                    interval_degrees,
                });
            }
        });
    }
}
