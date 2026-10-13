//! Board appearance controls (spec 04) — shared by the board setup screen and
//! the canvas "Change background" popover.

use base64::Engine;
use egui::Ui;
use sa_app::document::{BoardAppearance, GradientStop};
use sa_app::editor::Store;
use sa_domain::Point;
use sa_render::wood::{PAINT_PRESETS, WOOD_PRESETS};

use crate::widgets::{self, Kit};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Solid,
    Linear,
    Radial,
    Wood,
    Painted,
    Custom,
}

const KINDS: [(Kind, &str); 6] = [
    (Kind::Solid, "boardSetup:appearance.types.solid"),
    (Kind::Linear, "boardSetup:appearance.types.linearGradient"),
    (Kind::Radial, "boardSetup:appearance.types.radialGradient"),
    (Kind::Wood, "boardSetup:appearance.types.woodTexture"),
    (Kind::Painted, "boardSetup:appearance.types.paintedWood"),
    (Kind::Custom, "boardSetup:appearance.types.customTexture"),
];

fn kind_of(a: &BoardAppearance) -> Kind {
    match a {
        BoardAppearance::Solid { .. } => Kind::Solid,
        BoardAppearance::LinearGradient { .. } => Kind::Linear,
        BoardAppearance::RadialGradient { .. } => Kind::Radial,
        BoardAppearance::WoodTexture { .. } => Kind::Wood,
        BoardAppearance::PaintedWood { .. } => Kind::Painted,
        BoardAppearance::CustomTexture { .. } => Kind::Custom,
    }
}

fn stop(offset: f64, colour: &str) -> GradientStop {
    GradientStop {
        offset,
        colour: colour.into(),
    }
}

/// Web `defaultFor`.
fn default_for(kind: Kind) -> BoardAppearance {
    match kind {
        Kind::Solid => BoardAppearance::Solid {
            colour: "#8a5a3c".into(),
        },
        Kind::Linear => BoardAppearance::LinearGradient {
            stops: vec![stop(0.0, "#8a5a3c"), stop(100.0, "#3a2318")],
            direction: 45.0,
        },
        Kind::Radial => BoardAppearance::RadialGradient {
            stops: vec![stop(0.0, "#c9a06a"), stop(100.0, "#4a2a1a")],
            centre: Point::new(40.0, 35.0),
        },
        Kind::Wood => BoardAppearance::WoodTexture {
            preset_id: "walnut".into(),
        },
        Kind::Painted => BoardAppearance::PaintedWood {
            preset_id: "sage".into(),
        },
        Kind::Custom => BoardAppearance::CustomTexture {
            image_data_url: String::new(),
        },
    }
}

fn pick_texture() -> Option<String> {
    let path = rfd::FileDialog::new()
        .add_filter("Image", &["png", "jpg", "jpeg", "webp", "gif", "bmp"])
        .pick_file()?;
    let bytes = std::fs::read(&path).ok()?;
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "image/png",
    };
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

