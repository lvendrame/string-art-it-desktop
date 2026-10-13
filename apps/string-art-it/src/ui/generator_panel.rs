//! Generator mode (spec 32): pattern picker, per-pattern parameters, colour
//! list, Generate → live re-generate (300 ms debounce) → Confirm.

use egui::Ui;
use sa_app::Document;
use sa_app::document::{PinLayer, ThreadLayer};
use sa_app::editor::Store;
use sa_app::editor_threads::ThreadPropertyPatch;
use sa_app::generator::{GeneratorParams, PatternId, RingType};
use sa_render::scene::{Item, SceneOptions, build_scene};

use crate::canvas::paint::{Xf, paint_items};
use crate::ui::thread_panel::PALETTE;
use crate::widgets::{self, Kit};

pub struct GeneratorPanel {
    params: GeneratorParams,
    colours: Vec<String>,
    dirty_at: Option<f64>,
}

impl Default for GeneratorPanel {
    fn default() -> Self {
        Self {
            params: PatternId::Mandala.default_params(),
            colours: vec![PALETTE[0].into()],
            dirty_at: None,
        }
    }
}

const ROT: (f64, f64, f64) = (-3.15, 3.15, 0.05);

impl GeneratorPanel {
    fn visible_colours(&self) -> Vec<String> {
        self.colours
            .iter()
            .take(self.params.max_colours().max(1))
            .cloned()
            .collect()
    }

    pub fn show(&mut self, ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
        ui.spacing_mut().item_spacing.y = 8.0;
        widgets::section_label(ui, kit, &kit.tr("toolbars:generatorToolbar.sectionTitle"));
        let current = self.params.id();
        let label = |id: PatternId| {
            kit.tr(&format!(
                "toolbars:generatorToolbar.patterns.{}",
                id.label_key()
            ))
        };
        let mut changed = false;
        ui.label(
            egui::RichText::new(kit.tr("toolbars:generatorToolbar.patternLabel"))
                .size(12.0)
                .color(kit.t.text_secondary),
        );
        egui::ComboBox::from_id_salt("pattern")
            .width(ui.available_width())
            .selected_text(label(current))
            .show_ui(ui, |ui| {
                for id in PatternId::ALL {
                    if ui.selectable_label(id == current, label(id)).clicked() && id != current {
                        self.params = id.default_params();
                        changed = true;
                    }
                }
            });

        changed |= self.fields(ui, kit);

        let max = self.params.max_colours().max(1);
        let shown = self.visible_colours();
        let note = (max > 1).then(|| kit.trf("panels:generatorPanel.coloursMax", &[("max", &max)]));
        if let Some(v) = widgets::colour_list_field(
            ui,
            kit,
            &kit.tr("panels:generatorPanel.coloursSectionTitle"),
            &shown,
            max,
            &PALETTE,
            note.as_deref(),
        ) {
            // keep colours beyond the current cap so raising it restores them
            let mut all = v.clone();
            all.extend(self.colours.iter().skip(shown.len().max(v.len())).cloned());
            self.colours = all;
            changed = true;
        }
        let width = store.editor.thread_defaults.width;
        if let Some(w) = widgets::slider_field(
            ui,
            kit,
            &kit.tr("panels:generatorPanel.fields.threadWidth"),
            width,
            0.5..=5.0,
            0.5,
            true,
            |v| format!("{v}"),
        ) {
            store.set_thread_property(
                "gen-width",
                ThreadPropertyPatch {
                    width: Some(w),
                    ..Default::default()
                },
            );
            changed = true;
        }

        let now = ui.input(|i| i.time);
        if changed && store.editor.generator_draft.is_some() {
            self.dirty_at = Some(now);
        }
        if let Some(t) = self.dirty_at {
            if now - t >= 0.3 {
                store.generate_pattern(self.params.clone(), self.visible_colours());
                self.dirty_at = None;
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(60));
            }
        }

