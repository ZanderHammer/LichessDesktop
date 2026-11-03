//! The settings/about window (#15): bundled HTML in `ui/settings.html` with
//! scoped IPC (ADR 0012). Its capability, `capabilities/settings.json`,
//! grants exactly the commands below; every command is also listed in
//! `build.rs`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tauri::ipc::Invoke;
use tauri::webview::{NewWindowResponse, PageLoadEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

use crate::actions::Action;
use crate::i18n::{self, Key};
use crate::settings::{Language, SettingsStore};
use crate::shortcuts;

pub const LABEL: &str = "settings";

/// Shows the window anyway if the page never reports `settings_ready`.
const SHOW_FALLBACK: Duration = Duration::from_millis(1500);

/// Tells the page to fetch its strings again after a language change.
const STRINGS_CHANGED: &str = "strings-changed";

/// Opens the window, or brings it to the front if it is already open. The
/// main window is its parent, so it is shown first. A new window stays
/// hidden until the page has drawn its content (`settings_ready`), so it
/// never shows up empty.
pub fn open<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    crate::focus_main_window(app);
    let Some(main) = app.get_webview_window(crate::MAIN_WINDOW) else {
        return Ok(());
    };
    // `cargo tauri dev` serves `ui/` from its own server instead.
    let dev_url = app
        .config()
        .build
        .dev_url
        .clone()
        .filter(|_| tauri::is_dev());
    let nav_handle = app.clone();
    let popup_handle = app.clone();
    let title = app
        .state::<i18n::Current>()
        .get()
        .get(Key::Settings)
        .to_owned();
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("settings.html".into()))
        .title(title)
        .inner_size(640.0, 560.0)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .center()
        .visible(false)
        .parent(&main)?
        // Only the bundled page itself; its links open in the browser.
        .on_navigation(move |url| {
            if crate::navigation::is_bundled(url, dev_url.as_ref()) {
                return true;
            }
            crate::open_in_browser(&nav_handle, url);
            false
        })
        .on_new_window(move |url, _features| {
            crate::open_in_browser(&popup_handle, &url);
            NewWindowResponse::Deny
        })
        .on_page_load(|window, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let window = window.clone();
                thread::spawn(move || {
                    thread::sleep(SHOW_FALLBACK);
                    if !window.is_visible().unwrap_or(true) {
                        show(&window.as_ref().window());
                    }
                });
            }
        })
        .build()?;
    Ok(())
}

fn show<R: Runtime>(window: &tauri::Window<R>) {
    if let Err(err) = window.show().and_then(|()| window.set_focus()) {
        eprintln!("failed to show the settings window: {err}");
    }
}

/// The page has drawn its content: show the window.
#[tauri::command]
pub fn settings_ready<R: Runtime>(webview: tauri::Webview<R>) {
    show(&webview.window());
}

/// The handler for every app command. Keep it in step with `COMMANDS` in
/// `build.rs` and with `capabilities/settings.json`.
pub fn invoke_handler<R: Runtime>() -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_strings,
        get_settings,
        set_setting,
        get_app_info,
        get_shortcuts,
        settings_ready
    ]
}

/// The strings of the UI language, keyed by id.
#[derive(Serialize)]
pub struct StringsView {
    lang: &'static str,
    dir: &'static str,
    strings: HashMap<&'static str, String>,
}

#[tauri::command]
pub fn get_strings(strings: tauri::State<'_, i18n::Current>) -> StringsView {
    let strings = strings.get();
    StringsView {
        lang: strings.lang(),
        dir: strings.dir(),
        strings: strings
            .all()
            .into_iter()
            .map(|(id, text)| (id, text.to_owned()))
            .collect(),
    }
}

/// One action in the shortcuts overview.
#[derive(Debug, Serialize, PartialEq)]
pub struct ShortcutView {
    label: String,
    /// Each shortcut as its key caps, e.g. `["Ctrl", "R"]`.
    keys: Vec<Vec<String>>,
    /// A mouse button that also runs the action.
    mouse: Option<String>,
}

