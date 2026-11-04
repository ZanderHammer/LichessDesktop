//! UI strings for everything the app draws itself: tray, offline page and
//! the bundled windows. See ADR 0015.
//!
//! Each language is one JSON file in `src-tauri/i18n/`, named after its lila
//! language tag and mapping keys to strings. `en-GB.json` is the base, and a
//! test checks that every language has every key (ADR 0020). Keys are
//! `<lila module>.<lila key>` for strings shared with lichess and
//! `desktop.<key>` for our own.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::settings::Language;

/// `(tag, json)` for every file in `src-tauri/i18n/`, embedded by `build.rs`.
const TRANSLATIONS: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/translations.rs"));

/// The base language. Its file has every key.
pub const BASE: &str = "en-GB";

/// Languages the app offers, as `(lila tag, native name)`, in lichess's order.
/// Copied from lila `modules/i18n/src/main/LangList.scala` (commit 43e84ade40),
/// without the 11 languages ADR 0020 leaves out.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("af-ZA", "Afrikaans"),
    ("an-ES", "Aragonés"),
    ("ast-ES", "Asturianu"),
    ("az-AZ", "Azərbaycanca"),
    ("id-ID", "Bahasa Indonesia"),
    ("bs-BA", "Bosanski"),
    ("br-FR", "Brezhoneg"),
    ("ca-ES", "Català, valencià"),
    ("cs-CZ", "Čeština"),
    ("co-FR", "Corsu"),
    ("cy-GB", "Cymraeg"),
    ("da-DK", "Dansk"),
    ("de-DE", "Deutsch"),
    ("et-EE", "Eesti keel"),
    ("en-GB", "English"),
    ("en-US", "English (US)"),
    ("es-ES", "Español"),
    ("eo-AA", "Esperanto"),
    ("eu-ES", "Euskara"),
    ("fo-FO", "Føroyskt"),
    ("fr-FR", "Français"),
    ("fy-NL", "Frysk"),
    ("ga-IE", "Gaeilge"),
    ("gd-GB", "Gàidhlig"),
    ("gl-ES", "Galego"),
    ("hr-HR", "Hrvatski"),
    ("ia-AA", "Interlingua"),
    ("is-IS", "Íslenska"),
    ("it-IT", "Italiano"),
    ("sw-KE", "Kiswahili"),
    ("kmr-TR", "Kurdî (Kurmancî)"),
    ("lv-LV", "Latviešu valoda"),
    ("lb-LU", "Lëtzebuergesch"),
    ("lt-LT", "Lietuvių kalba"),
    ("la-VA", "Lingua Latina"),
    ("hu-HU", "Magyar"),
    ("nl-NL", "Nederlands"),
    ("nb-NO", "Norsk bokmål"),
    ("nn-NO", "Norsk nynorsk"),
    ("uz-UZ", "Özbekça"),
    ("pl-PL", "Polski"),
    ("pt-PT", "Português"),
    ("pt-BR", "Português (BR)"),
    ("ro-RO", "Română"),
    ("gsw-CH", "Schwizerdütsch"),
    ("sq-AL", "Shqip"),
    ("sk-SK", "Slovenčina"),
    ("sl-SI", "Slovenščina"),
    ("fi-FI", "Suomen kieli"),
    ("sv-SE", "Svenska"),
    ("tl-PH", "Tagalog"),
    ("vi-VN", "Tiếng Việt"),
    ("tr-TR", "Türkçe"),
    ("tk-TM", "Türkmençe"),
    ("el-GR", "Ελληνικά"),
    ("be-BY", "Беларуская"),
    ("bg-BG", "Български език"),
    ("kk-KZ", "Қазақша"),
    ("mk-MK", "Македонски јази"),
    ("mn-MN", "Монгол"),
    ("ru-RU", "Русский язык"),
    ("sr-SP", "Српски језик"),
    ("uk-UA", "Українська"),
    ("ka-GE", "ქართული"),
    ("hy-AM", "Հայերեն"),
    ("he-IL", "עִבְרִית"),
    ("ur-PK", "اُردُو"),
    ("ar-SA", "العربية"),
    ("fa-IR", "فارسی"),
    ("ckb-IR", "کوردی سۆرانی"),
    ("ne-NP", "नेपाली"),
    ("mr-IN", "मराठी"),
    ("hi-IN", "हिन्दी, हिंदी"),
    ("bn-BD", "বাংলা"),
    ("gu-IN", "ગુજરાતી"),
    ("ta-IN", "தமிழ்"),
    ("kn-IN", "ಕನ್ನಡ"),
    ("ml-IN", "മലയാളം"),
    ("th-TH", "ไทย"),
    ("ja-JP", "日本語"),
    ("ko-KR", "한국어"),
    ("zh-CN", "简体中文"),
    ("zh-TW", "繁體中文"),
];

