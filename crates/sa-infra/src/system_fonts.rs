//! System CJK fallback fonts (spec 24 desktop notes). The bundled UI fonts
//! (Manrope / JetBrains Mono) have no CJK glyphs; the web relied on the
//! browser's fallback. Here fontdb locates an installed face for the active
//! language's script; it is read once and kept for the process (only CJK
//! UI languages pay for it).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::i18n::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Japanese,
    SimplifiedChinese,
    Korean,
}

impl Script {
    pub const ALL: [Script; 3] = [Script::Japanese, Script::SimplifiedChinese, Script::Korean];

    pub fn for_language(lang: Language) -> Option<Script> {
        match lang {
            Language::Ja => Some(Script::Japanese),
            Language::ZhCn => Some(Script::SimplifiedChinese),
            Language::Ko => Some(Script::Korean),
            _ => None,
        }
    }

    /// Preferred families, macOS → Windows → Linux.
    fn families(self) -> &'static [&'static str] {
        match self {
            Script::Japanese => &[
                "Hiragino Sans",
                "Hiragino Kaku Gothic ProN",
                "Yu Gothic UI",
                "Yu Gothic",
                "Meiryo",
                "Noto Sans CJK JP",
                "Noto Sans JP",
                "Source Han Sans JP",
                "IPAexGothic",
            ],
            Script::SimplifiedChinese => &[
                "PingFang SC",
                "Hiragino Sans GB",
                "Microsoft YaHei UI",
                "Microsoft YaHei",
                "Noto Sans CJK SC",
                "Noto Sans SC",
                "Source Han Sans SC",
                "WenQuanYi Micro Hei",
                "Heiti SC",
                "STHeiti",
            ],
            Script::Korean => &[
                "Apple SD Gothic Neo",
                "Malgun Gothic",
                "Noto Sans CJK KR",
                "Noto Sans KR",
                "Source Han Sans KR",
                "NanumGothic",
            ],
        }
    }

    /// A character the face must contain.
    fn probe(self) -> char {
        match self {
            Script::Japanese => 'あ',
            Script::SimplifiedChinese => '简',
            Script::Korean => '한',
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SystemFont {
    pub bytes: &'static [u8],
    pub index: u32,
    pub script: Script,
}

fn database() -> &'static fontdb::Database {
    static DB: OnceLock<fontdb::Database> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        db
    })
}

/// The installed face for `script`, if any (scans/reads once per script).
pub fn cjk_font(script: Script) -> Option<SystemFont> {
    static CACHE: OnceLock<Mutex<HashMap<u8, Option<SystemFont>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    *cache
        .entry(script as u8)
        .or_insert_with(|| find(database(), script))
}

fn find(db: &fontdb::Database, script: Script) -> Option<SystemFont> {
    for family in script.families() {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight: fontdb::Weight::NORMAL,
            ..Default::default()
        };
        let Some(id) = db.query(&query) else { continue };
        let Some(face) = db.face(id) else { continue };
        let fontdb::Source::File(path) = &face.source else {
            continue;
        };
        let Ok(data) = std::fs::read(path) else {
            continue;
        };
        let ok = ttf_parser::Face::parse(&data, face.index)
            .is_ok_and(|f| f.glyph_index(script.probe()).is_some());
        if ok {
            log::info!("CJK fallback {script:?}: {family} ({})", path.display());
            // Lives for the process; at most one leak per script.
            let bytes: &'static [u8] = Box::leak(data.into_boxed_slice());
            return Some(SystemFont {
                bytes,
                index: face.index,
                script,
            });
        }
    }
    log::warn!("no system font found for {script:?}");
    None
}

/// egui places a fallback glyph at `ascent + ½·(primary_height − face_height)`
/// (heights include the line gap), so faces with different vertical metrics
/// drift off the primary baseline. Returns the downward shift, as a fraction of
/// the font size, that puts `fallback` on `primary`'s baseline.
pub fn baseline_offset_factor(primary: &[u8], fallback: &[u8], fallback_index: u32) -> f32 {
    let m = |data: &[u8], index: u32| {
        let f = ttf_parser::Face::parse(data, index).ok()?;
        let upm = f32::from(f.units_per_em());
        let (asc, desc, gap) = (
            f32::from(f.ascender()) / upm,
            f32::from(f.descender()) / upm,
            f32::from(f.line_gap()) / upm,
        );
        Some((asc, asc - desc + gap))
    };
    let (Some((pa, ph)), Some((fa, fh))) = (m(primary, 0), m(fallback, fallback_index)) else {
        return 0.0;
    };
    pa - (fa + 0.5 * (ph - fh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_fonts_need_no_offset() {
        let manrope = include_bytes!("../../../assets/fonts/ui/Manrope-Regular.ttf");
        assert_eq!(baseline_offset_factor(manrope, manrope, 0), 0.0);
        let noto = include_bytes!("../../../assets/fonts/ui/NotoSansCJK-LanguageNames.otf");
        assert!(baseline_offset_factor(manrope, noto, 0).abs() < 0.2);
    }

    #[test]
    fn bundled_subset_covers_native_language_names() {
        let manrope = ttf_parser::Face::parse(
            include_bytes!("../../../assets/fonts/ui/Manrope-Regular.ttf"),
            0,
        )
        .unwrap();
        let noto = ttf_parser::Face::parse(
            include_bytes!("../../../assets/fonts/ui/NotoSansCJK-LanguageNames.otf"),
            0,
        )
        .unwrap();
        for lang in Language::ALL {
            for c in lang.native_label().chars() {
                assert!(
                    manrope.glyph_index(c).is_some() || noto.glyph_index(c).is_some(),
                    "{c:?} in {}",
                    lang.native_label()
                );
            }
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn finds_all_three_scripts_on_macos() {
        for s in Script::ALL {
            let f = cjk_font(s).unwrap_or_else(|| panic!("{s:?}"));
            assert_eq!(f.script, s);
        }
    }
}
