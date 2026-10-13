//! Editor shell (spec 18 layout): top bar, mode side panel, canvas, layers.

use egui::{Align2, Sense, Ui, pos2, vec2};
use sa_app::editor::{EditorMode, Store};
use sa_infra::i18n::Language;

use crate::fonts;
use crate::icons::Icon;
use crate::theme;
use crate::widgets::{self, Kit};

/// Actions the top bar asks the app to perform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellAction {
    New,
    Save,
    Open,
    Export(sa_infra::export::ExportFormat),
    Stats,
    Help,
    Print,
    Language(Language),
    /// Turn the local MCP agent server on/off.
    Agent(bool),
}

const MODE_ICONS: [(EditorMode, Icon); 6] = [
    (EditorMode::Pin, Icon::Pin),
    (EditorMode::Select, Icon::MousePointer2),
    (EditorMode::Thread, Icon::Spline),
    (EditorMode::Generate, Icon::Wand2),
    (EditorMode::Pan, Icon::Hand),
    (EditorMode::Play, Icon::Play),
];

pub fn top_bar(
    ui: &mut Ui,
    kit: &mut Kit,
    store: &mut Store,
    dpi: &mut f64,
    agent: &crate::agent::AgentStatus,
) -> Option<ShellAction> {
    let mut action = None;
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(
            egui::RichText::new("StringArtIt")
                .font(fonts::extrabold(15.0))
                .color(kit.t.text_primary),
        );
        ui.add_space(12.0);
        for (icon, key, a) in [
            (Icon::FilePlus, "menus:file.new", ShellAction::New),
            (Icon::Save, "menus:file.save", ShellAction::Save),
            (Icon::FolderOpen, "menus:file.open", ShellAction::Open),
        ] {
            if widgets::icon_button(ui, kit, icon, &kit.tr(key), true).clicked() {
                action = Some(a);
            }
        }
        if let Some(a) = export_menu(ui, kit, dpi) {
            action = Some(a);
        }

        // Centre the mode switcher in the remaining space.
        let items: Vec<(Icon, String)> = MODE_ICONS
            .iter()
            .map(|(m, i)| (*i, kit.tr(&format!("common:modes.{}", m.key()))))
            .collect();
        let selected = MODE_ICONS
            .iter()
            .position(|(m, _)| *m == store.editor.mode)
            .unwrap_or(0);
        let right_w = 360.0;
        let seg_w_guess = 560.0;
        let space = (ui.available_width() - right_w - seg_w_guess).max(0.0) / 2.0;
        ui.add_space(space);
        if let Some(i) = widgets::segmented(ui, kit, &items, selected) {
            store.set_mode(MODE_ICONS[i].0);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if let Some(lang) = language_switcher(ui, kit) {
                action = Some(ShellAction::Language(lang));
            }
            for (icon, key, a) in [
                (Icon::Printer, "editorShell:print", ShellAction::Print),
                (Icon::HelpCircle, "editorShell:help", ShellAction::Help),
                (Icon::BarChart3, "editorShell:stats", ShellAction::Stats),
            ] {
                let label = kit.tr(key);
                if widgets::button(ui, kit, Some(icon), Some(&label), false, true).clicked() {
                    action = Some(a);
                }
            }
            if let Some(on) = agent_popover(ui, kit, agent) {
                action = Some(ShellAction::Agent(on));
            }
            if widgets::icon_button(
                ui,
                kit,
                Icon::Redo2,
                &kit.tr("editorShell:redo"),
                store.can_redo(),
            )
            .clicked()
            {
                store.redo();
            }
            if widgets::icon_button(
                ui,
                kit,
                Icon::Undo2,
                &kit.tr("editorShell:undo"),
                store.can_undo(),
            )
            .clicked()
            {
                store.undo();
            }
        });
    });
    action
}

