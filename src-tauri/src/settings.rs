//! App settings, persisted with `tauri-plugin-store` through its Rust API only.
//!
//! No capability grants the store plugin's commands, so the lichess page
//! cannot read or change settings (ADR 0005).
//!
//! Each setting is a top-level key in `settings.json`. Loading is
//! migration-safe: a missing key, or a value that does not parse as the
//! expected type, falls back to its default, and unknown keys are ignored.

use std::collections::HashMap;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tauri::{Manager, Runtime};
use tauri_plugin_store::{Store, StoreExt};

/// File name of the store, resolved by the plugin inside the app data dir.
const STORE_FILE: &str = "settings.json";

const ZOOM: &str = "zoom";
const CLOSE_TO_TRAY: &str = "close_to_tray";
const CHECK_UPDATES: &str = "check_updates";
const NOTIFY_YOUR_TURN: &str = "notify_your_turn";
const NOTIFY_GAME_OVER: &str = "notify_game_over";
const ATTENTION: &str = "attention";
const NOTIFY_ACCOUNT_EVENTS: &str = "notify_account_events";
const LANGUAGE: &str = "language";

/// UI language: follow the system, or a fixed language tag such as `de`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    System,
    Tag(String),
}

impl Language {
    const SYSTEM: &str = "system";

    fn from_value(value: &Value) -> Option<Self> {
        match value.as_str()? {
            "" => None,
            Self::SYSTEM => Some(Self::System),
            tag => Some(Self::Tag(tag.to_owned())),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Self::System => Self::SYSTEM.into(),
            Self::Tag(tag) => tag.as_str().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Page zoom factor, `1.0` is 100 %.
    pub zoom: f64,
    pub close_to_tray: bool,
    pub check_updates: bool,
    pub notify_your_turn: bool,
    pub notify_game_over: bool,
    /// Request taskbar/dock attention when a game needs the user.
    pub attention: bool,
    pub notify_account_events: bool,
    pub language: Language,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            close_to_tray: false,
            check_updates: true,
            notify_your_turn: true,
            notify_game_over: true,
            attention: true,
            notify_account_events: true,
            language: Language::System,
        }
    }
}

impl Settings {
    /// Builds settings from stored key/value pairs. Never fails: anything
    /// missing or invalid takes its default.
    fn from_entries(entries: &HashMap<String, Value>) -> Self {
        let defaults = Self::default();
        Self {
            zoom: field(entries, ZOOM)
                .filter(|zoom: &f64| zoom.is_finite() && *zoom > 0.0)
                .unwrap_or(defaults.zoom),
            close_to_tray: field(entries, CLOSE_TO_TRAY).unwrap_or(defaults.close_to_tray),
            check_updates: field(entries, CHECK_UPDATES).unwrap_or(defaults.check_updates),
            notify_your_turn: field(entries, NOTIFY_YOUR_TURN).unwrap_or(defaults.notify_your_turn),
            notify_game_over: field(entries, NOTIFY_GAME_OVER).unwrap_or(defaults.notify_game_over),
            attention: field(entries, ATTENTION).unwrap_or(defaults.attention),
            notify_account_events: field(entries, NOTIFY_ACCOUNT_EVENTS)
                .unwrap_or(defaults.notify_account_events),
            language: entries
                .get(LANGUAGE)
                .and_then(Language::from_value)
                .unwrap_or(defaults.language),
        }
    }

    fn to_entries(&self) -> [(&'static str, Value); 8] {
        [
            (ZOOM, self.zoom.into()),
            (CLOSE_TO_TRAY, self.close_to_tray.into()),
            (CHECK_UPDATES, self.check_updates.into()),
            (NOTIFY_YOUR_TURN, self.notify_your_turn.into()),
            (NOTIFY_GAME_OVER, self.notify_game_over.into()),
            (ATTENTION, self.attention.into()),
            (NOTIFY_ACCOUNT_EVENTS, self.notify_account_events.into()),
            (LANGUAGE, self.language.to_value()),
        ]
    }
}

fn field<T: DeserializeOwned>(entries: &HashMap<String, Value>, key: &str) -> Option<T> {
    serde_json::from_value(entries.get(key)?.clone()).ok()
}

/// Typed access to the settings store. Register it with [`init`].
#[expect(
    dead_code,
    reason = "read by upcoming features (zoom, tray, notifications)"
)]
pub struct SettingsStore<R: Runtime> {
    store: Arc<Store<R>>,
}

