//! UI strings for everything the app draws itself: tray, offline page and
//! the bundled windows. See ADR 0015.
//!
//! Each language is one JSON file in `src-tauri/i18n/`, named after its lila
//! language tag and mapping keys to strings. `en-GB.json` is the base: it has
//! every key, and any key a language lacks falls back to it. Keys are
//! `<lila module>.<lila key>` for strings shared with lichess and
//! `desktop.<key>` for our own.

use std::collections::HashMap;

/// `(tag, json)` for every file in `src-tauri/i18n/`, embedded by `build.rs`.
const TRANSLATIONS: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/translations.rs"));

/// The base language. Its file has every key.
pub const BASE: &str = "en-GB";

/// Declares [`Key`] and its string ids in one place, so the list the tests
/// check can't drift from the enum.
macro_rules! keys {
    ($($variant:ident => $id:literal,)*) => {
        /// Every string the app uses. A key that is missing from the base file
        /// fails the tests, so typos can't reach the UI.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Key {
            $($variant,)*
        }

        impl Key {
            #[cfg(test)]
            pub const ALL: &[Key] = &[$(Key::$variant,)*];

            pub const fn id(self) -> &'static str {
                match self {
                    $(Key::$variant => $id,)*
                }
            }
        }
    };
}

keys! {
    ShowHide => "desktop.showHide",
    Quit => "desktop.quit",
    CantReachLichess => "desktop.cantReachLichess",
    CheckConnection => "desktop.checkConnection",
    Retry => "desktop.retry",
}

/// Strings of one language, with the base language behind it.
#[derive(Debug)]
pub struct Strings {
    lang: &'static str,
    strings: HashMap<String, String>,
    base: HashMap<String, String>,
}

impl Strings {
    /// Loads `lang`. A language without a file shows the base strings.
    pub fn load(lang: &'static str) -> Self {
        Self {
            lang,
            strings: if lang == BASE {
                HashMap::new()
            } else {
                parse(lang)
            },
            base: parse(BASE),
        }
    }

    /// The lila tag of the language, for `<html lang>`.
    pub fn lang(&self) -> &'static str {
        self.lang
    }

    /// Text direction of the language, for `<html dir>`.
    pub fn dir(&self) -> &'static str {
        const RTL: &[&str] = &["ar", "ckb", "fa", "he", "ps", "ur"];
        let language = self.lang.split('-').next().unwrap_or_default();
        if RTL.contains(&language) {
            "rtl"
        } else {
            "ltr"
        }
    }

    /// The string for `key`, from this language or else the base language.
    pub fn get(&self, key: Key) -> &str {
        let id = key.id();
        self.strings
            .get(id)
            .or_else(|| self.base.get(id))
            .map_or(id, String::as_str)
    }
}

fn parse(lang: &str) -> HashMap<String, String> {
    let Some((_, json)) = TRANSLATIONS.iter().find(|(tag, _)| *tag == lang) else {
        return HashMap::new();
    };
    serde_json::from_str(json).unwrap_or_else(|err| panic!("i18n/{lang}.json: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_has_every_key_and_no_other() {
        let base = parse(BASE);
        for key in Key::ALL {
            assert!(
                base.contains_key(key.id()),
                "{BASE}.json lacks {}",
                key.id()
            );
        }
        let known: Vec<_> = Key::ALL.iter().map(|key| key.id()).collect();
        for id in base.keys() {
            assert!(
                known.contains(&id.as_str()),
                "{BASE}.json has unused key {id}"
            );
        }
    }

    #[test]
    fn every_file_parses_and_uses_known_keys() {
        let known: Vec<_> = Key::ALL.iter().map(|key| key.id()).collect();
        for (tag, _) in TRANSLATIONS {
            for id in parse(tag).keys() {
                assert!(
                    known.contains(&id.as_str()),
                    "{tag}.json has unknown key {id}"
                );
            }
        }
    }

    #[test]
    fn missing_language_or_key_falls_back_to_base() {
        let strings = Strings::load("xx-XX");
        assert_eq!(strings.get(Key::Quit), "Quit");
        assert_eq!(strings.lang(), "xx-XX");
    }

    #[test]
    fn direction_follows_the_language() {
        assert_eq!(Strings::load("ar-SA").dir(), "rtl");
        assert_eq!(Strings::load("he-IL").dir(), "rtl");
        assert_eq!(Strings::load(BASE).dir(), "ltr");
        assert_eq!(Strings::load("de-DE").dir(), "ltr");
    }
}
