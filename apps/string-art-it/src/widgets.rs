//! Design-system component vocabulary (spec 18): segmented mode switcher, tool
//! grid buttons, toggle chips, slide switch, icon buttons, section labels and
//! numeric/colour inputs. Every widget paints itself from [`Tokens`] so the
//! chrome matches the web app's CSS.

use egui::{
    Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
    pos2, vec2,
};
use sa_infra::i18n::I18n;

use crate::fonts;
use crate::icons::{Icon, IconCache};
use crate::theme::{self, Tokens};

/// Shared rendering context passed to every view.
pub struct Kit<'a> {
    pub t: Tokens,
    pub icons: &'a mut IconCache,
    pub i18n: I18n,
}

impl Kit<'_> {
    pub fn tr(&self, key: &str) -> String {
        self.i18n.t(key)
    }

    pub fn trf(&self, key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
        self.i18n.tf(key, args)
    }

    /// Paints `icon` centred in `rect`.
    pub fn paint_icon(&mut self, ui: &Ui, icon: Icon, rect: Rect, size: f32, color: Color32) {
        self.paint_icon_stroke(ui, icon, rect, size, color, None);
    }

    pub fn paint_icon_stroke(
        &mut self,
        ui: &Ui,
        icon: Icon,
        rect: Rect,
        size: f32,
        color: Color32,
        stroke_width: Option<f32>,
    ) {
        let tex = self.icons.texture(ui.ctx(), icon, size, stroke_width);
        let r = Rect::from_center_size(rect.center(), vec2(size, size));
        ui.painter().image(
            tex.id(),
            r,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            color,
        );
    }
}

/// The one "section label" style: 11px, 700, uppercase, 0.06em, tertiary.
pub fn section_label(ui: &mut Ui, kit: &Kit, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .font(fonts::extrabold(11.0))
            .color(kit.t.text_tertiary)
            .extra_letter_spacing(0.66),
    );
}

fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
    })
}

/// Visual state for a chrome button.
#[derive(Clone, Copy)]
struct Look {
    fill: Color32,
    stroke: Color32,
    fg: Color32,
}

fn look(kit: &Kit, resp: &Response, active: bool, enabled: bool, base_fill: Color32) -> Look {
    let t = &kit.t;
    if !enabled {
        return Look {
            fill: base_fill,
            stroke: t.border,
            fg: t.text_tertiary,
        };
    }
    if active {
        return Look {
            fill: t.accent_soft,
            stroke: t.accent,
            fg: t.accent,
        };
    }
    if resp.hovered() {
        Look {
            fill: t.bg_panel,
            stroke: t.border_strong,
            fg: t.text_primary,
        }
    } else {
        Look {
            fill: base_fill,
            stroke: t.border,
            fg: t.text_secondary,
        }
    }
}

/// `.btn` with optional icon + label (top bar Stats/Help/Print, Fit, …).
pub fn button(
    ui: &mut Ui,
    kit: &mut Kit,
    icon: Option<Icon>,
    label: Option<&str>,
    active: bool,
    enabled: bool,
) -> Response {
    let font = fonts::semibold(12.0);
    let icon_size = theme::ICON_SM;
    let pad = vec2(12.0, 7.0);
    let gap = 6.0;
    let tw = label.map(|l| text_width(ui, l, &font)).unwrap_or(0.0);
    let content_w = icon.map(|_| icon_size).unwrap_or(0.0)
        + if icon.is_some() && label.is_some() {
            gap
        } else {
            0.0
        }
        + tw;
    let size = vec2((content_w + pad.x * 2.0).max(32.0), 32.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    let l = look(kit, &resp, active, enabled, kit.t.bg_panel_2);
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(8),
        l.fill,
        Stroke::new(1.0, l.stroke),
        StrokeKind::Inside,
    );
    let mut x = rect.center().x - content_w / 2.0;
    if let Some(i) = icon {
        let r = Rect::from_min_size(
            pos2(x, rect.center().y - icon_size / 2.0),
            Vec2::splat(icon_size),
        );
        kit.paint_icon(ui, i, r, icon_size, l.fg);
        x += icon_size + gap;
    }
    if let Some(l2) = label {
        ui.painter().text(
            pos2(x, rect.center().y),
            Align2::LEFT_CENTER,
            l2,
            font,
            l.fg,
        );
    }
    resp.on_hover_cursor(if enabled {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::NotAllowed
    })
}

