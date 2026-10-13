//! Design-system tokens (docs/specs/18-design-system.md, web `src/ui/theme.css`).
//!
//! Every chrome colour resolves to one of these; never hardcode a literal elsewhere.

use egui::{Color32, CornerRadius, Stroke, Visuals};

/// Converts an `oklch(L C H)` colour (L in 0..=1, H in degrees) to sRGB.
pub fn oklch(l: f32, c: f32, h_deg: f32) -> Color32 {
    oklch_alpha(l, c, h_deg, 1.0)
}

pub fn oklch_alpha(l: f32, c: f32, h_deg: f32, alpha: f32) -> Color32 {
    let h = h_deg.to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());

    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    let r = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let g = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
    let b = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;

    let to_u8 = |v: f32| (linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(to_u8(r), to_u8(g), to_u8(b), (alpha * 255.0).round() as u8)
}

fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Resolved design tokens.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub bg_app: Color32,
    pub bg_panel: Color32,
    pub bg_panel_2: Color32,
    pub bg_canvas: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub accent: Color32,
    pub accent_strong: Color32,
    pub accent_soft: Color32,
    pub accent_soft_2: Color32,
    pub danger: Color32,
    pub danger_soft: Color32,
}

const HUE: f32 = 264.0;
const ACCENT_HUE: f32 = 292.0;

impl Tokens {
    pub fn dark() -> Self {
        Self {
            bg_app: oklch(0.19, 0.012, HUE),
            bg_panel: oklch(0.225, 0.014, HUE),
            bg_panel_2: oklch(0.26, 0.015, HUE),
            bg_canvas: oklch(0.135, 0.010, HUE),
            border: oklch(0.33, 0.014, HUE),
            border_strong: oklch(0.41, 0.016, HUE),
            text_primary: oklch(0.93, 0.006, HUE),
            text_secondary: oklch(0.68, 0.014, HUE),
            text_tertiary: oklch(0.50, 0.014, HUE),
            accent: oklch(0.68, 0.19, ACCENT_HUE),
            accent_strong: oklch(0.55, 0.19, ACCENT_HUE),
            accent_soft: oklch_alpha(0.68, 0.19, ACCENT_HUE, 0.08),
            accent_soft_2: oklch_alpha(0.68, 0.19, ACCENT_HUE, 0.28),
            danger: oklch(0.66, 0.18, 22.0),
            danger_soft: oklch_alpha(0.66, 0.18, 22.0, 0.16),
        }
    }
}

pub const RADIUS_SM: u8 = 6;
pub const RADIUS_MD: u8 = 10;
pub const ICON: f32 = 16.0;
pub const ICON_SM: f32 = 14.0;
pub const TOP_BAR_HEIGHT: f32 = 56.0;
pub const SIDE_PANEL_WIDTH: f32 = 248.0;
pub const LAYERS_PANEL_WIDTH: f32 = 300.0;

/// Applies the tokens to egui's global style.
pub fn apply(ctx: &egui::Context, t: &Tokens) {
    ctx.all_styles_mut(|style| {
        let mut v = Visuals::dark();
        v.panel_fill = t.bg_panel;
        v.window_fill = t.bg_panel;
        v.extreme_bg_color = t.bg_app;
        v.faint_bg_color = t.bg_panel_2;
        v.override_text_color = None;
        v.hyperlink_color = t.accent;
        v.selection.bg_fill = t.accent_soft_2;
        v.selection.stroke = Stroke::new(1.0, t.accent);
        v.window_stroke = Stroke::new(1.0, t.border);
        v.window_corner_radius = CornerRadius::same(RADIUS_MD);
        v.menu_corner_radius = CornerRadius::same(RADIUS_MD);

        let w = &mut v.widgets;
        for (state, bg, fg, border) in [
            (&mut w.noninteractive, t.bg_panel, t.text_primary, t.border),
            (&mut w.inactive, t.bg_panel_2, t.text_secondary, t.border),
            (&mut w.hovered, t.bg_panel, t.text_primary, t.border_strong),
            (&mut w.active, t.accent_soft, t.accent, t.accent),
            (&mut w.open, t.bg_panel_2, t.text_primary, t.border_strong),
        ] {
            state.bg_fill = bg;
            state.weak_bg_fill = bg;
            state.fg_stroke = Stroke::new(1.0, fg);
            state.bg_stroke = Stroke::new(1.0, border);
            state.corner_radius = CornerRadius::same(RADIUS_SM);
            state.expansion = 0.0;
        }
        style.visuals = v;

        let s = &mut style.spacing;
        s.item_spacing = egui::vec2(8.0, 8.0);
        s.button_padding = egui::vec2(10.0, 6.0);
        s.interact_size.y = 32.0;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklch_white_and_black() {
        assert_eq!(oklch(1.0, 0.0, 0.0), Color32::WHITE);
        assert_eq!(oklch(0.0, 0.0, 0.0), Color32::BLACK);
    }

    #[test]
    fn accent_is_violet() {
        // oklch(68% 0.19 292) ≈ #9d7cff-ish violet used by the web favicon/accent.
        let c = oklch(0.68, 0.19, 292.0);
        assert!(c.b() > c.r() && c.r() > c.g(), "{c:?}");
    }
}