pub fn appearance_panel(ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
    widgets::section_label(ui, kit, &kit.tr("boardSetup:appearance.section"));
    let appearance = store.doc.board.appearance.clone();
    let current = kind_of(&appearance);
    let gap = 6.0;
    let w = ((ui.available_width() - gap * 2.0) / 3.0).floor();
    for row in KINDS.chunks(3) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (kind, key) in row {
                if widgets::text_button(ui, kit, &kit.tr(key), *kind == current, Some(w)).clicked()
                    && *kind != current
                {
                    store.set_board_appearance(default_for(*kind));
                }
            }
        });
    }
    ui.add_space(6.0);

    match appearance {
        BoardAppearance::Solid { colour } => {
            widgets::property_row(ui, kit, &kit.tr("boardSetup:appearance.colour"), |ui| {
                if let Some(c) = widgets::colour(ui, &colour) {
                    store.edit_board_appearance(
                        "solid-colour",
                        BoardAppearance::Solid { colour: c },
                    );
                }
            });
        }
        BoardAppearance::LinearGradient { stops, direction } => {
            stop_row(ui, &stops, |stops| {
                store.edit_board_appearance(
                    "gradient-stops",
                    BoardAppearance::LinearGradient { stops, direction },
                )
            });
            widgets::property_row(ui, kit, &kit.tr("boardSetup:appearance.direction"), |ui| {
                if let Some(d) = widgets::number(ui, kit, direction, -360.0..=360.0, 1.0, 0) {
                    store.edit_board_appearance(
                        "gradient-direction",
                        BoardAppearance::LinearGradient {
                            stops: stops.clone(),
                            direction: d,
                        },
                    );
                }
            });
        }
        BoardAppearance::RadialGradient { stops, centre } => {
            stop_row(ui, &stops, |stops| {
                store.edit_board_appearance(
                    "gradient-stops",
                    BoardAppearance::RadialGradient { stops, centre },
                )
            });
            widgets::property_row(ui, kit, &kit.tr("boardSetup:appearance.centreX"), |ui| {
                if let Some(x) = widgets::number(ui, kit, centre.x, 0.0..=100.0, 0.5, 0) {
                    store.edit_board_appearance(
                        "gradient-centre-x",
                        BoardAppearance::RadialGradient {
                            stops: stops.clone(),
                            centre: Point::new(x, centre.y),
                        },
                    );
                }
            });
            widgets::property_row(ui, kit, &kit.tr("boardSetup:appearance.centreY"), |ui| {
                if let Some(y) = widgets::number(ui, kit, centre.y, 0.0..=100.0, 0.5, 0) {
                    store.edit_board_appearance(
                        "gradient-centre-y",
                        BoardAppearance::RadialGradient {
                            stops: stops.clone(),
                            centre: Point::new(centre.x, y),
                        },
                    );
                }
            });
        }
        BoardAppearance::WoodTexture { preset_id } | BoardAppearance::PaintedWood { preset_id }
            if matches!(current, Kind::Wood | Kind::Painted) =>
        {
            let ids: Vec<&str> = if current == Kind::Wood {
                WOOD_PRESETS.iter().map(|(k, _)| *k).collect()
            } else {
                PAINT_PRESETS.iter().map(|(k, _)| *k).collect()
            };
            let label = |id: &str| kit.try_preset(id);
            egui::ComboBox::from_id_salt("board-preset")
                .width(ui.available_width())
                .selected_text(label(&preset_id))
                .show_ui(ui, |ui| {
                    for id in ids {
                        if ui.selectable_label(id == preset_id, label(id)).clicked()
                            && id != preset_id
                        {
                            let a = if current == Kind::Wood {
                                BoardAppearance::WoodTexture {
                                    preset_id: id.into(),
                                }
                            } else {
                                BoardAppearance::PaintedWood {
                                    preset_id: id.into(),
                                }
                            };
                            store.set_board_appearance(a);
                        }
                    }
                });
        }
        BoardAppearance::CustomTexture { image_data_url } => {
            if ui
                .button(kit.tr("boardSetup:appearance.types.customTexture") + "…")
                .clicked()
                && let Some(url) = pick_texture()
            {
                store.set_board_appearance(BoardAppearance::CustomTexture {
                    image_data_url: url,
                });
            }
            if !image_data_url.is_empty()
                && sa_render::image::decode_data_url(&image_data_url).is_none()
            {
                ui.colored_label(kit.t.danger, "⚠");
            }
        }
        _ => {}
    }
}

fn stop_row(ui: &mut Ui, stops: &[GradientStop], mut set: impl FnMut(Vec<GradientStop>)) {
    ui.horizontal(|ui| {
        for (i, s) in stops.iter().enumerate() {
            if let Some(c) = widgets::colour(ui, &s.colour) {
                let mut next = stops.to_vec();
                next[i].colour = c;
                set(next);
            }
        }
    });
}

impl Kit<'_> {
    /// Preset display name with web's `defaultValue` fallback (capitalised id).
    pub fn try_preset(&self, id: &str) -> String {
        self.i18n
            .try_t(&format!("boardSetup:appearance.presets.{id}"))
            .unwrap_or_else(|| {
                let mut c = id.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            })
    }
}