/// lila's `defaultRegions`: the variant a bare language code stands for.
const DEFAULT_REGIONS: &[(&str, &str)] = &[
    ("de", "de-DE"),
    ("en", "en-US"),
    ("pt", "pt-BR"),
    ("zh", "zh-CN"),
];

/// The UI language: the one chosen in Settings if lichess offers it, else the
/// first system locale that matches a lichess language, else [`BASE`].
pub fn resolve(setting: &Language, system: impl IntoIterator<Item = String>) -> &'static str {
    if let Language::Tag(chosen) = setting
        && let Some(&(tag, _)) = LANGUAGES.iter().find(|(tag, _)| tag == chosen)
    {
        return tag;
    }
    system
        .into_iter()
        .find_map(|locale| match_locale(&locale))
        .unwrap_or(BASE)
}

/// Matches a BCP 47 or POSIX locale (`de-AT`, `de_AT.UTF-8`) to a lichess
/// language: the same language and region if lichess has it, else lila's
/// default region for the language, else the first variant of it.
pub fn match_locale(locale: &str) -> Option<&'static str> {
    let locale = locale.split(['.', '@']).next().unwrap_or_default();
    let mut subtags = locale.split(['-', '_']).filter(|s| !s.is_empty());
    let mut language = subtags.next()?.to_ascii_lowercase();
    let (mut script, mut region) = (None, None);
    for subtag in subtags {
        match subtag.len() {
            4 if script.is_none() && region.is_none() => script = Some(subtag.to_ascii_lowercase()),
            2 | 3 if region.is_none() => region = Some(subtag.to_ascii_uppercase()),
            _ => {}
        }
    }

    match language.as_str() {
        // lichess has Traditional Chinese as zh-TW and Simplified as zh-CN.
        "zh" => {
            let traditional = script.as_deref() == Some("hant")
                || matches!(region.as_deref(), Some("TW" | "HK" | "MO"));
            return Some(if traditional { "zh-TW" } else { "zh-CN" });
        }
        // Norwegian without a written standard means Bokmål.
        "no" => language = "nb".to_owned(),
        _ => {}
    }

    if let Some(region) = &region {
        let wanted = format!("{language}-{region}");
        if let Some(&(tag, _)) = LANGUAGES.iter().find(|(tag, _)| *tag == wanted) {
            return Some(tag);
        }
    }
    if let Some(&(_, tag)) = DEFAULT_REGIONS.iter().find(|(lang, _)| *lang == language) {
        return Some(tag);
    }
    LANGUAGES
        .iter()
        .map(|&(tag, _)| tag)
        .find(|tag| tag.split('-').next() == Some(language.as_str()))
}

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
    SettingsMenu => "desktop.settingsMenu",
    Quit => "desktop.quit",
    CantReachLichess => "desktop.cantReachLichess",
    CheckConnection => "desktop.checkConnection",
    Retry => "site.retry",
    Settings => "settings.settings",
    About => "desktop.about",
    General => "preferences.generalSettings",
    CloseToTray => "desktop.closeToTray",
    CloseToTrayHint => "desktop.closeToTrayHint",
    CheckForUpdates => "desktop.checkForUpdates",
    Language => "site.language",
    SystemLanguage => "desktop.systemLanguage",
    Notifications => "site.notifications",
    NotifyYourTurn => "site.yourTurn",
    NotifyGameOver => "site.gameOver",
    Attention => "desktop.attention",
    AttentionHint => "desktop.attentionHint",
    Unofficial => "desktop.unofficial",
    Version => "desktop.version",
    SourceCode => "site.sourceCode",
    License => "desktop.license",
    Navigation => "desktop.navigation",
    Back => "study.back",
    Forward => "desktop.forward",
    Reload => "desktop.reload",
    Home => "desktop.home",
    CopyUrl => "desktop.copyUrl",
    OpenInBrowser => "desktop.openInBrowser",
    ZoomIn => "desktop.zoomIn",
    ZoomOut => "desktop.zoomOut",
    ZoomReset => "desktop.zoomReset",
    Fullscreen => "desktop.fullscreen",
    AlwaysOnTop => "desktop.alwaysOnTop",
    Shortcuts => "desktop.shortcuts",
    MouseBack => "desktop.mouseBack",
    MouseForward => "desktop.mouseForward",
    KeyboardShortcuts => "site.keyboardShortcuts",
    UpdateAvailable => "desktop.updateAvailable",
    UpdateChecksOff => "desktop.updateChecksOff",
    CheckingForUpdates => "desktop.checkingForUpdates",
    UpToDate => "desktop.upToDate",
    UpdateCheckFailed => "desktop.updateCheckFailed",
    CommandPalette => "desktop.commandPalette",
    LichessTv => "desktop.lichessTv",
    Search => "site.search",
    NoResultsFor => "site.thereAreNoResultsForX",
    CreateLobbyGame => "site.createLobbyGame",
    ChallengeAFriend => "site.challengeAFriend",
    PlayAgainstComputer => "site.playAgainstComputer",
    Tournaments => "site.tournaments",
    Puzzles => "site.puzzles",
    ChessBasics => "site.chessBasics",
    Study => "site.studyMenu",
    Broadcasts => "broadcast.broadcasts",
    CurrentGames => "site.currentGames",
    AnalysisBoard => "site.analysis",
    BoardEditor => "site.boardEditor",
    Inbox => "site.inbox",
    EditProfile => "site.editProfile",
    Preferences => "preferences.preferences",
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
        const RTL: &[&str] = &["ar", "ckb", "fa", "he", "ur"];
        let language = self.lang.split('-').next().unwrap_or_default();
        if RTL.contains(&language) {
            "rtl"
        } else {
            "ltr"
        }
    }

    /// Every string by key id, for a bundled page.
    pub fn all(&self) -> HashMap<&'static str, &str> {
        Key::ALL
            .iter()
            .map(|&key| (key.id(), self.get(key)))
            .collect()
    }

    /// The string for `key`, from this language or else the base language.
    pub fn get(&self, key: Key) -> &str {
        let id = key.id();
        self.strings
            .get(id)
            .or_else(|| self.base.get(id))
            .map_or(id, String::as_str)
    }

    /// The string for `key` with its lila placeholder (`%s` or `%1$s`)
    /// filled in, like `t()` in the bundled pages.
    pub fn format(&self, key: Key, arg: &str) -> String {
        self.get(key)
            .replacen("%1$s", arg, 1)
            .replacen("%s", arg, 1)
    }
}