/// Transparent icon-only button (undo/redo): 32×32 hit target.
pub fn icon_button(
    ui: &mut Ui,
    kit: &mut Kit,
    icon: Icon,
    tooltip: &str,
    enabled: bool,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(32.0, 32.0), sense);
    let fg = if !enabled {
        kit.t.text_tertiary
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), kit.t.bg_panel_2);
        kit.t.text_primary
    } else {
        kit.t.text_secondary
    };
    kit.paint_icon(ui, icon, rect, theme::ICON_SM, fg);
    resp.on_hover_text(tooltip)
}

/// Plain text button in `.btn` style, e.g. board-setup shape options.
pub fn text_button(
    ui: &mut Ui,
    kit: &mut Kit,
    label: &str,
    active: bool,
    width: Option<f32>,
) -> Response {
    let font = fonts::semibold(12.5);
    let tw = text_width(ui, label, &font);
    let size = vec2(width.unwrap_or(tw + 24.0), 34.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let l = look(kit, &resp, active, true, kit.t.bg_panel_2);
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS_SM),
        l.fill,
        Stroke::new(1.0, l.stroke),
        StrokeKind::Inside,
    );
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, font, l.fg);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Primary CTA: `--accent-strong` fill, white text.
pub fn primary_button(ui: &mut Ui, kit: &Kit, label: &str, width: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 40.0), Sense::click());
    let fill = if resp.hovered() {
        kit.t.accent
    } else {
        kit.t.accent_strong
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(theme::RADIUS_SM), fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        fonts::extrabold(13.5),
        Color32::WHITE,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Segmented pill (mode switcher). Returns the clicked index.
pub fn segmented(
    ui: &mut Ui,
    kit: &mut Kit,
    items: &[(Icon, String)],
    selected: usize,
) -> Option<usize> {
    let font = fonts::semibold(12.5);
    let pad = 3.0;
    let widths: Vec<f32> = items
        .iter()
        .map(|(_, l)| theme::ICON_SM + 6.0 + text_width(ui, l, &font) + 24.0)
        .collect();
    let total =
        widths.iter().sum::<f32>() + 2.0 * (items.len().saturating_sub(1)) as f32 + pad * 2.0;
    let (outer, _) = ui.allocate_exact_size(vec2(total, 38.0), Sense::hover());
    ui.painter().rect(
        outer,
        CornerRadius::same(10),
        kit.t.bg_app,
        Stroke::new(1.0, kit.t.border),
        StrokeKind::Inside,
    );
    let mut clicked = None;
    let mut x = outer.min.x + pad;
    for (i, ((icon, label), w)) in items.iter().zip(&widths).enumerate() {
        let rect = Rect::from_min_size(
            pos2(x, outer.min.y + pad),
            vec2(*w, outer.height() - pad * 2.0),
        );
        let resp = ui.interact(rect, ui.id().with(("seg", i)), Sense::click());
        let active = i == selected;
        let fg = if active {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(7), kit.t.accent_soft);
            kit.t.accent
        } else if resp.hovered() {
            kit.t.text_primary
        } else {
            kit.t.text_secondary
        };
        let content_w = w - 24.0;
        let ix = rect.center().x - content_w / 2.0;
        let ir = Rect::from_min_size(
            pos2(ix, rect.center().y - theme::ICON_SM / 2.0),
            Vec2::splat(theme::ICON_SM),
        );
        kit.paint_icon(ui, *icon, ir, theme::ICON_SM, fg);
        ui.painter().text(
            pos2(ix + theme::ICON_SM + 6.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            font.clone(),
            fg,
        );
        if resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = Some(i);
        }
        x += w + 2.0;
    }
    clicked
}

