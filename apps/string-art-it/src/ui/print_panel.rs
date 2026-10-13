//! Print preview overlay (spec 14): element toggles, scale, paper,
//! calibration, tiling; live page previews; print to a system printer or open
//! the PDF in the OS viewer.

use egui::{TextureHandle, Ui};
use sa_app::editor::Store;
use sa_app::print::{
    CALIBRATION_LENGTH_CM, Orientation, PaperSize, PrintSettings, ScaleMode, correction_factor,
};
use sa_infra::print::{
    calibration_pdf, layout, layout_pdf, open_pdf, print_pdf, printer_names, render_page_preview,
};

use crate::fonts;
use crate::icons::Icon;
use crate::widgets::{self, Kit};

/// Preview cache key: document revision, settings, grid gap.
type PreviewKey = (u64, PrintSettings, (f64, f64));

#[derive(Default)]
pub struct PrintPanel {
    calibrating: bool,
    measured: f64,
    printer: Option<String>,
    printers: Option<Vec<(String, bool)>>,
    status: Option<String>,
    previews: Option<(PreviewKey, Vec<TextureHandle>)>,
}

pub enum PrintPanelAction {
    Close,
    /// Calibration factor changed → persist in prefs.
    Calibrated(f64),
}

impl PrintPanel {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        kit: &mut Kit,
        store: &mut Store,
    ) -> Option<PrintPanelAction> {
        let mut action = None;
        let t = kit.t;
        egui::Panel::left("print-sidebar")
            .exact_size(300.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(t.bg_panel)
                    .inner_margin(16)
                    .stroke(egui::Stroke::new(1.0, t.border)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(kit.tr("printPreview:panelTitle"))
                                .font(fonts::extrabold(15.0))
                                .color(t.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if widgets::icon_button(
                                ui,
                                kit,
                                Icon::X,
                                &kit.tr("common:actions.close"),
                                true,
                            )
                            .clicked()
                            {
                                action = Some(PrintPanelAction::Close);
                            }
                        });
                    });
                    if let Some(a) = self.controls(ui, kit, store) {
                        action = Some(a);
                    }
                });
            });
        egui::CentralPanel::no_frame()
            .frame(egui::Frame::new().fill(t.bg_canvas).inner_margin(24))
            .show(ui, |ui| {
                self.previews(ui, kit, store);
            });
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            action = Some(PrintPanelAction::Close);
        }
        action
    }

    fn controls(
        &mut self,
        ui: &mut Ui,
        kit: &mut Kit,
        store: &mut Store,
    ) -> Option<PrintPanelAction> {
        let mut action = None;
        let s = &mut store.editor.print;
        widgets::section_label(ui, kit, &kit.tr("printPreview:sections.printElements"));
        let e = &mut s.elements;
        for (flag, key) in [
            (&mut e.board_outline, "boardOutline"),
            (&mut e.background, "background"),
            (&mut e.pins, "pins"),
            (&mut e.pin_guides, "pinGuides"),
            (&mut e.pin_numbers, "pinNumbers"),
            (&mut e.threads, "threads"),
            (&mut e.grid, "grid"),
        ] {
            ui.checkbox(flag, kit.tr(&format!("printPreview:elements.{key}")));
        }

        widgets::section_label(ui, kit, &kit.tr("printPreview:sections.scale"));
        for (m, key) in [
            (ScaleMode::Actual, "actualSize"),
            (ScaleMode::Fit, "fit"),
            (ScaleMode::Custom, "custom"),
        ] {
            ui.radio_value(
                &mut s.scale_mode,
                m,
                kit.tr(&format!("printPreview:scaleModes.{key}")),
            );
        }
        if s.scale_mode == ScaleMode::Custom
            && let Some(v) = widgets::number(ui, kit, s.custom_ratio, 0.01..=100.0, 0.01, 3)
        {
            s.custom_ratio = v;
        }

        widgets::section_label(ui, kit, &kit.tr("printPreview:sections.paper"));
        egui::ComboBox::from_id_salt("paper")
            .selected_text(kit.tr(&format!("printPreview:paperSizes.{}", paper_key(s.paper))))
            .show_ui(ui, |ui| {
                for p in [
                    PaperSize::A4,
                    PaperSize::A3,
                    PaperSize::Letter,
                    PaperSize::Custom,
                ] {
                    ui.selectable_value(
                        &mut s.paper,
                        p,
                        kit.tr(&format!("printPreview:paperSizes.{}", paper_key(p))),
                    );
                }
            });
        if s.paper == PaperSize::Custom {
            ui.horizontal(|ui| {
                if let Some(v) = widgets::number(ui, kit, s.custom_width_cm, 1.0..=500.0, 0.1, 1) {
                    s.custom_width_cm = v;
                }
                ui.label("×");
                if let Some(v) = widgets::number(ui, kit, s.custom_height_cm, 1.0..=500.0, 0.1, 1) {
                    s.custom_height_cm = v;
                }
                ui.label("cm");
            });
        }
        ui.horizontal(|ui| {
            for o in [Orientation::Portrait, Orientation::Landscape] {
                let key = if o == Orientation::Portrait {
                    "portrait"
                } else {
                    "landscape"
                };
                ui.radio_value(
                    &mut s.orientation,
                    o,
                    kit.tr(&format!("printPreview:orientation.{key}")),
                );
            }
        });

        widgets::section_label(ui, kit, &kit.tr("printPreview:sections.calibration"));
        ui.label(
            egui::RichText::new(format!(
                "{} {:.4}",
                kit.tr("printPreview:calibration.correctionFactor"),
                s.correction_factor
            ))
            .font(fonts::mono(12.0)),
        );
        if !self.calibrating {
            if ui
                .button(kit.tr("printPreview:calibration.testButton"))
                .clicked()
            {
                self.calibrating = true;
                self.measured = CALIBRATION_LENGTH_CM;
            }
        } else {
            ui.label(kit.trf(
                "printPreview:calibration.instructions",
                &[("length", &CALIBRATION_LENGTH_CM)],
            ));
            if ui
                .button(kit.tr("printPreview:calibration.printReference"))
                .clicked()
            {
                self.status = Some(
                    match calibration_pdf(s)
                        .and_then(|b| print_pdf(&b, self.printer.as_deref(), "calibration"))
                    {
                        Ok(t) => format!("→ {t}"),
                        Err(e) => e,
                    },
                );
            }
            widgets::property_row(
                ui,
                kit,
                &kit.tr("printPreview:calibration.measuredLabel"),
                |ui| {
                    if let Some(v) = widgets::number(ui, kit, self.measured, 0.1..=100.0, 0.01, 2) {
                        self.measured = v;
                    }
                },
            );
            ui.horizontal(|ui| {
                if ui
                    .button(kit.tr("printPreview:calibration.apply"))
                    .clicked()
                {
                    s.correction_factor = correction_factor(CALIBRATION_LENGTH_CM, self.measured);
                    action = Some(PrintPanelAction::Calibrated(s.correction_factor));
                    self.calibrating = false;
                }
                if ui
                    .button(kit.tr("printPreview:calibration.cancel"))
                    .clicked()
                {
                    self.calibrating = false;
                }
            });
        }

        widgets::section_label(ui, kit, &kit.tr("printPreview:sections.tiling"));
        let ti = &mut s.tiling;
        ui.checkbox(&mut ti.enabled, kit.tr("printPreview:tiling.enableLabel"));
        if ti.enabled {
            widgets::property_row(ui, kit, &kit.tr("printPreview:tiling.overlapLabel"), |ui| {
                if let Some(v) = widgets::number(ui, kit, ti.overlap_cm, 0.0..=10.0, 0.1, 1) {
                    ti.overlap_cm = v;
                }
            });
            ui.checkbox(&mut ti.trim_marks, kit.tr("printPreview:tiling.trimMarks"));
            ui.checkbox(
                &mut ti.alignment_marks,
                kit.tr("printPreview:tiling.alignmentMarks"),
            );
            ui.checkbox(
                &mut ti.page_numbers,
                kit.tr("printPreview:tiling.pageNumbers"),
            );
            ui.checkbox(
                &mut ti.page_coordinates,
                kit.tr("printPreview:tiling.pageCoordinates"),
            );
        }

        let gap = (store.editor.grid.gap_x, store.editor.grid.gap_y);
        let l = layout(&store.doc, &store.editor.print, gap);
        ui.label(
            egui::RichText::new(kit.trf(
                "printPreview:effective",
                &[("value", &format!("{:.3}", l.scale))],
            ))
            .font(fonts::mono(12.0))
            .color(kit.t.text_secondary),
        );
        if l.pages.len() > 1 {
            ui.label(kit.trf(
                "printPreview:tiling.pagesCount",
                &[
                    ("count", &l.pages.len()),
                    ("cols", &l.cols),
                    ("rows", &l.rows),
                ],
            ));
        }

        ui.add_space(8.0);
        let printers = self.printers.get_or_insert_with(printer_names).clone();
        if !printers.is_empty() {
            let current = self
                .printer
                .clone()
                .or_else(|| printers.iter().find(|p| p.1).map(|p| p.0.clone()));
            egui::ComboBox::from_id_salt("printer")
                .width(ui.available_width())
                .selected_text(current.clone().unwrap_or_default())
                .show_ui(ui, |ui| {
                    for (name, _) in &printers {
                        if ui
                            .selectable_label(current.as_deref() == Some(name), name)
                            .clicked()
                        {
                            self.printer = Some(name.clone());
                        }
                    }
                });
        }
        let print_label = kit.tr("printPreview:printButton");
        if widgets::primary_button(ui, kit, &print_label, ui.available_width()).clicked() {
            self.status = Some(
                match layout_pdf(&l)
                    .and_then(|b| print_pdf(&b, self.printer.as_deref(), "string-art-print"))
                {
                    Ok(t) => format!("→ {t}"),
                    Err(e) => e,
                },
            );
        }
        if ui
            .button("PDF…")
            .on_hover_text("Open the print PDF in the system viewer")
            .clicked()
            && let Err(e) = layout_pdf(&l).and_then(|b| open_pdf(&b, "string-art-print"))
        {
            self.status = Some(e);
        }
        if let Some(st) = &self.status {
            ui.label(
                egui::RichText::new(st)
                    .size(11.0)
                    .color(kit.t.text_tertiary),
            );
        }
        action
    }

    fn previews(&mut self, ui: &mut Ui, kit: &Kit, store: &Store) {
        let gap = (store.editor.grid.gap_x, store.editor.grid.gap_y);
        let key = (store.revision(), store.editor.print, gap);
        let l = layout(&store.doc, &store.editor.print, gap);
        let ppp = ui.ctx().pixels_per_point() as f64;
        // Fit the whole tile grid in the available width.
        let avail = ui.available_size();
        let cols = l.cols.max(1) as f64;
        let gap_px = 12.0;
        let page_w = ((avail.x as f64 - gap_px * (cols - 1.0)) / cols).clamp(80.0, 600.0);
        let px_per_cm = page_w / l.paper.0;
        if self.previews.as_ref().is_none_or(|(k, _)| *k != key) {
            let textures = l
                .pages
                .iter()
                .enumerate()
                .filter_map(|(i, page)| {
                    let (w, h, rgba) = render_page_preview(&l, page, px_per_cm * ppp)?;
                    let img =
                        egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], &rgba);
                    Some(ui.ctx().load_texture(
                        format!("print-page-{i}"),
                        img,
                        egui::TextureOptions::LINEAR,
                    ))
                })
                .collect();
            self.previews = Some((key, textures));
        }
        let Some((_, textures)) = &self.previews else {
            return;
        };
        let size = egui::vec2(page_w as f32, (page_w * l.paper.1 / l.paper.0) as f32);
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("print-pages")
                .spacing(egui::vec2(gap_px as f32, gap_px as f32))
                .show(ui, |ui| {
                    for (i, tex) in textures.iter().enumerate() {
                        let r = ui.add(egui::Image::new((tex.id(), size)));
                        ui.painter().rect_stroke(
                            r.rect,
                            0.0,
                            egui::Stroke::new(1.0, kit.t.border),
                            egui::StrokeKind::Outside,
                        );
                        if (i + 1) % l.cols.max(1) == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
    }
}

fn paper_key(p: PaperSize) -> &'static str {
    match p {
        PaperSize::A4 => "A4",
        PaperSize::A3 => "A3",
        PaperSize::Letter => "Letter",
        PaperSize::Custom => "custom",
    }
}