/// The strings of the current UI language, in managed state. Replaced when
/// the language setting changes.
pub struct Current(RwLock<Arc<Strings>>);

impl Current {
    pub fn new(strings: Strings) -> Self {
        Self(RwLock::new(Arc::new(strings)))
    }

    pub fn get(&self) -> Arc<Strings> {
        Arc::clone(
            &self
                .0
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }

    pub fn set(&self, strings: Strings) {
        *self
            .0
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Arc::new(strings);
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
    fn every_language_has_every_key() {
        for &(tag, _) in LANGUAGES {
            assert!(
                TRANSLATIONS.iter().any(|(file, _)| *file == tag),
                "i18n/{tag}.json is missing: see ADR 0020"
            );
            let strings = parse(tag);
            for key in Key::ALL {
                let text = strings.get(key.id()).map_or("", String::as_str);
                assert!(
                    !text.trim().is_empty(),
                    "{tag}.json lacks {}: see ADR 0020",
                    key.id()
                );
            }
        }
    }

    #[test]
    fn every_language_keeps_the_placeholders() {
        let count = |text: &str| text.matches("%s").count() + text.matches("%1$s").count();
        let base = parse(BASE);
        for (tag, _) in TRANSLATIONS {
            for (id, text) in parse(tag) {
                assert_eq!(
                    count(&text),
                    count(&base[&id]),
                    "{tag}.json has the wrong placeholders in {id}: {text}"
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
    fn format_fills_in_the_placeholder() {
        let strings = Strings::load(BASE);
        assert_eq!(strings.format(Key::Version, "1.2.3"), "Version 1.2.3");
    }

    #[test]
    fn direction_follows_the_language() {
        assert_eq!(Strings::load("ar-SA").dir(), "rtl");
        assert_eq!(Strings::load("he-IL").dir(), "rtl");
        assert_eq!(Strings::load(BASE).dir(), "ltr");
        assert_eq!(Strings::load("de-DE").dir(), "ltr");
    }

    #[test]
    fn matches_region_then_default_region_then_any_variant() {
        assert_eq!(match_locale("de-DE"), Some("de-DE"));
        assert_eq!(match_locale("de_AT"), Some("de-DE"));
        assert_eq!(match_locale("de_AT.UTF-8@euro"), Some("de-DE"));
        assert_eq!(match_locale("pt-PT"), Some("pt-PT"));
        assert_eq!(match_locale("pt"), Some("pt-BR"));
        assert_eq!(match_locale("en-GB"), Some("en-GB"));
        assert_eq!(match_locale("en-AU"), Some("en-US"));
        assert_eq!(match_locale("fr-CA"), Some("fr-FR"));
        assert_eq!(match_locale("SV"), Some("sv-SE"));
    }

    #[test]
    fn matches_chinese_by_script_and_norwegian_to_bokmal() {
        assert_eq!(match_locale("zh-Hant-HK"), Some("zh-TW"));
        assert_eq!(match_locale("zh-TW"), Some("zh-TW"));
        assert_eq!(match_locale("zh-Hans-SG"), Some("zh-CN"));
        assert_eq!(match_locale("zh"), Some("zh-CN"));
        assert_eq!(match_locale("no-NO"), Some("nb-NO"));
        assert_eq!(match_locale("nn-NO"), Some("nn-NO"));
    }

    #[test]
    fn unknown_locales_do_not_match() {
        for locale in ["", "C", "POSIX", "xx-XX", "-", "zu-ZA"] {
            assert_eq!(match_locale(locale), None, "{locale:?}");
        }
    }

    #[test]
    fn resolves_setting_then_system_then_english() {
        let system = |locales: &[&str]| locales.iter().map(|l| l.to_string()).collect::<Vec<_>>();
        assert_eq!(
            resolve(&Language::Tag("fr-FR".into()), system(&["de-DE"])),
            "fr-FR"
        );
        // A tag lichess doesn't offer falls back to the system.
        assert_eq!(
            resolve(&Language::Tag("fr".into()), system(&["de-DE"])),
            "de-DE"
        );
        assert_eq!(
            resolve(&Language::System, system(&["xx", "it-CH", "de"])),
            "it-IT"
        );
        assert_eq!(resolve(&Language::System, system(&["C"])), BASE);
        assert_eq!(resolve(&Language::System, system(&[])), BASE);
    }

    #[test]
    fn language_list_matches_lichess() {
        assert_eq!(LANGUAGES.len(), 83);
        assert!(LANGUAGES.iter().any(|&(tag, _)| tag == BASE));
        for (tag, _) in TRANSLATIONS {
            assert!(
                LANGUAGES.iter().any(|(lang, _)| lang == tag),
                "i18n/{tag}.json is not a lichess language"
            );
        }
    }
}
