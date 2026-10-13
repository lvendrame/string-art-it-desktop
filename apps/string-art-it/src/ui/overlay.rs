//! Full-bleed overlay shell shared by Statistics and Help (ui-patterns §Overlay panels).

use egui::Ui;

use crate::fonts;
use crate::icons::Icon;
use crate::widgets::{self, Kit};

/// Draws the overlay over the editor area; returns true when Close is requested.
pub fn show(ui: &mut Ui, kit: &mut Kit, title: &str, body: impl FnOnce(&mut Ui, &mut Kit)) -> bool {
    let mut close = false;
    egui::CentralPanel::no_frame()
        .frame(egui::Frame::new().fill(kit.t.bg_app).inner_margin(24))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .font(fonts::extrabold(15.0))
                        .color(kit.t.text_primary),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = widgets::icon_button(
                        ui,
                        kit,
                        Icon::X,
                        &kit.tr("common:actions.close"),
                        true,
                    )
                    .clicked();
                });
            });
            ui.add_space(20.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| body(ui, kit));
        });
    close
}
