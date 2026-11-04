//! Your-turn and game-over alerts while the main window isn't focused (#23,
//! ADR 0021, ADR 0022): a native notification and taskbar attention, each with
//! its own setting. Everything is sent from Rust, so the page gets no capability.
//! The tray icon also shows a badge during the user's turn.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, Url, UserAttentionType, WebviewWindow, WindowEvent};
use tauri_plugin_notification::NotificationExt;

use crate::i18n::{self, Key};
use crate::navigation;
use crate::settings::{Settings, SettingsStore};
use crate::tray;
use crate::turn::{self, Alert, Badge, Detector, TurnState};

/// Tells our notifications apart from others in `on_action`.
const NOTIFICATION_ID: i32 = 23_000;

/// The key of the game's URL in a notification's extra payload.
const GAME_URL: &str = "gameUrl";

/// What the title and focus changes are compared against. In managed state.
#[derive(Default)]
struct Turn {
    detector: Detector,
    badge: Badge,
}

/// Registers the state and the click handler. A notification shown before a
/// handler exists reports no click, so this runs before the main window exists.
pub fn init<R: Runtime>(app: &AppHandle<R>) {
    app.manage(Mutex::new(Turn::default()));
    let handle = app.clone();
    let _ = app.notification().on_action(move |action| {
        if action.action_id() != "tap" {
            return;
        }
        if let Some(notification) = action.notification()
            && notification.id() == NOTIFICATION_ID
        {
            // This handler runs on a background thread.
            let app = handle.clone();
            let game = clicked_game(notification.extra());
            if let Err(err) = handle.run_on_main_thread(move || open_game(&app, game)) {
                eprintln!("failed to open the game: {err}");
            }
        }
    });
}

/// Keeps the tray badge in step with the focus, and clears the attention request
/// when the window gets focus. The docs of `request_user_attention` say the
/// window manager might not do it.
pub fn watch_focus<R: Runtime>(window: &WebviewWindow<R>) {
    let handle = window.clone();
    window.on_window_event(move |event| {
        let WindowEvent::Focused(focused) = event else {
            return;
        };
        let badge = lock_turn(&handle).badge.update(None, *focused);
        set_badge(&handle, badge);
        if *focused && let Err(err) = handle.request_user_attention(None) {
            eprintln!("failed to clear the attention request: {err}");
        }
    });
}

/// The main window's title changed: alerts the user if a game's state did.
pub fn title_changed<R: Runtime>(window: WebviewWindow<R>, title: String) {
    let game = window.url().ok().as_ref().and_then(game_url);
    let state = match game {
        Some(_) => turn::parse(&title),
        None => TurnState::Other,
    };
    // If the focus is unknown, count it as focused: better no alert than a wrong one.
    let focused = window.is_focused().unwrap_or(true);
    let (badge, alert) = {
        let mut turn = lock_turn(&window);
        let badge = turn.badge.update(Some(state), focused);
        let alert = game
            .as_ref()
            .and_then(|game| turn.detector.observe(game.as_str(), state, focused));
        (badge, alert)
    };
    set_badge(&window, badge);
    if let (Some(alert), Some(game)) = (alert, &game) {
        send(&window, alert, game);
    }
}

fn lock_turn<'a, R: Runtime>(window: &'a WebviewWindow<R>) -> MutexGuard<'a, Turn> {
    window
        .state::<Mutex<Turn>>()
        .inner()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
}

/// Sets the tray badge if `change` says it changed. Called without the lock on
/// [`Turn`], since this may wait for the main thread.
fn set_badge<R: Runtime>(window: &WebviewWindow<R>, change: Option<bool>) {
    if let Some(show) = change
        && let Err(err) = tray::set_turn_badge(window.app_handle(), show)
    {
        eprintln!("failed to update the tray icon: {err}");
    }
}

fn send<R: Runtime>(window: &WebviewWindow<R>, alert: Alert, game: &Url) {
    let settings = window.state::<SettingsStore<R>>().get();
    if let Some(key) = notification_text(alert, &settings) {
        let body = window.state::<i18n::Current>().get().get(key).to_owned();
        // No sound on purpose: lichess plays its own (ADR 0021).
        if let Err(err) = window
            .notification()
            .builder()
            .id(NOTIFICATION_ID)
            .body(body)
            .extra(GAME_URL, game.as_str())
            .show()
        {
            eprintln!("failed to show the notification: {err}");
        }
    }
    if settings.attention
        && let Err(err) = window.request_user_attention(Some(UserAttentionType::Informational))
    {
        eprintln!("failed to request attention: {err}");
    }
}