/// The shortcuts of this platform, one entry per action, straight from the
/// shortcut table, so the overview can't drift from what is bound.
#[tauri::command]
pub fn get_shortcuts(strings: tauri::State<'_, i18n::Current>) -> Vec<ShortcutView> {
    shortcuts_view(&strings.get(), cfg!(target_os = "macos"))
}

fn shortcuts_view(strings: &i18n::Strings, macos: bool) -> Vec<ShortcutView> {
    Action::ALL
        .iter()
        .map(|&action| ShortcutView {
            label: strings.get(action.label()).to_owned(),
            keys: keys(action, macos),
            mouse: shortcuts::mouse_button(action).map(|key| strings.get(key).to_owned()),
        })
        .collect()
}

/// The key caps of every shortcut of `action` on this platform.
fn keys(action: Action, macos: bool) -> Vec<Vec<String>> {
    #[cfg(any(target_os = "macos", test))]
    if macos && let Some(caps) = shortcuts::macos_built_in_key_caps(action) {
        return vec![caps];
    }
    shortcuts::for_platform(macos)
        .filter(|shortcut| shortcut.action == action)
        .map(|shortcut| shortcuts::key_caps(shortcut, macos))
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    close_to_tray: bool,
    check_updates: bool,
    notify_your_turn: bool,
    notify_game_over: bool,
    attention: bool,
    /// `system` or a language tag.
    language: String,
    /// The language "System" stands for right now.
    system_language: &'static str,
    /// Every language lichess offers, as `[tag, native name]`.
    languages: &'static [(&'static str, &'static str)],
}

#[tauri::command]
pub fn get_settings<R: Runtime>(app: AppHandle<R>) -> SettingsView {
    let settings = app.state::<SettingsStore<R>>().get();
    SettingsView {
        close_to_tray: settings.close_to_tray,
        check_updates: settings.check_updates,
        notify_your_turn: settings.notify_your_turn,
        notify_game_over: settings.notify_game_over,
        attention: settings.attention,
        language: settings.language.as_str().to_owned(),
        system_language: i18n::resolve(&Language::System, sys_locale::get_locales()),
        languages: i18n::LANGUAGES,
    }
}

/// One setting the page may change. Serde rejects unknown keys and wrong
/// value types before the command runs.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(
    tag = "key",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum SettingChange {
    CloseToTray(bool),
    CheckUpdates(bool),
    NotifyYourTurn(bool),
    NotifyGameOver(bool),
    Attention(bool),
    /// `system` or a tag from [`i18n::LANGUAGES`].
    Language(String),
}

/// Applies `change` and saves it. A language change takes effect at once.
#[tauri::command]
pub fn set_setting<R: Runtime>(app: AppHandle<R>, change: SettingChange) -> Result<(), String> {
    let language = match &change {
        SettingChange::Language(value) => Some(parse_language(value)?),
        _ => None,
    };
    app.state::<SettingsStore<R>>()
        .update(|settings| match change {
            SettingChange::CloseToTray(on) => settings.close_to_tray = on,
            SettingChange::CheckUpdates(on) => settings.check_updates = on,
            SettingChange::NotifyYourTurn(on) => settings.notify_your_turn = on,
            SettingChange::NotifyGameOver(on) => settings.notify_game_over = on,
            SettingChange::Attention(on) => settings.attention = on,
            SettingChange::Language(_) => {
                if let Some(language) = &language {
                    settings.language = language.clone();
                }
            }
        })
        .map_err(|err| err.to_string())?;
    if let Some(language) = language {
        apply_language(&app, &language);
    }
    Ok(())
}

fn parse_language(value: &str) -> Result<Language, String> {
    match Language::parse(value) {
        Some(Language::Tag(tag)) if !i18n::LANGUAGES.iter().any(|(lang, _)| *lang == tag) => {
            Err(format!("unknown language {tag:?}"))
        }
        Some(language) => Ok(language),
        None => Err("empty language".to_owned()),
    }
}

