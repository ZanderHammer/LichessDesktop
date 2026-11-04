//! The settings/about window (#15): bundled HTML in `ui/settings.html` with
//! scoped IPC (ADR 0012). Its capability, `capabilities/settings.json`,
//! grants exactly the commands below; every command is also listed in
//! `build.rs` and in `crate::invoke_handler`. The palette uses
//! `get_strings` too.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tauri::webview::{NewWindowResponse, PageLoadEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

use crate::account;
use crate::actions::Action;
use crate::i18n::{self, Key};
use crate::offline;
use crate::settings::{Language, SettingsStore};
use crate::shortcuts;
use crate::update::{self, Status};

pub const LABEL: &str = "settings";

/// Shows the window anyway if the page never reports `settings_ready`.
const SHOW_FALLBACK: Duration = Duration::from_millis(1500);

/// Tells the page to fetch its strings again after a language change.
const STRINGS_CHANGED: &str = "strings-changed";

/// Tells an open page to switch to the tab in the payload.
const SHOW_TAB: &str = "show-tab";

/// A tab the window can be opened on. The id is the tab's suffix in
/// `settings.html` (`tab-<id>`) and the URL fragment of a new window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Settings,
    Shortcuts,
    About,
}

impl Tab {
    const fn id(self) -> &'static str {
        match self {
            Tab::Settings => "settings",
            Tab::Shortcuts => "shortcuts",
            Tab::About => "about",
        }
    }
}

/// Opens the window, or brings it to the front if it is already open. The
/// main window is its parent, so it is shown first. A new window stays
/// hidden until the page has drawn its content (`settings_ready`), so it
/// never shows up empty.
pub fn open<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    open_at(app, None)
}

/// Like [`open`], on `tab`.
pub fn open_tab<R: Runtime>(app: &AppHandle<R>, tab: Tab) -> tauri::Result<()> {
    open_at(app, Some(tab))
}