/// One square-ish tool button: 16px icon on top + 10px label below.
pub fn tool_button(
    ui: &mut Ui,
    kit: &mut Kit,
    icon: Icon,
    label: &str,
    active: bool,
    size: Vec2,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let l = look(kit, &resp, active, true, kit.t.bg_app);
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS_SM),
        l.fill,
        Stroke::new(1.0, l.stroke),
        StrokeKind::Inside,
    );
    let icon_rect = Rect::from_center_size(
        pos2(rect.center().x, rect.min.y + 18.0),
        Vec2::splat(theme::ICON),
    );
    kit.paint_icon(ui, icon, icon_rect, theme::ICON, l.fg);
    ui.painter().text(
        pos2(rect.center().x, rect.max.y - 11.0),
        Align2::CENTER_CENTER,
        label,
        fonts::semibold(10.0),
        l.fg,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// 3-column tool grid. Returns the clicked index.
pub fn tool_grid(
    ui: &mut Ui,
    kit: &mut Kit,
    items: &[(Icon, String)],
    selected: Option<usize>,
) -> Option<usize> {
    let gap = 6.0;
    let w = ((ui.available_width() - gap * 2.0) / 3.0).floor();
    let mut clicked = None;
    for (row_i, row) in items.chunks(3).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (j, (icon, label)) in row.iter().enumerate() {
                let i = row_i * 3 + j;
                if tool_button(ui, kit, *icon, label, selected == Some(i), vec2(w, 56.0))
                    .on_hover_text(label)
                    .clicked()
                {
                    clicked = Some(i);
                }
            }
        });
    }
    clicked
}

/// Pill toggle chip with icon (Grid ON/OFF, Snap ON/OFF).
pub fn chip(ui: &mut Ui, kit: &mut Kit, icon: Icon, label: &str, active: bool) -> Response {
    let font = fonts::semibold(11.5);
    let w = theme::ICON_SM + 6.0 + text_width(ui, label, &font) + 24.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
    let (fill, stroke, fg) = if active {
        (kit.t.accent_soft, kit.t.accent, kit.t.accent)
    } else if resp.hovered() {
        (kit.t.bg_panel, kit.t.border_strong, kit.t.text_primary)
    } else {
        (kit.t.bg_panel_2, kit.t.border, kit.t.text_secondary)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(255),
        fill,
        Stroke::new(1.0, stroke),
        StrokeKind::Inside,
    );
    let ir = Rect::from_min_size(
        pos2(rect.min.x + 12.0, rect.center().y - theme::ICON_SM / 2.0),
        Vec2::splat(theme::ICON_SM),
    );
    kit.paint_icon(ui, icon, ir, theme::ICON_SM, fg);
    ui.painter().text(
        pos2(ir.max.x + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        fg,
    );
    resp.on_hover_text(label)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Labeled slide switch (spec 18): returns `Some(right)` when changed.
pub fn slide_switch(
    ui: &mut Ui,
    kit: &Kit,
    left: &str,
    right: &str,
    is_right: bool,
) -> Option<bool> {
    let mut out = None;
    ui.horizontal(|ui| {
        let lf = |active: bool| {
            if active {
                fonts::extrabold(12.0)
            } else {
                fonts::semibold(12.0)
            }
        };
        let lc = |active: bool| {
            if active {
                kit.t.accent
            } else {
                kit.t.text_secondary
            }
        };
        if ui
            .add(
                egui::Label::new(
                    egui::RichText::new(left)
                        .font(lf(!is_right))
                        .color(lc(!is_right)),
                )
                .sense(Sense::click()),
            )
            .clicked()
        {
            out = Some(false);
        }
        let (hit, resp) = ui.allocate_exact_size(vec2(44.0, 32.0), Sense::click());
        let track = Rect::from_center_size(hit.center(), vec2(40.0, 20.0));
        ui.painter().rect(
            track,
            CornerRadius::same(255),
            kit.t.bg_app,
            Stroke::new(1.0, kit.t.border),
            StrokeKind::Inside,
        );
        let t = ui.ctx().animate_bool_with_time(resp.id, is_right, 0.15);
        let x = egui::lerp(track.min.x + 10.0..=track.max.x - 10.0, t);
        ui.painter()
            .circle_filled(pos2(x, track.center().y), 7.0, kit.t.accent);
        if resp.clicked() {
            out = Some(!is_right);
        }
        if ui
            .add(
                egui::Label::new(
                    egui::RichText::new(right)
                        .font(lf(is_right))
                        .color(lc(is_right)),
                )
                .sense(Sense::click()),
            )
            .clicked()
        {
            out = Some(true);
        }
    });
    out.filter(|v| *v != is_right)
}

/// Monospace numeric input (DragValue). Returns the new value when changed.
pub fn number(
    ui: &mut Ui,
    kit: &Kit,
    value: f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
    decimals: usize,
) -> Option<f64> {
    let mut v = value;
    let resp = ui.add(
        egui::DragValue::new(&mut v)
            .range(range)
            .speed(speed)
            .max_decimals(decimals)
            .custom_formatter(move |n, _| {
                let s = format!("{n:.decimals$}");
                if s.contains('.') {
                    s.trim_end_matches('0').trim_end_matches('.').to_owned()
                } else {
                    s
                }
            }),
    );
    let _ = kit;
    (resp.changed() && v != value).then_some(v)
}

/// Labelled property row: label on the left, widget right-aligned.
pub fn property_row<R>(ui: &mut Ui, kit: &Kit, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(12.5)
                .color(kit.t.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add)
            .inner
    })
    .inner
}

