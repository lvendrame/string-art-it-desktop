//! Layers panel (specs 13, 36): Pin / Thread tabs, rows with visibility, lock
//! and inline rename, footer actions.

use egui::{Align2, CornerRadius, Sense, Ui, pos2, vec2};
use sa_app::document::layers::Direction;
use sa_app::editor::{LayerPanelTab, Store};

use crate::fonts;
use crate::icons::Icon;
use crate::theme;
use crate::widgets::Kit;

/// Inline rename state (double-click a name).
#[derive(Default)]
pub struct LayersPanelState {
    editing: Option<(String, String)>,
}

struct Row {
    id: String,
    name: String,
    visible: bool,
    locked: bool,
}

enum Action {
    Select(String),
    ToggleVisible(String),
    ToggleLocked(String),
    Rename(String, String),
    Add,
    Duplicate,
    MergeUp,
    Move(Direction),
    Delete,
}

impl LayersPanelState {
    pub fn show(&mut self, ui: &mut Ui, kit: &mut Kit, store: &mut Store) {
        let is_pin = store.editor.layer_panel_tab == LayerPanelTab::Pin;
        // Tabs
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            for (tab, key) in [
                (LayerPanelTab::Pin, "panels:layersPanel.pinLayersTab"),
                (LayerPanelTab::Thread, "panels:layersPanel.threadLayersTab"),
            ] {
                let active = store.editor.layer_panel_tab == tab;
                let label = kit.tr(key);
                let font = fonts::semibold(12.5);
                let w = ui.fonts_mut(|f| {
                    f.layout_no_wrap(label.clone(), font.clone(), egui::Color32::WHITE)
                        .size()
                        .x
                });
                let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
                let fg = if active || resp.hovered() {
                    kit.t.text_primary
                } else {
                    kit.t.text_secondary
                };
                ui.painter().text(
                    pos2(r.min.x, r.center().y - 2.0),
                    Align2::LEFT_CENTER,
                    &label,
                    font,
                    fg,
                );
                if active {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(pos2(r.min.x, r.max.y - 2.0), vec2(w, 2.0)),
                        0.0,
                        kit.t.accent,
                    );
                }
                if resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    store.editor.layer_panel_tab = tab;
                }
            }
        });
        let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(line, 0.0, kit.t.border);

        let rows: Vec<Row> = if is_pin {
            store
                .doc
                .pin_layers
                .iter()
                .map(|l| Row {
                    id: l.id.clone(),
                    name: l.name.clone(),
                    visible: l.visible,
                    locked: l.locked,
                })
                .collect()
        } else {
            store
                .doc
                .thread_layers
                .iter()
                .map(|l| Row {
                    id: l.id.clone(),
                    name: l.name.clone(),
                    visible: l.visible,
                    locked: l.locked,
                })
                .collect()
        };
        let active_id = if is_pin {
            store.editor.active_pin_layer_id.clone()
        } else {
            store.editor.active_thread_layer_id.clone()
        };
        let mut action = None;

        let footer_h = 48.0;
        let list_h = (ui.available_height() - footer_h).max(60.0);
        egui::ScrollArea::vertical()
            .max_height(list_h)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for row in &rows {
                    if let Some(a) = self.row(ui, kit, row, row.id == active_id) {
                        action = Some(a);
                    }
                }
            });

        // Footer
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let idx = rows.iter().position(|r| r.id == active_id);
                let merge_ok = idx.is_some_and(|i| i > 0 && !rows[i].locked && !rows[i - 1].locked);
                let buttons = [
                    (
                        Icon::Plus,
                        "panels:layersPanel.newLayer",
                        true,
                        false,
                        Action::Add,
                    ),
                    (
                        Icon::Copy,
                        "panels:layersPanel.duplicate",
                        true,
                        false,
                        Action::Duplicate,
                    ),
                    (
                        Icon::ArrowUpToLine,
                        "panels:layersPanel.mergeUp",
                        merge_ok,
                        false,
                        Action::MergeUp,
                    ),
                    (
                        Icon::ChevronUp,
                        "panels:layersPanel.moveUp",
                        true,
                        false,
                        Action::Move(Direction::Up),
                    ),
                    (
                        Icon::ChevronDown,
                        "panels:layersPanel.moveDown",
                        true,
                        false,
                        Action::Move(Direction::Down),
                    ),
                    (
                        Icon::Trash2,
                        "panels:layersPanel.delete",
                        rows.len() > 1,
                        true,
                        Action::Delete,
                    ),
                ];
                for (icon, key, enabled, danger, a) in buttons {
                    if footer_button(ui, kit, icon, &kit.tr(key), enabled, danger).clicked() {
                        action = Some(a);
                    }
                }
            });
        });

        let Some(a) = action else { return };
        match (a, is_pin) {
            (Action::Select(id), true) => store.editor.active_pin_layer_id = id,
            (Action::Select(id), false) => store.editor.active_thread_layer_id = id,
            (Action::ToggleVisible(id), true) => store.toggle_pin_layer_visible(&id),
            (Action::ToggleVisible(id), false) => store.toggle_thread_layer_visible(&id),
            (Action::ToggleLocked(id), true) => store.toggle_pin_layer_locked(&id),
            (Action::ToggleLocked(id), false) => store.toggle_thread_layer_locked(&id),
            (Action::Rename(id, n), true) => store.rename_pin_layer(&id, &n),
            (Action::Rename(id, n), false) => store.rename_thread_layer(&id, &n),
            (Action::Add, true) => store.add_pin_layer(),
            (Action::Add, false) => store.add_thread_layer(),
            (Action::Duplicate, true) => store.duplicate_pin_layer(&active_id),
            (Action::Duplicate, false) => store.duplicate_thread_layer(&active_id),
            (Action::MergeUp, true) => store.merge_pin_layer_above(&active_id),
            (Action::MergeUp, false) => store.merge_thread_layer_above(&active_id),
            (Action::Move(d), true) => store.reorder_pin_layer(&active_id, d),
            (Action::Move(d), false) => store.reorder_thread_layer(&active_id, d),
            (Action::Delete, true) if rows.len() > 1 => store.delete_pin_layer(&active_id),
            (Action::Delete, false) if rows.len() > 1 => store.delete_thread_layer(&active_id),
            _ => {}
        }
    }

    fn row(&mut self, ui: &mut Ui, kit: &mut Kit, row: &Row, active: bool) -> Option<Action> {
        let mut action = None;
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
        let p = ui.painter();
        if active {
            p.rect_filled(
                rect,
                CornerRadius::same(theme::RADIUS_SM),
                kit.t.accent_soft,
            );
            p.rect_filled(
                egui::Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
                0.0,
                kit.t.accent,
            );
        } else if resp.hovered() {
            p.rect_filled(rect, CornerRadius::same(theme::RADIUS_SM), kit.t.bg_panel_2);
        }
        if resp.clicked() {
            action = Some(Action::Select(row.id.clone()));
        }
        // Eye + lock buttons
        let icon_btn = |ui: &mut Ui,
                        kit: &mut Kit,
                        x: f32,
                        icon: Icon,
                        color: egui::Color32,
                        tip: String,
                        salt: &str| {
            let r = egui::Rect::from_center_size(pos2(x, rect.center().y), vec2(28.0, 28.0));
            let resp = ui.interact(r, ui.id().with((salt, &row.id)), Sense::click());
            kit.paint_icon(
                ui,
                icon,
                r,
                theme::ICON_SM,
                if resp.hovered() {
                    kit.t.text_primary
                } else {
                    color
                },
            );
            resp.on_hover_text(tip).clicked()
        };
        let eye_c = if row.visible {
            kit.t.text_secondary
        } else {
            kit.t.text_tertiary
        };
        let eye_tip = kit.trf(
            if row.visible {
                "panels:layersPanel.hideLayer"
            } else {
                "panels:layersPanel.showLayer"
            },
            &[("name", &row.name)],
        );
        if icon_btn(
            ui,
            kit,
            rect.min.x + 20.0,
            if row.visible { Icon::Eye } else { Icon::EyeOff },
            eye_c,
            eye_tip,
            "eye",
        ) {
            action = Some(Action::ToggleVisible(row.id.clone()));
        }
        let lock_c = if row.locked {
            kit.t.danger
        } else {
            kit.t.text_tertiary
        };
        let lock_tip = kit.trf(
            if row.locked {
                "panels:layersPanel.unlockLayer"
            } else {
                "panels:layersPanel.lockLayer"
            },
            &[("name", &row.name)],
        );
        if icon_btn(
            ui,
            kit,
            rect.min.x + 48.0,
            if row.locked { Icon::Lock } else { Icon::Unlock },
            lock_c,
            lock_tip,
            "lock",
        ) {
            action = Some(Action::ToggleLocked(row.id.clone()));
        }
        // Name / rename
        let name_rect = egui::Rect::from_min_max(
            pos2(rect.min.x + 68.0, rect.min.y + 4.0),
            pos2(rect.max.x - 6.0, rect.max.y - 4.0),
        );
        match &mut self.editing {
            Some((id, draft)) if *id == row.id => {
                let te = ui.put(name_rect, egui::TextEdit::singleline(draft));
                if !te.has_focus() && !te.lost_focus() {
                    te.request_focus();
                }
                let done = te.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter));
                if done {
                    let name = draft.trim().to_owned();
                    let name = if name.is_empty() {
                        row.name.clone()
                    } else {
                        name
                    };
                    action = Some(Action::Rename(row.id.clone(), name));
                    self.editing = None;
                }
            }
            _ => {
                let color = if row.visible {
                    kit.t.text_primary
                } else {
                    kit.t.text_tertiary
                };
                ui.painter().text(
                    pos2(name_rect.min.x, rect.center().y),
                    Align2::LEFT_CENTER,
                    &row.name,
                    fonts::semibold(12.5),
                    color,
                );
                if resp.double_clicked() {
                    self.editing = Some((row.id.clone(), row.name.clone()));
                }
            }
        }
        action
    }
}

fn footer_button(
    ui: &mut Ui,
    kit: &mut Kit,
    icon: Icon,
    tip: &str,
    enabled: bool,
    danger: bool,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (r, resp) = ui.allocate_exact_size(vec2(36.0, 32.0), sense);
    let (fill, fg) = if !enabled {
        (kit.t.bg_panel_2, kit.t.text_tertiary)
    } else if resp.hovered() {
        (
            if danger {
                kit.t.danger_soft
            } else {
                kit.t.bg_panel
            },
            if danger {
                kit.t.danger
            } else {
                kit.t.text_primary
            },
        )
    } else {
        (
            kit.t.bg_panel_2,
            if danger {
                kit.t.danger
            } else {
                kit.t.text_secondary
            },
        )
    };
    ui.painter().rect(
        r,
        CornerRadius::same(theme::RADIUS_SM),
        fill,
        egui::Stroke::new(1.0, kit.t.border),
        egui::StrokeKind::Inside,
    );
    kit.paint_icon(ui, icon, r, theme::ICON_SM, fg);
    resp.on_hover_text(tip)
}
