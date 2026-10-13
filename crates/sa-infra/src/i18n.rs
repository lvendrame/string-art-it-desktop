//! Translations (spec 24): the web app's i18next JSON resources, embedded at
//! build time. Keys are `"ns:dotted.path"`; `{{var}}` placeholders are
//! interpolated. Missing keys fall back to English, then to the key itself.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    En,
    PtBr,
    Es,
    Fr,
    De,
    It,
    Nl,
    Pl,
    Ja,
    Ru,
    Uk,
    Tr,
    ZhCn,
    Ko,
}

impl Language {
    pub const ALL: [Language; 14] = [
        Language::En,
        Language::PtBr,
        Language::Es,
        Language::Fr,
        Language::De,
        Language::It,
        Language::Nl,
        Language::Pl,
        Language::Ja,
        Language::Ru,
        Language::Uk,
        Language::Tr,
        Language::ZhCn,
        Language::Ko,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::PtBr => "pt-BR",
            Language::Es => "es",
            Language::Fr => "fr",
            Language::De => "de",
            Language::It => "it",
            Language::Nl => "nl",
            Language::Pl => "pl",
            Language::Ja => "ja",
            Language::Ru => "ru",
            Language::Uk => "uk",
            Language::Tr => "tr",
            Language::ZhCn => "zh-CN",
            Language::Ko => "ko",
        }
    }

    pub fn native_label(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::PtBr => "Português (BR)",
            Language::Es => "Español",
            Language::Fr => "Français",
            Language::De => "Deutsch",
            Language::It => "Italiano",
            Language::Nl => "Nederlands",
            Language::Pl => "Polski",
            Language::Ja => "日本語",
            Language::Ru => "Русский",
            Language::Uk => "Українська",
            Language::Tr => "Türkçe",
            Language::ZhCn => "简体中文",
            Language::Ko => "한국어",
        }
    }

    pub fn from_code(code: &str) -> Option<Language> {
        Language::ALL.into_iter().find(|l| l.code() == code)
    }

    /// Needs a CJK-capable UI font.
    pub fn is_cjk(self) -> bool {
        matches!(self, Language::Ja | Language::ZhCn | Language::Ko)
    }

    /// Web `detectInitialLanguage`: a stored choice wins, else the OS locale prefix.
    pub fn detect(system_locale: Option<&str>, stored: Option<&str>) -> Language {
        if let Some(l) = stored.and_then(Language::from_code) {
            return l;
        }
        let lower = system_locale.unwrap_or("en").to_ascii_lowercase();
        let table = [
            ("pt", Language::PtBr),
            ("es", Language::Es),
            ("fr", Language::Fr),
            ("de", Language::De),
            ("it", Language::It),
            ("nl", Language::Nl),
            ("pl", Language::Pl),
            ("ja", Language::Ja),
            ("ru", Language::Ru),
            ("uk", Language::Uk),
            ("tr", Language::Tr),
            ("zh", Language::ZhCn),
            ("ko", Language::Ko),
        ];
        table
            .into_iter()
            .find(|(p, _)| lower.starts_with(p))
            .map(|(_, l)| l)
            .unwrap_or(Language::En)
    }
}

type Table = HashMap<String, String>;

fn flatten(prefix: &str, v: &Value, out: &mut Table) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                let key = if prefix.ends_with(':') {
                    format!("{prefix}{k}")
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, child, out);
            }
        }
        Value::String(s) => {
            out.insert(prefix.to_owned(), s.clone());
        }
        other => {
            out.insert(prefix.to_owned(), other.to_string());
        }
    }
}

fn tables() -> &'static HashMap<&'static str, Table> {
    static TABLES: OnceLock<HashMap<&'static str, Table>> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut all: HashMap<&'static str, Table> = HashMap::new();
        for (lang, ns, json) in LOCALE_FILES {
            let value: Value = serde_json::from_str(json).unwrap_or(Value::Null);
            flatten(&format!("{ns}:"), &value, all.entry(lang).or_default());
        }
        all
    })
}

/// Every flattened key for `lang` (tests / parity checks).
pub fn keys(lang: Language) -> Vec<String> {
    let mut k: Vec<String> = tables()
        .get(lang.code())
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default();
    k.sort();
    k
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I18n {
    pub lang: Language,
}

impl I18n {
    pub fn new(lang: Language) -> Self {
        Self { lang }
    }

    /// Raw lookup with English fallback; returns the key when missing.
    pub fn t(&self, key: &str) -> String {
        let t = tables();
        t.get(self.lang.code())
            .and_then(|m| m.get(key))
            .or_else(|| t.get("en").and_then(|m| m.get(key)))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    /// Lookup with `{{name}}` interpolation.
    pub fn tf(&self, key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
        let mut s = self.t(key);
        for (name, value) in args {
            s = s.replace(&format!("{{{{{name}}}}}"), &value.to_string());
        }
        s
    }

    /// Like [`I18n::t`] but `None` when the key is absent in every language.
    pub fn try_t(&self, key: &str) -> Option<String> {
        let t = tables();
        t.get(self.lang.code())
            .and_then(|m| m.get(key))
            .or_else(|| t.get("en").and_then(|m| m.get(key)))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_lookup_and_interpolation() {
        let i = I18n::new(Language::En);
        assert_eq!(i.t("common:modes.select"), "Edit");
        assert_eq!(i.t("missing:key"), "missing:key");
        let s = i.tf("boardSetup:hypotenuse", &[("value", &"50.0")]);
        assert!(s.contains("50.0"), "{s}");
    }

    #[test]
    fn all_locales_have_identical_keys() {
        let en = keys(Language::En);
        assert!(en.len() > 400, "{}", en.len());
        for lang in Language::ALL {
            assert_eq!(keys(lang), en, "key parity for {}", lang.code());
        }
    }

    #[test]
    fn detect() {
        assert_eq!(Language::detect(Some("pt-PT"), None), Language::PtBr);
        assert_eq!(Language::detect(Some("zh-Hant"), None), Language::ZhCn);
        assert_eq!(Language::detect(Some("sv-SE"), None), Language::En);
        assert_eq!(Language::detect(Some("fr"), Some("ja")), Language::Ja);
    }

    #[test]
    fn translated() {
        assert_ne!(
            I18n::new(Language::De).t("common:modes.pin"),
            "common:modes.pin"
        );
    }
}