/// Colour swatch button with picker. Returns the new `#rrggbb` when changed.
pub fn colour(ui: &mut Ui, hex: &str) -> Option<String> {
    let c = sa_render::Rgba::parse(hex);
    let mut rgb = [c.r, c.g, c.b];
    let resp = egui::color_picker::color_edit_button_srgb(ui, &mut rgb);
    resp.changed()
        .then(|| sa_render::Rgba::rgb(rgb[0], rgb[1], rgb[2]).to_hex())
}

/// Thin vertical divider used in toolbars.
pub fn vdivider(ui: &mut Ui, kit: &Kit, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, height), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, kit.t.border);
}

/// Slider with label and monospace value readout (web SliderField).
#[allow(clippy::too_many_arguments)]
pub fn slider_field(
    ui: &mut Ui,
    kit: &Kit,
    label: &str,
    value: f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    enabled: bool,
    format: impl Fn(f64) -> String,
) -> Option<f64> {
    let mut out = None;
    ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(label)
                    .size(12.5)
                    .color(kit.t.text_secondary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format(value))
                        .font(fonts::mono(12.0))
                        .color(kit.t.text_primary),
                );
            });
        });
        let mut v = value;
        ui.spacing_mut().slider_width = ui.available_width();
        let r = ui.add(
            egui::Slider::new(&mut v, range)
                .step_by(step)
                .show_value(false),
        );
        if r.changed() && v != value {
            out = Some(v);
        }
    });
    out
}

pub fn checkbox_field(ui: &mut Ui, label: &str, checked: bool, enabled: bool) -> Option<bool> {
    let mut c = checked;
    let r = ui.add_enabled(enabled, egui::Checkbox::new(&mut c, label));
    (r.changed() && c != checked).then_some(c)
}

/// Add/remove colour list (web ColourListField). Returns the new list.
pub fn colour_list_field(
    ui: &mut Ui,
    kit: &mut Kit,
    title: &str,
    colours: &[String],
    max: usize,
    palette: &[&str],
    note: Option<&str>,
) -> Option<Vec<String>> {
    let mut out = None;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .size(12.5)
                .color(kit.t.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(colours.len() > 1, egui::Button::new("−"))
                .on_hover_text(kit.tr("panels:threadPropertiesPanel.removeColour"))
                .clicked()
            {
                out = Some(colours[..colours.len() - 1].to_vec());
            }
            if ui
                .add_enabled(colours.len() < max, egui::Button::new("+"))
                .on_hover_text(kit.tr("panels:threadPropertiesPanel.addColour"))
                .clicked()
            {
                let mut v = colours.to_vec();
                v.push(palette[colours.len() % palette.len()].to_owned());
                out = Some(v);
            }
        });
    });
    ui.horizontal_wrapped(|ui| {
        for (i, c) in colours.iter().enumerate() {
            if let Some(nc) = colour(ui, c) {
                let mut v = colours.to_vec();
                v[i] = nc;
                out = Some(v);
            }
        }
    });
    if let Some(n) = note {
        ui.label(egui::RichText::new(n).size(11.0).color(kit.t.text_tertiary));
    }
    out
}
