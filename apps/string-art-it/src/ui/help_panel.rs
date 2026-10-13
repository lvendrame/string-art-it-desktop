//! Help overlay (specs 20, 28): 8 tabs incl. About with a mailto contact form.

use egui::{Align2, Sense, Ui, pos2, vec2};
use sa_app::editor::EditorMode;

use super::help_content::{HELP_TABS, HelpTab};
use crate::fonts;
use crate::icons::Icon;
use crate::theme;
use crate::widgets::Kit;

const CONTACT_EMAIL: &str = "lfsvendrame@gmail.com";
const APP_NAME: &str = "StringArtIt";
const AUTHOR_NAME: &str = "Luís Fernando Saquetim Vendrame";
const SUBJECTS: [(&str, &str); 4] = [
    ("question", "Question"),
    ("support", "Support"),
    ("issue", "Issue"),
    ("other", "Other"),
];

#[derive(Default)]
pub struct HelpPanel {
    tab: Option<&'static str>,
    name: String,
    subject: Option<usize>,
    message: String,
}

/// Help tab opened by default for the live mode (web `MODE_TO_HELP_TAB`).
fn tab_for_mode(m: EditorMode) -> &'static str {
    match m {
        EditorMode::Pin => "pin",
        EditorMode::Thread => "thread",
        EditorMode::Pan => "pan",
        EditorMode::Play => "play",
        EditorMode::Select | EditorMode::Generate => "edit",
    }
}

impl HelpPanel {
    /// Called when the overlay opens: re-default the tab to the current mode.
    pub fn open(&mut self, mode: EditorMode) {
        self.tab = Some(tab_for_mode(mode));
    }

    pub fn set_tab(&mut self, id: &str) {
        self.tab = HELP_TABS.iter().find(|t| t.id == id).map(|t| t.id);
    }

    pub fn show(&mut self, ui: &mut Ui, kit: &mut Kit, mode: EditorMode) -> bool {
        let active = *self.tab.get_or_insert_with(|| tab_for_mode(mode));
        let title = kit.tr("editorShell:help");
        super::overlay::show(ui, kit, &title, |ui, kit| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for tab in HELP_TABS {
                    if tab_button(ui, kit, tab, tab.id == active) {
                        self.tab = Some(tab.id);
                    }
                }
            });
            let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().rect_filled(line, 0.0, kit.t.border);
            ui.add_space(20.0);
            ui.set_max_width(720.0);
            let tab = HELP_TABS
                .iter()
                .find(|t| t.id == active)
                .unwrap_or(&HELP_TABS[0]);
            if tab.id == "about" {
                self.about(ui, kit);
            } else {
                sections(ui, kit, tab);
            }
        })
    }

    fn about(&mut self, ui: &mut Ui, kit: &mut Kit) {
        ui.spacing_mut().item_spacing.y = 8.0;
        heading(ui, kit, &atr(kit, "appInfo.heading"));
        let version = env!("CARGO_PKG_VERSION");
        for (label, value, mono) in [
            (atr(kit, "appInfo.name"), APP_NAME.to_owned(), true),
            (
                atr(kit, "appInfo.description"),
                kit.tr("desktop:about.descriptionValue"),
                false,
            ),
            (atr(kit, "appInfo.version"), version.to_owned(), true),
            (atr(kit, "appInfo.author"), AUTHOR_NAME.to_owned(), false),
        ] {
            ui.horizontal(|ui| {
                ui.set_max_width(420.0);
                ui.allocate_ui_with_layout(
                    vec2(110.0, 18.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(110.0);
                        ui.label(
                            egui::RichText::new(label)
                                .size(12.0)
                                .color(kit.t.text_secondary),
                        );
                    },
                );
                let rt = egui::RichText::new(value).color(kit.t.text_primary);
                ui.add(
                    egui::Label::new(if mono {
                        rt.font(fonts::mono(12.0))
                    } else {
                        rt.size(12.5)
                    })
                    .wrap(),
                );
            });
        }
        ui.add_space(16.0);
        heading(ui, kit, &atr(kit, "contactForm.heading"));
        ui.scope(|ui| {
            ui.set_max_width(420.0);
            ui.label(
                egui::RichText::new(atr(kit, "contactForm.intro"))
                    .size(12.0)
                    .color(kit.t.text_secondary),
            );
            ui.add_space(4.0);
            field_label(ui, kit, &atr(kit, "contactForm.name.label"));
            ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(420.0));
            field_label(ui, kit, &atr(kit, "contactForm.subject.label"));
            let selected = self
                .subject
                .map(|i| {
                    kit.tr(&format!(
                        "help:about.contactForm.subject.options.{}",
                        SUBJECTS[i].0
                    ))
                })
                .unwrap_or_else(|| atr(kit, "contactForm.subject.placeholder"));
            egui::ComboBox::from_id_salt("help-subject")
                .width(420.0)
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for (i, (key, _)) in SUBJECTS.iter().enumerate() {
                        let label =
                            kit.tr(&format!("help:about.contactForm.subject.options.{key}"));
                        ui.selectable_value(&mut self.subject, Some(i), label);
                    }
                });
            field_label(ui, kit, &atr(kit, "contactForm.message.label"));
            ui.add(
                egui::TextEdit::multiline(&mut self.message)
                    .desired_rows(5)
                    .desired_width(420.0),
            );
            ui.add_space(4.0);
            let can_send = !self.name.trim().is_empty()
                && self.subject.is_some()
                && !self.message.trim().is_empty();
            if send_button(ui, kit, &atr(kit, "contactForm.send"), can_send)
                && let Some(url) = self.mailto(kit)
                && let Err(e) = open::that(&url)
            {
                log::warn!("mailto failed: {e}");
            }
        });
    }

    fn mailto(&self, kit: &Kit) -> Option<String> {
        let subject = SUBJECTS[self.subject?].1;
        let body = kit.trf(
            "help:about.contactForm.mailBody",
            &[("name", &self.name), ("message", &self.message)],
        );
        Some(format!(
            "mailto:{CONTACT_EMAIL}?subject={}&body={}",
            encode_uri_component(&format!("{APP_NAME} - {subject}")),
            encode_uri_component(&body)
        ))
    }
}