/// The text of the notification for `alert`, or `None` if its setting is off.
fn notification_text(alert: Alert, settings: &Settings) -> Option<Key> {
    match alert {
        Alert::YourTurn => settings.notify_your_turn.then_some(Key::NotifyYourTurn),
        Alert::GameOver => settings.notify_game_over.then_some(Key::NotifyGameOver),
    }
}

/// Identifies the game on `url`: a lichess page without its fragment, which
/// the page changes while the game runs. `None` for anything else.
fn game_url(url: &Url) -> Option<Url> {
    let mut game = url.clone();
    game.set_fragment(None);
    navigation::is_lichess(&game).then_some(game)
}

/// The game a clicked notification belongs to. The payload passes through the
/// notification server, so it is checked again.
fn clicked_game(extra: &HashMap<String, Value>) -> Option<Url> {
    game_url(&extra.get(GAME_URL)?.as_str()?.parse().ok()?)
}

/// Brings the window to the front and shows `game`, unless it is showing already.
fn open_game<R: Runtime>(app: &AppHandle<R>, game: Option<Url>) {
    crate::raise_main_window(app);
    let (Some(window), Some(game)) = (app.get_webview_window(crate::MAIN_WINDOW), game) else {
        return;
    };
    let shown = window.url().ok().as_ref().and_then(game_url);
    if shown.as_ref() != Some(&game)
        && let Err(err) = window.navigate(game)
    {
        eprintln!("failed to open the game: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tauri_plugin_notification::ActionPerformed;

    fn url(url: &str) -> Url {
        url.parse().unwrap()
    }

    #[test]
    fn each_notification_has_its_own_setting() {
        let on = Settings::default();
        assert_eq!(
            notification_text(Alert::YourTurn, &on),
            Some(Key::NotifyYourTurn)
        );
        assert_eq!(
            notification_text(Alert::GameOver, &on),
            Some(Key::NotifyGameOver)
        );

        let no_turn = Settings {
            notify_your_turn: false,
            ..Settings::default()
        };
        assert_eq!(notification_text(Alert::YourTurn, &no_turn), None);
        assert_eq!(
            notification_text(Alert::GameOver, &no_turn),
            Some(Key::NotifyGameOver)
        );

        let no_end = Settings {
            notify_game_over: false,
            ..Settings::default()
        };
        assert_eq!(
            notification_text(Alert::YourTurn, &no_end),
            Some(Key::NotifyYourTurn)
        );
        assert_eq!(notification_text(Alert::GameOver, &no_end), None);
    }

    #[test]
    fn a_game_is_a_lichess_page_without_its_fragment() {
        assert_eq!(
            game_url(&url("https://lichess.org/abcd1234efgh#12")),
            Some(url("https://lichess.org/abcd1234efgh"))
        );
        assert_eq!(game_url(&url("https://example.com/abcd1234")), None);
        assert_eq!(game_url(&url("http://lichess.org/abcd1234")), None);
        assert_eq!(game_url(&url("https://challenges.cloudflare.com/x")), None);
    }

    /// The payload the plugin reports for a click, as in its own tests.
    fn clicked(extra: Value) -> ActionPerformed {
        serde_json::from_value(json!({
            "actionId": "tap",
            "inputValue": null,
            "notification": { "id": NOTIFICATION_ID, "extra": extra },
        }))
        .unwrap()
    }

    #[test]
    fn a_click_finds_the_game_in_the_payload() {
        let action = clicked(json!({ GAME_URL: "https://lichess.org/abcd1234efgh" }));
        let notification = action.notification().unwrap();
        assert_eq!(notification.id(), NOTIFICATION_ID);
        assert_eq!(
            clicked_game(notification.extra()),
            Some(url("https://lichess.org/abcd1234efgh"))
        );
    }

    #[test]
    fn a_click_only_goes_to_lichess() {
        for extra in [
            json!({}),
            json!({ GAME_URL: 7 }),
            json!({ GAME_URL: "not a url" }),
            json!({ GAME_URL: "https://example.com/abcd1234" }),
            json!({ GAME_URL: "javascript:alert(1)" }),
        ] {
            let action = clicked(extra.clone());
            assert_eq!(
                clicked_game(action.notification().unwrap().extra()),
                None,
                "{extra}"
            );
        }
    }
}