/// Switches the UI language: strings, tray menu, the macOS app menu, and this
/// window.
fn apply_language<R: Runtime>(app: &AppHandle<R>, language: &Language) {
    let strings = i18n::Strings::load(i18n::resolve(language, sys_locale::get_locales()));
    let title = strings.get(Key::Settings).to_owned();
    app.state::<i18n::Current>().set(strings);
    if let Err(err) = crate::tray::refresh_menu(app) {
        eprintln!("failed to rebuild the tray menu: {err}");
    }
    #[cfg(target_os = "macos")]
    if let Err(err) = crate::shortcuts::refresh_menu(app) {
        eprintln!("failed to rebuild the app menu: {err}");
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.set_title(&title);
        let _ = window.emit_to(LABEL, STRINGS_CHANGED, ());
    }
}

/// Facts for the About tab.
#[derive(Serialize)]
pub struct AppInfo {
    version: &'static str,
    repository: &'static str,
    license_url: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    let repository = env!("CARGO_PKG_REPOSITORY");
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        repository,
        license_url: format!("{repository}/blob/main/LICENSE"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, json};
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
    use tauri::webview::InvokeRequest;

    fn request(cmd: &str, url: &str) -> InvokeRequest {
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body: InvokeBody::default(),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.into(),
        }
    }

    /// Runs real IPC requests through the app's ACL (ADR 0012): only the
    /// settings window's bundled page may call app commands.
    #[test]
    fn only_the_settings_page_may_call_app_commands() {
        let app = mock_builder()
            .invoke_handler(invoke_handler::<MockRuntime>())
            .build(crate::context())
            .unwrap();
        app.manage(i18n::Current::new(i18n::Strings::load(i18n::BASE)));
        let settings = WebviewWindowBuilder::new(&app, LABEL, Default::default())
            .build()
            .unwrap();
        let main = WebviewWindowBuilder::new(&app, crate::MAIN_WINDOW, Default::default())
            .build()
            .unwrap();
        let local = if cfg!(windows) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        };

        for cmd in [
            "get_strings",
            "get_app_info",
            "get_shortcuts",
            "settings_ready",
        ] {
            assert!(
                get_ipc_response(&settings, request(cmd, local)).is_ok(),
                "{cmd} denied"
            );
            assert!(
                get_ipc_response(&main, request(cmd, local)).is_err(),
                "{cmd} allowed in main"
            );
            assert!(
                get_ipc_response(&settings, request(cmd, "https://lichess.org/")).is_err(),
                "{cmd} allowed for remote content"
            );
        }
        assert!(get_ipc_response(&main, request("plugin:event|listen", local)).is_err());
    }

    #[test]
    fn lists_every_action_with_its_shortcuts() {
        let strings = i18n::Strings::load(i18n::BASE);
        let view = shortcuts_view(&strings, false);
        assert_eq!(view.len(), Action::ALL.len());
        let reload = view.iter().find(|entry| entry.label == "Reload").unwrap();
        assert!(reload.keys.iter().all(|keys| !keys.is_empty()));
        assert!(!reload.keys.is_empty());
        assert!(reload.mouse.is_none());
    }

    #[test]
    fn parses_typed_changes_only() {
        let change: SettingChange =
            from_value(json!({ "key": "closeToTray", "value": true })).unwrap();
        assert_eq!(change, SettingChange::CloseToTray(true));
        let change: SettingChange =
            from_value(json!({ "key": "language", "value": "de-DE" })).unwrap();
        assert_eq!(change, SettingChange::Language("de-DE".into()));

        for bad in [
            json!({ "key": "closeToTray", "value": "yes" }),
            json!({ "key": "zoom", "value": 2.0 }),
            json!({ "key": "attention" }),
            json!({ "key": "attention", "value": true, "extra": 1 }),
        ] {
            assert!(
                from_value::<SettingChange>(bad.clone()).is_err(),
                "accepted {bad}"
            );
        }
    }

    #[test]
    fn accepts_system_and_lichess_languages_only() {
        assert_eq!(parse_language("system"), Ok(Language::System));
        assert_eq!(parse_language("pt-BR"), Ok(Language::Tag("pt-BR".into())));
        assert!(parse_language("pt").is_err());
        assert!(parse_language("../../etc").is_err());
        assert!(parse_language("").is_err());
    }
}