/// JS `encodeURIComponent`.
fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn atr(kit: &Kit, k: &str) -> String {
    kit.tr(&format!("help:about.{k}"))
}

fn heading(ui: &mut Ui, kit: &Kit, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .font(fonts::extrabold(12.5))
            .color(kit.t.text_secondary),
    );
}

fn field_label(ui: &mut Ui, kit: &Kit, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .color(kit.t.text_secondary),
    );
}

fn sections(ui: &mut Ui, kit: &Kit, tab: &HelpTab) {
    for s in tab.sections {
        heading(ui, kit, &kit.tr(s.heading));
        ui.add_space(0.0);
        for item in s.items {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                ui.allocate_ui_with_layout(
                    vec2(160.0, 16.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(160.0);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(kit.tr(item.label))
                                    .font(fonts::extrabold(12.5))
                                    .color(kit.t.text_primary),
                            )
                            .wrap(),
                        );
                    },
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(kit.tr(item.description))
                            .size(12.5)
                            .color(kit.t.text_secondary),
                    )
                    .wrap(),
                );
            });
        }
        ui.add_space(12.0);
    }
}

fn tab_button(ui: &mut Ui, kit: &mut Kit, tab: &HelpTab, active: bool) -> bool {
    let label = kit.tr(tab.label);
    let font = fonts::extrabold(12.0);
    let tw = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.clone(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
    });
    let (r, resp) = ui.allocate_exact_size(vec2(14.0 + 6.0 + tw + 28.0, 36.0), Sense::click());
    let fg = if active || resp.hovered() {
        kit.t.text_primary
    } else {
        kit.t.text_secondary
    };
    let icon_r =
        egui::Rect::from_center_size(pos2(r.min.x + 14.0 + 7.0, r.center().y), vec2(14.0, 14.0));
    kit.paint_icon(ui, tab.icon, icon_r, 14.0, fg);
    ui.painter().text(
        pos2(icon_r.max.x + 6.0, r.center().y),
        Align2::LEFT_CENTER,
        &label,
        font,
        fg,
    );
    if active {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(pos2(r.min.x, r.max.y - 2.0), vec2(r.width(), 2.0)),
            0.0,
            kit.t.accent,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn send_button(ui: &mut Ui, kit: &mut Kit, label: &str, enabled: bool) -> bool {
    let font = fonts::extrabold(12.0);
    let tw = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_owned(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
    });
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (r, resp) = ui.allocate_exact_size(vec2(16.0 + 14.0 + 6.0 + tw + 16.0, 32.0), sense);
    let (fill, fg) = if enabled {
        (kit.t.accent_strong, egui::Color32::WHITE)
    } else {
        (kit.t.bg_panel_2, kit.t.text_tertiary)
    };
    ui.painter().rect(
        r,
        theme::RADIUS_SM,
        fill,
        egui::Stroke::new(1.0, if enabled { fill } else { kit.t.border }),
        egui::StrokeKind::Inside,
    );
    let icon_r =
        egui::Rect::from_center_size(pos2(r.min.x + 16.0 + 7.0, r.center().y), vec2(14.0, 14.0));
    kit.paint_icon(ui, Icon::Send, icon_r, 14.0, fg);
    ui.painter().text(
        pos2(icon_r.max.x + 6.0, r.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        fg,
    );
    enabled
        && resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_like_js() {
        assert_eq!(
            encode_uri_component("StringArtIt - Issue"),
            "StringArtIt%20-%20Issue"
        );
        assert_eq!(
            encode_uri_component("Name: Zé\n\nhi?&"),
            "Name%3A%20Z%C3%A9%0A%0Ahi%3F%26"
        );
    }

    #[test]
    fn every_help_key_resolves_in_english() {
        let i18n = sa_infra::i18n::I18n::new(sa_infra::i18n::Language::En);
        for tab in HELP_TABS {
            assert!(i18n.try_t(tab.label).is_some(), "{}", tab.label);
            for s in tab.sections {
                assert!(i18n.try_t(s.heading).is_some(), "{}", s.heading);
                for it in s.items {
                    assert!(i18n.try_t(it.label).is_some(), "{}", it.label);
                    assert!(i18n.try_t(it.description).is_some(), "{}", it.description);
                }
            }
        }
    }
}