/// Globe button + language list (web LanguageSwitcher). Returns the pick.
pub fn language_switcher(ui: &mut Ui, kit: &mut Kit) -> Option<Language> {
    let mut picked = None;
    let current = kit.i18n.lang;
    let id = ui.id().with("language-switcher");
    let open = egui::Popup::is_id_open(ui.ctx(), id);
    let resp = widgets::button(
        ui,
        kit,
        Some(Icon::Globe),
        Some(current.native_label()),
        open,
        true,
    )
    .on_hover_text(kit.tr("common:language.trigger"));
    egui::Popup::from_toggle_button_response(&resp)
        .id(id)
        .show(|ui| {
            ui.set_min_width(180.0);
            for lang in Language::ALL {
                let (rect, r) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                if r.hovered() {
                    ui.painter().rect_filled(rect, 6.0, kit.t.bg_panel_2);
                }
                let fg = if lang == current {
                    kit.t.accent
                } else {
                    kit.t.text_primary
                };
                ui.painter().text(
                    pos2(rect.min.x + 10.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    lang.native_label(),
                    fonts::semibold(12.5),
                    fg,
                );
                if lang == current {
                    let ir = egui::Rect::from_center_size(
                        pos2(rect.max.x - 14.0, rect.center().y),
                        egui::Vec2::splat(theme::ICON_SM),
                    );
                    kit.paint_icon(ui, Icon::Check, ir, theme::ICON_SM, kit.t.accent);
                }
                if r.clicked() {
                    picked = Some(lang);
                    ui.close();
                }
            }
        });
    picked
}

/// Placeholder body for side panels whose milestone hasn't landed yet.
pub fn pending_panel(ui: &mut Ui, kit: &Kit, title: &str) {
    widgets::section_label(ui, kit, title);
}

/// Export popover (web ExportMenu): SVG / PDF vectors, PNG / JPEG at a DPI.
fn export_menu(ui: &mut Ui, kit: &mut Kit, dpi: &mut f64) -> Option<ShellAction> {
    use sa_infra::export::ExportFormat;
    let id = ui.id().with("export-menu");
    let open = egui::Popup::is_id_open(ui.ctx(), id);
    let label = kit.tr("menus:export.trigger");
    let resp = widgets::button(ui, kit, Some(Icon::Download), Some(&label), open, true);
    let mut out = None;
    egui::Popup::from_toggle_button_response(&resp)
        .id(id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(220.0);
            let d = dpi.round();
            for (icon, text, f) in [
                (
                    Icon::FileType,
                    kit.tr("menus:export.svg"),
                    ExportFormat::Svg,
                ),
                (
                    Icon::FileType,
                    kit.tr("menus:export.pdf"),
                    ExportFormat::Pdf,
                ),
                (
                    Icon::FileImage,
                    kit.trf("menus:export.png", &[("dpi", &d)]),
                    ExportFormat::Png,
                ),
                (
                    Icon::FileImage,
                    kit.trf("menus:export.jpeg", &[("dpi", &d)]),
                    ExportFormat::Jpeg,
                ),
            ] {
                let (rect, r) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                if r.hovered() {
                    ui.painter().rect_filled(rect, 6.0, kit.t.bg_panel_2);
                }
                let ir = egui::Rect::from_center_size(
                    pos2(rect.min.x + 16.0, rect.center().y),
                    egui::Vec2::splat(theme::ICON_SM),
                );
                kit.paint_icon(ui, icon, ir, theme::ICON_SM, kit.t.text_secondary);
                ui.painter().text(
                    pos2(rect.min.x + 32.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    fonts::semibold(12.5),
                    kit.t.text_primary,
                );
                if r.clicked() {
                    out = Some(ShellAction::Export(f));
                    ui.close();
                }
            }
            ui.separator();
            widgets::property_row(ui, kit, &kit.tr("menus:export.dpi"), |ui| {
                if let Some(v) = widgets::number(ui, kit, *dpi, 10.0..=1200.0, 1.0, 0) {
                    *dpi = v;
                }
            });
        });
    out
}

/// Bot button + popover for the local MCP agent server.
fn agent_popover(ui: &mut Ui, kit: &mut Kit, status: &crate::agent::AgentStatus) -> Option<bool> {
    let mut out = None;
    let tr = |kit: &Kit, k: &str| kit.tr(&format!("desktop:agent.{k}"));
    let id = ui.id().with("agent-popover");
    let open = egui::Popup::is_id_open(ui.ctx(), id);
    let resp = widgets::button(ui, kit, Some(Icon::Bot), None, open || status.enabled, true)
        .on_hover_text(tr(kit, "title"));
    egui::Popup::from_toggle_button_response(&resp)
        .id(id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(320.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            widgets::section_label(ui, kit, &tr(kit, "title"));
            if let Some(on) = widgets::checkbox_field(ui, &tr(kit, "enable"), status.enabled, true)
            {
                out = Some(on);
            }
            let secondary = kit.t.text_secondary;
            let small = move |t: String| egui::RichText::new(t).size(11.5).color(secondary);
            if let Some(url) = &status.url {
                ui.label(small(tr(kit, "running")));
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(url)
                            .font(fonts::mono(12.0))
                            .color(kit.t.text_primary),
                    );
                    let copied_id = ui.id().with("copied");
                    let copied = ui
                        .data(|d| d.get_temp::<f64>(copied_id))
                        .is_some_and(|t| ui.input(|i| i.time) - t < 1.5);
                    let label = tr(kit, if copied { "copied" } else { "copy" });
                    if widgets::button(ui, kit, Some(Icon::Copy), Some(&label), false, true)
                        .clicked()
                    {
                        ui.ctx().copy_text(url.clone());
                        let now = ui.input(|i| i.time);
                        ui.data_mut(|d| d.insert_temp(copied_id, now));
                    }
                });
                ui.add(egui::Label::new(small(tr(kit, "hint"))).wrap());
                let cmd = format!("claude mcp add --transport http string-art-it {url}");
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(cmd)
                            .font(fonts::mono(11.0))
                            .color(kit.t.text_primary),
                    )
                    .wrap(),
                );
            } else {
                ui.add(egui::Label::new(small(tr(kit, "off"))).wrap());
            }
            if let Some(e) = &status.error {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(kit.trf("desktop:agent.error", &[("error", e)]))
                            .size(11.5)
                            .color(kit.t.danger),
                    )
                    .wrap(),
                );
            }
        });
    out
}
