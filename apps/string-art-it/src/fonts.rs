//! UI fonts (spec 18): Manrope for chrome text, JetBrains Mono for every
//! measurement/count/coordinate. Both are bundled (OFL). CJK UI languages add
//! an installed system face as fallback (spec 24); a 10-glyph Noto Sans CJK
//! subset always covers the native language names in the switcher.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

pub const MANROPE_SEMIBOLD: &str = "manrope-semibold";
pub const MANROPE_EXTRABOLD: &str = "manrope-extrabold";

macro_rules! font {
    ($file:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/ui/",
            $file
        ))
    };
}

pub fn install(ctx: &egui::Context, lang: sa_infra::i18n::Language) {
    let mut fonts = FontDefinitions::default();
    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add("manrope", font!("Manrope-Regular.ttf"));
    add(MANROPE_SEMIBOLD, font!("Manrope-SemiBold.ttf"));
    add(MANROPE_EXTRABOLD, font!("Manrope-ExtraBold.ttf"));
    add("jetbrains-mono", font!("JetBrainsMono-Regular.ttf"));
    // CJK fallbacks, shifted onto Manrope's baseline.
    let manrope: &[u8] = font!("Manrope-Regular.ttf");
    let mut fallback = |name: &str, bytes: &'static [u8], index: u32| {
        let mut data = FontData::from_static(bytes);
        data.index = index;
        data.tweak.y_offset_factor =
            sa_infra::system_fonts::baseline_offset_factor(manrope, bytes, index);
        fonts.font_data.insert(name.to_owned(), Arc::new(data));
        name.to_owned()
    };
    let mut fallbacks = Vec::new();
    if let Some(f) = sa_infra::system_fonts::Script::for_language(lang)
        .and_then(sa_infra::system_fonts::cjk_font)
    {
        fallbacks.push(fallback("cjk-system", f.bytes, f.index));
    }
    fallbacks.push(fallback(
        "cjk-language-names",
        font!("NotoSansCJK-LanguageNames.otf"),
        0,
    ));

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "manrope".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains-mono".to_owned());
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .extend(fallbacks.iter().cloned());
    }
    for weight in [MANROPE_SEMIBOLD, MANROPE_EXTRABOLD] {
        let mut chain = vec![weight.to_owned()];
        chain.extend(
            fonts.families[&FontFamily::Proportional]
                .iter()
                .skip(1)
                .cloned(),
        );
        fonts
            .families
            .insert(FontFamily::Name(weight.into()), chain);
    }
    ctx.set_fonts(fonts);
}

/// Bold/semibold text uses a named family since egui has no weight axis.
pub fn semibold(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(MANROPE_SEMIBOLD.into()))
}

pub fn extrabold(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(MANROPE_EXTRABOLD.into()))
}

pub fn mono(size: f32) -> egui::FontId {
    egui::FontId::monospace(size)
}