fn open_at<R: Runtime>(app: &AppHandle<R>, tab: Option<Tab>) -> tauri::Result<()> {
    account::settings_opened(app);
    if let Some(window) = app.get_webview_window(LABEL) {
        if let Some(tab) = tab {
            window.emit_to(LABEL, SHOW_TAB, tab.id())?;
        }
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
    let page = match tab {
        Some(tab) => format!("settings.html#{}", tab.id()),
        None => "settings.html".to_owned(),
    };
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(page.into()))
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

/// Esc in the page.
#[tauri::command]
pub fn close_settings<R: Runtime>(window: tauri::Window<R>) {
    if let Err(err) = window.close() {
        eprintln!("failed to close the settings window: {err}");
    }
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

/// The shortcuts of this platform, one entry per bound action, straight from
/// the shortcut table, so the overview can't drift from what is bound.
#[tauri::command]
pub fn get_shortcuts(strings: tauri::State<'_, i18n::Current>) -> Vec<ShortcutView> {
    shortcuts_view(&strings.get(), cfg!(target_os = "macos"))
}

fn shortcuts_view(strings: &i18n::Strings, macos: bool) -> Vec<ShortcutView> {
    Action::ALL
        .iter()
        .filter(|action| !shortcuts::UNBOUND.contains(action))
        .map(|&action| ShortcutView {
            label: strings.get(action.label()).to_owned(),
            keys: shortcuts::action_key_caps(action, macos),
            mouse: shortcuts::mouse_button(action).map(|key| strings.get(key).to_owned()),
        })
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
    /// Every language the app offers, as `[tag, native name]`.
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

/// Applies `change` and saves it. A change of the language or of the update
/// check takes effect at once; the others are read when they are needed.
#[tauri::command]
pub fn set_setting<R: Runtime>(app: AppHandle<R>, change: SettingChange) -> Result<(), String> {
    let language = match &change {
        SettingChange::Language(value) => Some(parse_language(value)?),
        _ => None,
    };
    let check_updates = match &change {
        SettingChange::CheckUpdates(on) => Some(*on),
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
    if let Some(on) = check_updates {
        update::setting_changed(&app, on);
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

/// Switches the UI language: strings, tray menu, the macOS app menu, this
/// window, and the offline page if the main window shows it. The palette needs
/// nothing: it closes when it loses focus, so it can't be open during a change.
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
    // Rendered per request and never cached, so a reload brings the new strings.
    if let Some(main) = app.get_webview_window(crate::MAIN_WINDOW)
        && main.url().is_ok_and(|url| offline::is_page(&url))
        && let Err(err) = main.reload()
    {
        eprintln!("failed to reload the offline page: {err}");
    }
}

/// Facts for the About tab.
#[derive(Serialize)]
pub struct AppInfo {
    version: &'static str,
    repository: &'static str,
    license_url: String,
    update: UpdateView,
}

/// The update status for the About tab.
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum UpdateView {
    Off,
    Checking,
    UpToDate,
    Failed,
    Available {
        /// As shown to the user, e.g. `v0.3.0`.
        version: String,
        /// Its release page, opened in the browser.
        url: String,
    },
}

impl From<Status> for UpdateView {
    fn from(status: Status) -> Self {
        match status {
            Status::Off => Self::Off,
            Status::Checking => Self::Checking,
            Status::UpToDate => Self::UpToDate,
            Status::Failed => Self::Failed,
            Status::Available(update) => Self::Available {
                version: update.label(),
                url: update.url().into(),
            },
        }
    }
}

#[tauri::command]
pub fn get_app_info<R: Runtime>(app: AppHandle<R>) -> AppInfo {
    let repository = env!("CARGO_PKG_REPOSITORY");
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        repository,
        license_url: format!("{repository}/blob/main/LICENSE"),
        update: update::status(&app).into(),
    }
}

/// The account section. The token itself never reaches the page.
#[tauri::command]
pub fn get_account<R: Runtime>(app: AppHandle<R>) -> account::Status {
    account::status(&app)
}

#[tauri::command]
pub fn connect_account<R: Runtime>(app: AppHandle<R>) {
    account::connect(&app);
}

#[tauri::command]
pub fn cancel_connect_account<R: Runtime>(app: AppHandle<R>) {
    account::cancel(&app);
}

#[tauri::command]
pub fn disconnect_account<R: Runtime>(app: AppHandle<R>) {
    account::disconnect(&app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, json, to_value};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tauri::Listener;
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
            .invoke_handler(crate::invoke_handler::<MockRuntime>())
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
            "get_account",
            "settings_ready",
            // Last: it closes the window.
            "close_settings",
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
    #[cfg_attr(
        target_os = "macos",
        ignore = "rebuilds the app menu, which needs the main thread"
    )]
    fn switching_language_swaps_the_strings_and_tells_the_open_window() {
        let app = mock_builder()
            .invoke_handler(crate::invoke_handler::<MockRuntime>())
            .build(crate::context())
            .unwrap();
        app.manage(i18n::Current::new(i18n::Strings::load(i18n::BASE)));
        let settings = WebviewWindowBuilder::new(&app, LABEL, Default::default())
            .build()
            .unwrap();
        let told = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&told);
        settings.listen(STRINGS_CHANGED, move |_| {
            flag.store(true, Ordering::SeqCst);
        });

        apply_language(app.handle(), &Language::Tag("de-DE".into()));

        assert_eq!(app.state::<i18n::Current>().get().lang(), "de-DE");
        assert!(told.load(Ordering::SeqCst));
    }

    #[test]
    fn lists_every_action_with_its_shortcuts() {
        let strings = i18n::Strings::load(i18n::BASE);
        let view = shortcuts_view(&strings, false);
        assert_eq!(view.len(), Action::ALL.len() - shortcuts::UNBOUND.len());
        assert!(view.iter().all(|entry| !entry.keys.is_empty()));
        let reload = view.iter().find(|entry| entry.label == "Reload").unwrap();
        assert!(reload.keys.iter().all(|keys| !keys.is_empty()));
        assert!(!reload.keys.is_empty());
        assert!(reload.mouse.is_none());
    }

    #[test]
    fn update_status_reaches_the_page_as_its_state() {
        let state = |status| to_value(UpdateView::from(status)).unwrap();
        assert_eq!(state(Status::Off), json!({ "state": "off" }));
        assert_eq!(state(Status::Checking), json!({ "state": "checking" }));
        assert_eq!(state(Status::UpToDate), json!({ "state": "upToDate" }));
        assert_eq!(state(Status::Failed), json!({ "state": "failed" }));
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
