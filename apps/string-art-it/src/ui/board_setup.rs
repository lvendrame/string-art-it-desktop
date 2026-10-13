//! Board setup screen (spec 03): shape, dimensions, appearance, Continue.

use egui::Ui;
use sa_app::document::{BoardDimensions, BoardShape, TriangleType};
use sa_app::editor::Store;

use crate::fonts;
use crate::ui::appearance::appearance_panel;
use crate::widgets::{self, Kit};

const SHAPES: [(BoardShape, &str); 5] = [
    (BoardShape::Circle, "boardSetup:shapes.circle"),
    (BoardShape::Oval, "boardSetup:shapes.oval"),
    (BoardShape::Rectangle, "boardSetup:shapes.rectangle"),
    (BoardShape::Square, "boardSetup:shapes.square"),
    (BoardShape::Triangle, "boardSetup:shapes.triangle"),
];

pub enum SetupAction {
    None,
    Continue,
    CycleLanguage,
}

pub fn board_setup(
    ui: &mut Ui,
    kit: &mut Kit,
    store: &mut Store,
    language_switcher: impl FnOnce(&mut Ui, &mut Kit),
) -> SetupAction {
    let t = kit.t;
    let mut action = SetupAction::None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            egui::Frame::new()
                .fill(t.bg_panel)
                .stroke(egui::Stroke::new(1.0, t.border))
                .corner_radius(crate::theme::RADIUS_MD)
                .inner_margin(28)
                .show(ui, |ui| {
                    ui.set_width(440.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.spacing_mut().item_spacing.y = 10.0;
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(kit.tr("boardSetup:title"))
                                    .font(fonts::extrabold(20.0))
                                    .color(t.text_primary),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| language_switcher(ui, kit),
                            );
                        });
                        ui.add_space(6.0);

                        widgets::section_label(ui, kit, &kit.tr("boardSetup:sections.shape"));
                        let board = store.doc.board.clone();
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                            for (shape, key) in SHAPES {
                                if widgets::text_button(
                                    ui,
                                    kit,
                                    &kit.tr(key),
                                    board.shape == shape,
                                    Some(82.0),
                                )
                                .clicked()
                                    && board.shape != shape
                                {
                                    let tri = (shape == BoardShape::Triangle)
                                        .then_some(TriangleType::Equilateral);
                                    store.set_board_shape(shape, tri);
                                }
                            }
                        });
                        if board.shape == BoardShape::Triangle {
                            ui.horizontal(|ui| {
                                for (tri, key) in [
                                    (
                                        TriangleType::Equilateral,
                                        "boardSetup:triangleTypes.equilateral",
                                    ),
                                    (
                                        TriangleType::RightAngled,
                                        "boardSetup:triangleTypes.rightAngled",
                                    ),
                                ] {
                                    if widgets::text_button(
                                        ui,
                                        kit,
                                        &kit.tr(key),
                                        board.triangle_type == Some(tri),
                                        Some(130.0),
                                    )
                                    .clicked()
                                        && board.triangle_type != Some(tri)
                                    {
                                        store.set_triangle_type(tri);
                                    }
                                }
                            });
                        }

                        ui.add_space(6.0);
                        widgets::section_label(ui, kit, &kit.tr("boardSetup:sections.dimensions"));
                        dimension_fields(ui, kit, store);

                        ui.add_space(6.0);
                        appearance_panel(ui, kit, store);

                        ui.add_space(12.0);
                        if widgets::primary_button(
                            ui,
                            kit,
                            &kit.tr("boardSetup:continueButton"),
                            ui.available_width(),
                        )
                        .clicked()
                        {
                            action = SetupAction::Continue;
                        }
                    });
                });
            ui.add_space(40.0);
        });
    });
    // Screen shortcuts (spec 34): Enter continues, L cycles language.
    if !ui.ctx().egui_wants_keyboard_input() {
        ui.input(|i| {
            if i.key_pressed(egui::Key::Enter) {
                action = SetupAction::Continue;
            } else if i.key_pressed(egui::Key::L) && i.modifiers.is_none() {
                action = SetupAction::CycleLanguage;
            }
        });
    }
    action
}

fn dimension_fields(ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
    let b = store.doc.board.clone();
    let d = &b.dimensions;
    let mut field = |ui: &mut Ui, key: &str, value: f64, make: fn(f64) -> BoardDimensions| {
        widgets::property_row(ui, kit, &kit.tr(key), |ui| {
            if let Some(v) = widgets::number(ui, kit, value, 0.1..=10_000.0, 0.1, 1) {
                store.edit_board_dimensions(key, make(v));
            }
        });
    };
    match b.shape {
        BoardShape::Circle => field(
            ui,
            "boardSetup:fields.diameter",
            d.diameter.unwrap_or(60.0),
            |v| BoardDimensions {
                diameter: Some(v),
                ..Default::default()
            },
        ),
        BoardShape::Oval | BoardShape::Rectangle => {
            field(
                ui,
                "boardSetup:fields.width",
                d.width.unwrap_or(60.0),
                |v| BoardDimensions {
                    width: Some(v),
                    ..Default::default()
                },
            );
            field(
                ui,
                "boardSetup:fields.height",
                d.height.unwrap_or(40.0),
                |v| BoardDimensions {
                    height: Some(v),
                    ..Default::default()
                },
            );
        }
        BoardShape::Square => field(ui, "boardSetup:fields.side", d.side.unwrap_or(50.0), |v| {
            BoardDimensions {
                side: Some(v),
                ..Default::default()
            }
        }),
        BoardShape::Triangle => {
            if b.triangle_type == Some(TriangleType::RightAngled) {
                field(ui, "boardSetup:fields.base", d.base.unwrap_or(40.0), |v| {
                    BoardDimensions {
                        base: Some(v),
                        ..Default::default()
                    }
                });
                field(
                    ui,
                    "boardSetup:fields.height",
                    d.height.unwrap_or(30.0),
                    |v| BoardDimensions {
                        height: Some(v),
                        ..Default::default()
                    },
                );
                let h = store.doc.board.hypotenuse().unwrap_or(0.0);
                ui.label(
                    egui::RichText::new(
                        kit.trf("boardSetup:hypotenuse", &[("value", &format!("{h:.1}"))]),
                    )
                    .font(fonts::mono(12.0))
                    .color(kit.t.text_secondary),
                );
            } else {
                field(ui, "boardSetup:fields.side", d.side.unwrap_or(50.0), |v| {
                    BoardDimensions {
                        side: Some(v),
                        ..Default::default()
                    }
                })
            }
        }
    }
}