        ui.add_space(8.0);
        if store.editor.generator_draft.is_none() {
            if widgets::primary_button(
                ui,
                kit,
                &kit.tr("panels:generatorPanel.generate"),
                ui.available_width(),
            )
            .clicked()
            {
                store.generate_pattern(self.params.clone(), self.visible_colours());
            }
        } else {
            ui.label(
                egui::RichText::new(kit.tr("panels:generatorPanel.liveHint"))
                    .size(11.5)
                    .color(kit.t.text_tertiary),
            );
            if widgets::primary_button(
                ui,
                kit,
                &kit.tr("panels:generatorPanel.confirm"),
                ui.available_width(),
            )
            .clicked()
            {
                store.confirm_generated_pattern();
            }
        }
    }

    /// Per-pattern parameter fields (ranges from the web GeneratorPanel).
    fn fields(&mut self, ui: &mut Ui, kit: &Kit) -> bool {
        let ch = std::cell::Cell::new(false);
        let f = |k: &str| kit.tr(&format!("panels:generatorPanel.fields.{k}"));
        let s = |ui: &mut Ui, key: &str, v: &mut f64, min: f64, max: f64, step: f64| {
            let decimals = if step >= 1.0 {
                0
            } else if step >= 0.01 {
                2
            } else {
                3
            };
            if let Some(n) =
                widgets::slider_field(ui, kit, &f(key), *v, min..=max, step, true, move |x| {
                    format!("{x:.decimals$}")
                })
            {
                *v = n;
                ch.set(true);
            }
        };
        let check = |ui: &mut Ui, key: &str, v: &mut bool| {
            if let Some(n) = widgets::checkbox_field(ui, &f(key), *v, true) {
                *v = n;
                ch.set(true);
            }
        };
        use GeneratorParams as G;
        match &mut self.params {
            G::Mandala { n, base, layers } => {
                s(ui, "n", n, 3.0, 400.0, 1.0);
                s(ui, "base", base, 2.0, 99.0, 1.0);
                s(ui, "layers", layers, 1.0, 20.0, 1.0);
            }
            G::Star {
                side_nails,
                star_points,
                outer_ratio,
                inner_ratio,
                rotation,
            } => {
                s(ui, "sideNails", side_nails, 2.0, 200.0, 1.0);
                s(ui, "starPoints", star_points, 3.0, 20.0, 1.0);
                s(ui, "starOuterRatio", outer_ratio, 0.1, 1.0, 0.05);
                s(ui, "starInnerRatio", inner_ratio, 0.0, 0.95, 0.05);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
            }
            G::StarOfDavid {
                depth,
                layer_angle,
                rotation,
                mirror_tiling,
            }
            | G::HexagonSpades {
                depth,
                layer_angle,
                rotation,
                mirror_tiling,
            } => {
                s(ui, "depth", depth, 1.0, 40.0, 1.0);
                s(ui, "layerAngle", layer_angle, 0.02, 0.15, 0.001);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
                check(ui, "mirrorTiling", mirror_tiling);
            }
            G::Spirals {
                arms,
                nails_per_spiral,
                total_turns,
                rotation,
            } => {
                s(ui, "arms", arms, 2.0, 20.0, 1.0);
                s(ui, "nailsPerSpiral", nails_per_spiral, 3.0, 300.0, 1.0);
                s(ui, "totalAngleTurns", total_turns, 0.0, 5.0, 0.05);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
            }
            G::Freestyle { circles } => {
                for (i, k) in circles.iter_mut().enumerate() {
                    ui.separator();
                    let mut en = k.enabled;
                    if ui
                        .checkbox(
                            &mut en,
                            kit.trf("panels:generatorPanel.fields.circleN", &[("n", &(i + 1))]),
                        )
                        .changed()
                    {
                        k.enabled = en;
                        ch.set(true);
                    }
                    s(ui, "nails", &mut k.nails, 0.0, 300.0, 1.0);
                    s(ui, "radiusRatio", &mut k.radius_ratio, 0.05, 1.0, 0.05);
                    s(ui, "centerXRatio", &mut k.center_x_ratio, -1.0, 1.0, 0.01);
                    s(ui, "centerYRatio", &mut k.center_y_ratio, -1.0, 1.0, 0.01);
                }
            }
            G::Wave {
                n,
                base,
                layers,
                layer_fill,
                layer_spread,
            } => {
                s(ui, "n", n, 3.0, 400.0, 1.0);
                s(ui, "base", base, 2.0, 99.0, 1.0);
                s(ui, "layers", layers, 1.0, 20.0, 1.0);
                s(ui, "layerFill", layer_fill, 1.0, 400.0, 1.0);
                s(ui, "layerSpread", layer_spread, 0.0, 200.0, 1.0);
            }
            G::DanceOfPlanets {
                outer_type,
                outer_nails,
                outer_sides,
                inner_type,
                inner_nails,
                inner_sides,
                inner_size_ratio,
                rounds,
                reverse,
                rotation,
            } => {
                for (key, ty) in [
                    ("outerType", &mut *outer_type),
                    ("innerType", &mut *inner_type),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(f(key));
                        ch.set(
                            ch.get()
                                | ui.radio_value(ty, RingType::Circle, f("shapeCircle"))
                                    .changed(),
                        );
                        ch.set(
                            ch.get()
                                | ui.radio_value(ty, RingType::Polygon, f("shapePolygon"))
                                    .changed(),
                        );
                    });
                }
                s(ui, "outerNails", outer_nails, 3.0, 400.0, 1.0);
                if *outer_type == RingType::Polygon {
                    s(ui, "outerSides", outer_sides, 3.0, 20.0, 1.0);
                }
                s(ui, "innerNails", inner_nails, 3.0, 400.0, 1.0);
                if *inner_type == RingType::Polygon {
                    s(ui, "innerSides", inner_sides, 3.0, 20.0, 1.0);
                }
                s(ui, "innerSizeRatio", inner_size_ratio, 0.05, 0.95, 0.05);
                s(ui, "rounds", rounds, 1.0, 20.0, 1.0);
                check(ui, "reverse", reverse);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
            }
            G::Sun {
                side_nails,
                star_points,
                outer_ratio,
                inner_ratio,
                rotation,
                layers,
                layer_spread,
            } => {
                s(ui, "sideNails", side_nails, 2.0, 200.0, 1.0);
                s(ui, "starPoints", star_points, 3.0, 20.0, 1.0);
                s(ui, "starOuterRatio", outer_ratio, 0.1, 1.0, 0.05);
                s(ui, "starInnerRatio", inner_ratio, 0.0, 0.95, 0.05);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
                s(ui, "layers", layers, 0.0, 10.0, 1.0);
                s(ui, "layerSpread", layer_spread, 0.0, 0.3, 0.01);
            }
            G::Vortex {
                sides,
                nails_per_side,
                layers,
                layer_angle,
                rotation,
            } => {
                s(ui, "sides", sides, 3.0, 10.0, 1.0);
                s(ui, "nailsPerSide", nails_per_side, 3.0, 200.0, 1.0);
                s(ui, "layers", layers, 1.0, 40.0, 1.0);
                s(ui, "layerAngle", layer_angle, 0.01, 0.2, 0.005);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
            }
            G::Polygon {
                sides,
                nails_per_side,
                bezier_step,
                rotation,
            } => {
                s(ui, "sides", sides, 3.0, 20.0, 1.0);
                s(ui, "nailsPerSide", nails_per_side, 2.0, 100.0, 1.0);
                s(ui, "bezierStep", bezier_step, 1.0, 19.0, 1.0);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
            }
            G::Flower {
                sides,
                nails_per_side,
                layers,
                rotation,
            } => {
                s(ui, "sides", sides, 3.0, 20.0, 1.0);
                s(ui, "nailsPerSide", nails_per_side, 2.0, 100.0, 1.0);
                s(ui, "layers", layers, 1.0, 20.0, 1.0);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
            }
            G::Assymetry {
                circle_nails,
                layers,
                rotation,
            } => {
                s(ui, "circleNails", circle_nails, 3.0, 400.0, 1.0);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
                for (i, l) in layers.iter_mut().enumerate() {
                    ui.separator();
                    let mut en = l.enabled;
                    if ui
                        .checkbox(
                            &mut en,
                            kit.trf("panels:generatorPanel.fields.layerN", &[("n", &(i + 1))]),
                        )
                        .changed()
                    {
                        l.enabled = en;
                        ch.set(true);
                    }
                    s(ui, "startFraction", &mut l.start, 0.0, 1.0, 0.001);
                    s(ui, "endFraction", &mut l.end, 0.0, 1.0, 0.001);
                    check(ui, "reverse", &mut l.reverse);
                }
            }
            G::Spiral {
                n,
                repetition,
                inner_length,
                rotation,
            } => {
                s(ui, "n", n, 10.0, 400.0, 1.0);
                s(ui, "repetition", repetition, 1.0, 20.0, 1.0);
                s(ui, "innerLength", inner_length, 1.0, 100.0, 1.0);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
            }
            G::MaurerRose {
                n,
                max_steps,
                angle_degrees,
                rotation,
            } => {
                s(ui, "N", n, 1.0, 30.0, 1.0);
                s(ui, "maxSteps", max_steps, 10.0, 720.0, 1.0);
                s(ui, "angleDegrees", angle_degrees, 1.0, 180.0, 1.0);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
            }
            G::Comet {
                n,
                layers,
                first_layer_size,
                layer_distance,
                cluster_strength,
                distortion,
                rotation,
            } => {
                s(ui, "n", n, 10.0, 400.0, 1.0);
                s(ui, "layers", layers, 1.0, 40.0, 1.0);
                s(ui, "firstLayerSize", first_layer_size, 2.0, 200.0, 1.0);
                s(ui, "layerDistance", layer_distance, 1.0, 200.0, 1.0);
                s(ui, "clusterStrength", cluster_strength, 0.0, 1.0, 0.05);
                s(ui, "distortion", distortion, 0.0, 0.9, 0.02);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
            }
            G::FlowerOfLife {
                levels,
                density,
                rotation,
                ring_enabled,
                ring_nails,
                ring_base,
            } => {
                s(ui, "levels", levels, 1.0, 6.0, 1.0);
                s(ui, "density", density, 2.0, 30.0, 1.0);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
                check(ui, "ringEnabled", ring_enabled);
                if *ring_enabled {
                    s(ui, "ringNails", ring_nails, 3.0, 400.0, 1.0);
                    s(ui, "ringBase", ring_base, 2.0, 99.0, 1.0);
                }
            }
            G::Crosses {
                nails_per_line,
                vertical,
                gap,
                sides_rotation,
            } => {
                s(ui, "nailsPerLine", nails_per_line, 2.0, 100.0, 1.0);
                ui.horizontal(|ui| {
                    ui.label(f("orientation"));
                    ch.set(
                        ch.get()
                            | ui.radio_value(vertical, true, f("orientationVertical"))
                                .changed(),
                    );
                    ch.set(
                        ch.get()
                            | ui.radio_value(vertical, false, f("orientationHorizontal"))
                                .changed(),
                    );
                });
                s(ui, "gap", gap, 0.0, 1.0, 0.01);
                s(ui, "sidesRotation", sides_rotation, -1.57, 1.57, 0.02);
            }
            G::Lotus {
                sides,
                density,
                rotation,
                remove_sections,
                render_center,
                center_radius,
                radial_color,
            } => {
                s(ui, "sides", sides, 5.0, 64.0, 1.0);
                s(ui, "density", density, 1.0, 30.0, 1.0);
                s(ui, "rotation", rotation, ROT.0, ROT.1, ROT.2);
                s(ui, "removeSections", remove_sections, 0.0, 1.0, 0.02);
                check(ui, "renderCenter", render_center);
                if *render_center {
                    s(ui, "centerRadius", center_radius, 0.0, 1.0, 0.02);
                }
                check(ui, "radialColor", radial_color);
            }
        }
        ch.get()
    }
}

/// Generator draft drawn over the canvas at 60 % opacity.
pub fn paint_preview(painter: &egui::Painter, xf: &Xf, store: &Store) {
    let Some((_, out)) = &store.editor.generator_draft else {
        return;
    };
    let mut pl = PinLayer::new("preview");
    pl.pin_paths = out.pin_paths.clone();
    let mut tl = ThreadLayer::new("preview");
    tl.thread_paths = out.thread_paths.clone();
    let doc = Document {
        board: store.doc.board.clone(),
        pin_layers: vec![pl],
        thread_layers: vec![tl],
    };
    let mut scene = build_scene(&doc, &SceneOptions::editor());
    scene.items.retain(|i| !matches!(i, Item::Board { .. }));
    for i in &mut scene.items {
        match i {
            Item::Circle { fill, stroke, .. } => {
                *fill = fill.with_opacity(0.6);
                if let Some(s) = stroke {
                    s.color = s.color.with_opacity(0.6);
                }
            }
            Item::Polyline { stroke, .. }
            | Item::Path {
                stroke: Some(stroke),
                ..
            } => stroke.color = stroke.color.with_opacity(0.6),
            _ => {}
        }
    }
    paint_items(painter, xf, &scene);
}