#[expect(
    dead_code,
    reason = "read by upcoming features (zoom, tray, notifications)"
)]
impl<R: Runtime> SettingsStore<R> {
    /// Current settings, with defaults for anything missing or invalid.
    pub fn get(&self) -> Settings {
        Settings::from_entries(&self.store.entries().into_iter().collect())
    }

    /// Replaces all settings and writes them to disk.
    pub fn set(&self, settings: &Settings) -> tauri_plugin_store::Result<()> {
        for (key, value) in settings.to_entries() {
            self.store.set(key, value);
        }
        self.store.save()
    }

    /// Changes some settings and writes them to disk.
    pub fn update(&self, change: impl FnOnce(&mut Settings)) -> tauri_plugin_store::Result<()> {
        let mut settings = self.get();
        change(&mut settings);
        self.set(&settings)
    }
}

/// Opens the settings store and puts it in managed state as
/// `SettingsStore<R>`. Requires the store plugin to be registered.
pub fn init<R: Runtime, M: Manager<R>>(manager: &M) -> tauri_plugin_store::Result<()> {
    let store = manager.store(STORE_FILE)?;
    manager.manage(SettingsStore { store });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entries(value: Value) -> HashMap<String, Value> {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn defaults_match_the_spec() {
        let settings = Settings::default();
        assert_eq!(settings.zoom, 1.0);
        assert!(!settings.close_to_tray);
        assert!(settings.check_updates);
        assert!(settings.notify_your_turn);
        assert!(settings.notify_game_over);
        assert!(settings.attention);
        assert!(settings.notify_account_events);
        assert_eq!(settings.language, Language::System);
    }

    #[test]
    fn empty_store_gives_defaults() {
        assert_eq!(Settings::from_entries(&HashMap::new()), Settings::default());
    }

    #[test]
    fn round_trips_through_entries() {
        let settings = Settings {
            zoom: 1.25,
            close_to_tray: true,
            check_updates: false,
            notify_your_turn: false,
            notify_game_over: false,
            attention: false,
            notify_account_events: false,
            language: Language::Tag("de".into()),
        };
        let stored = settings
            .to_entries()
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        assert_eq!(Settings::from_entries(&stored), settings);
    }

    #[test]
    fn round_trips_through_json_file_format() {
        let settings = Settings {
            language: Language::Tag("pt-BR".into()),
            ..Settings::default()
        };
        let map: HashMap<_, _> = settings
            .to_entries()
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        let bytes = serde_json::to_vec(&map).unwrap();
        let loaded = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(Settings::from_entries(&loaded), settings);
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let settings = Settings::from_entries(&entries(json!({ "close_to_tray": true })));
        assert_eq!(
            settings,
            Settings {
                close_to_tray: true,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let settings = Settings::from_entries(&entries(json!({
            "removed_setting": 42,
            "check_updates": false,
        })));
        assert_eq!(
            settings,
            Settings {
                check_updates: false,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn invalid_values_fall_back_to_defaults() {
        let settings = Settings::from_entries(&entries(json!({
            "zoom": "big",
            "close_to_tray": "yes",
            "check_updates": null,
            "language": 7,
        })));
        assert_eq!(settings, Settings::default());

        for zoom in [json!(0.0), json!(-1.0)] {
            let settings = Settings::from_entries(&entries(json!({ "zoom": zoom })));
            assert_eq!(settings.zoom, 1.0);
        }
        let settings = Settings::from_entries(&entries(json!({ "language": "" })));
        assert_eq!(settings.language, Language::System);
    }
}
