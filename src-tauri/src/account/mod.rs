//! The opt-in Lichess account connection (#24, ADR 0024): OAuth PKCE in the
//! main window, the token in the OS credential store, and no IPC for the page.
//! The token stays in this module: no command returns it, and nothing logs it.
//!
//! The store is only touched once the user has asked to connect, so everyone
//! else never sees an unlock prompt. The `account_connected` setting says when
//! there is a token to look for.

mod api;
mod events;
mod oauth;
mod stream;
mod token_store;

use std::sync::Mutex;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, Url};

use crate::settings::SettingsStore;
use crate::settings_window::{self, Tab};
use api::Account;
use oauth::{Callback, Request};

/// Tells an open settings window that [`status`] changed.
const CHANGED: &str = "account-changed";

/// Opening the settings window checks the token again at most this often. Lila
/// asks clients not to poll the account endpoint.
const RECHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// What the settings window shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Status {
    Disconnected {
        notice: Option<Notice>,
    },
    /// Waiting for the user on the authorize page in the main window.
    Connecting,
    /// Busy with the store or lichess.org.
    Checking,
    Connected {
        username: String,
    },
    /// There is a token, but lichess.org didn't say whose.
    Unverified,
}

/// Why the account is disconnected, when the user should know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Notice {
    /// Lichess rejected the token.
    Expired,
    /// The credential store can't be used, so nothing was saved.
    NoKeyring,
    /// The authorization didn't complete.
    Failed,
    /// Disconnected here, but the token may still be valid on lichess.org.
    RevokeFailed,
}

/// An attempt waiting for its callback.
struct Pending {
    request: Request,
    /// Where the main window goes back to.
    return_to: Option<Url>,
}

struct Inner {
    status: Status,
    pending: Option<Pending>,
    /// Bumped by every user action, so a check that started earlier can't
    /// overwrite its result.
    generation: u64,
    checked_at: Option<Instant>,
    /// Dropping it stops the running event stream.
    stream: Option<Sender<()>>,
}

impl Inner {
    /// The attempt a callback answers, which it uses up. Any other callback
    /// changes nothing, so a forged one can't end a running attempt.
    fn take_answered(&mut self, callback: &Callback) -> Option<Pending> {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.request.answered_by(callback))
        {
            self.pending.take()
        } else {
            None
        }
    }
}

/// In managed state.
struct State(Mutex<Inner>);

impl Default for State {
    fn default() -> Self {
        Self(Mutex::new(Inner {
            status: Status::Disconnected { notice: None },
            pending: None,
            generation: 0,
            checked_at: None,
            stream: None,
        }))
    }
}

fn with_state<R: Runtime, T>(app: &AppHandle<R>, read: impl FnOnce(&mut Inner) -> T) -> Option<T> {
    let state = app.try_state::<State>()?;
    let mut inner = state.0.lock().unwrap_or_else(|err| err.into_inner());
    Some(read(&mut inner))
}

pub fn status<R: Runtime>(app: &AppHandle<R>) -> Status {
    with_state(app, |inner| inner.status.clone()).unwrap_or(Status::Disconnected { notice: None })
}

fn set_status<R: Runtime>(app: &AppHandle<R>, status: Status) {
    with_state(app, |inner| inner.status = status);
    status_changed(app);
}

fn status_changed<R: Runtime>(app: &AppHandle<R>) {
    if app.get_webview_window(settings_window::LABEL).is_some() {
        let _ = app.emit_to(settings_window::LABEL, CHANGED, ());
    }
    sync_stream(app);
}

/// The event stream runs while there is a token and its setting is on.
fn wants_stream(status: &Status, notify_account_events: bool) -> bool {
    notify_account_events && matches!(status, Status::Connected { .. } | Status::Unverified)
}

/// Starts or stops the event stream to match the status and the setting.
fn sync_stream<R: Runtime>(app: &AppHandle<R>) {
    let on = app.state::<SettingsStore<R>>().get().notify_account_events;
    let start = with_state(app, |inner| {
        if !wants_stream(&inner.status, on) {
            inner.stream = None;
            return None;
        }
        if inner.stream.is_some() {
            return None;
        }
        let (sender, stop) = mpsc::channel();
        inner.stream = Some(sender);
        let username = match &inner.status {
            Status::Connected { username } => Some(username.clone()),
            _ => None,
        };
        Some((stop, username))
    })
    .flatten();
    if let Some((stop, username)) = start {
        let app = app.clone();
        thread::spawn(move || stream::run(&app, &stop, username));
    }
}

/// The stream's 401: the token is no good, whatever a running check may say.
fn token_rejected<R: Runtime>(app: &AppHandle<R>) {
    let status = forget_rejected_token(app);
    with_state(app, |inner| {
        inner.generation += 1;
        inner.status = status;
    });
    status_changed(app);
}

/// Lichess.org answered after all, which the settings window can show.
fn token_verified<R: Runtime>(app: &AppHandle<R>, username: &str) {
    let changed = with_state(app, |inner| {
        let unverified = inner.status == Status::Unverified;
        if unverified {
            inner.status = Status::Connected {
                username: username.to_owned(),
            };
        }
        unverified
    });
    if changed == Some(true) {
        status_changed(app);
    }
}

/// Moves on from `from` to `to` for a user action, unless the state has moved
/// on already, such as after a double click.
fn begin<R: Runtime>(app: &AppHandle<R>, from: impl Fn(&Status) -> bool, to: Status) -> bool {
    let started = with_state(app, |inner| {
        if !from(&inner.status) {
            return false;
        }
        inner.status = to;
        inner.generation += 1;
        true
    })
    .unwrap_or(false);
    if started {
        status_changed(app);
    }
    started
}

/// Registers the state and, if the user has connected before, checks the token
/// in the background. Needs [`crate::settings::init`].
pub fn init<R: Runtime>(app: &AppHandle<R>) {
    app.manage(State::default());
    if is_saved_as_connected(app) {
        set_status(app, Status::Checking);
        start_check(app);
    }
}

fn is_saved_as_connected<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<SettingsStore<R>>().get().account_connected
}

fn save_connected<R: Runtime>(app: &AppHandle<R>, connected: bool) {
    let saved = app
        .state::<SettingsStore<R>>()
        .update(|settings| settings.account_connected = connected);
    if let Err(err) = saved {
        eprintln!("failed to save the account setting: {err}");
    }
}

/// Checks the token again when the settings window opens, so a token revoked
/// while the app was running shows up there.
pub fn settings_opened<R: Runtime>(app: &AppHandle<R>) {
    let due = with_state(app, |inner| {
        let connected = matches!(inner.status, Status::Connected { .. } | Status::Unverified);
        connected
            && inner
                .checked_at
                .is_none_or(|checked_at| checked_at.elapsed() >= RECHECK_INTERVAL)
    });
    if due == Some(true) {
        start_check(app);
    }
}

fn start_check<R: Runtime>(app: &AppHandle<R>) {
    let Some(generation) = with_state(app, |inner| {
        inner.checked_at = Some(Instant::now());
        inner.generation
    }) else {
        return;
    };
    let app = app.clone();
    thread::spawn(move || {
        let status = check_saved_token(&app);
        // Only if the user hasn't acted in the meantime.
        with_state(&app, |inner| {
            if inner.generation == generation {
                inner.status = status;
            }
        });
        status_changed(&app);
    });
}

fn check_saved_token<R: Runtime>(app: &AppHandle<R>) -> Status {
    match token_store::read() {
        Ok(Some(token)) => status_of_token(app, &token),
        Ok(None) => {
            save_connected(app, false);
            Status::Disconnected {
                notice: Some(Notice::Expired),
            }
        }
        Err(err) => {
            eprintln!("failed to read the token: {err}");
            Status::Disconnected {
                notice: Some(Notice::NoKeyring),
            }
        }
    }
}

/// Asks lichess.org about the saved `token`, and forgets it if it is rejected.
fn status_of_token<R: Runtime>(app: &AppHandle<R>, token: &str) -> Status {
    match api::account(token) {
        Account::Username(username) => Status::Connected { username },
        Account::Unknown => Status::Unverified,
        Account::Rejected => forget_rejected_token(app),
    }
}

fn forget_rejected_token<R: Runtime>(app: &AppHandle<R>) -> Status {
    if let Err(err) = token_store::remove() {
        eprintln!("failed to remove the rejected token: {err}");
    }
    save_connected(app, false);
    Status::Disconnected {
        notice: Some(Notice::Expired),
    }
}

/// "Connect": checks that the token will have a safe place to go, then opens
/// the authorize page in the main window, where the user is logged in.
pub fn connect<R: Runtime>(app: &AppHandle<R>) {
    let is_disconnected = |status: &Status| matches!(status, Status::Disconnected { .. });
    if !begin(app, is_disconnected, Status::Checking) {
        return;
    }
    let app = app.clone();
    thread::spawn(move || {
        if let Err(err) = token_store::read() {
            eprintln!("no usable credential store: {err}");
            set_status(
                &app,
                Status::Disconnected {
                    notice: Some(Notice::NoKeyring),
                },
            );
            return;
        }
        if let Err(err) = open_authorize_page(&app) {
            eprintln!("failed to open the authorize page: {err}");
            set_status(
                &app,
                Status::Disconnected {
                    notice: Some(Notice::Failed),
                },
            );
        }
    });
}

fn open_authorize_page<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let main = app
        .get_webview_window(crate::MAIN_WINDOW)
        .ok_or("the main window is gone")?;
    let request = Request::new()?;
    let url = request.authorize_url();
    with_state(app, |inner| {
        inner.pending = Some(Pending {
            request,
            return_to: main.url().ok(),
        });
    });
    set_status(app, Status::Connecting);
    main.navigate(url)?;
    // It stays above its parent and would cover the authorize page.
    if let Some(settings) = app.get_webview_window(settings_window::LABEL) {
        settings.close()?;
    }
    crate::focus_main_window(app);
    Ok(())
}

/// "Cancel" while waiting for the user: forgets the attempt and leaves the
/// authorize page, if the main window is still on it.
pub fn cancel<R: Runtime>(app: &AppHandle<R>) {
    let was_waiting = begin(
        app,
        |status| *status == Status::Connecting,
        Status::Disconnected { notice: None },
    );
    if !was_waiting {
        return;
    }
    let pending = with_state(app, |inner| inner.pending.take()).flatten();
    let on_authorize_page = app
        .get_webview_window(crate::MAIN_WINDOW)
        .and_then(|main| main.url().ok())
        .is_some_and(|url| crate::navigation::is_lichess(&url) && url.path().starts_with("/oauth"));
    if on_authorize_page && let Some(pending) = pending {
        return_main_window(app, pending.return_to);
    }
}

fn return_main_window<R: Runtime>(app: &AppHandle<R>, url: Option<Url>) {
    let url = url
        .filter(crate::navigation::is_internal)
        .unwrap_or_else(|| crate::START_URL.parse().expect("START_URL is a valid URL"));
    if let Some(main) = app.get_webview_window(crate::MAIN_WINDOW)
        && let Err(err) = main.navigate(url)
    {
        eprintln!("failed to return the main window: {err}");
    }
}

/// "Disconnect": revokes the token on lichess.org and removes it here.
pub fn disconnect<R: Runtime>(app: &AppHandle<R>) {
    let is_connected =
        |status: &Status| matches!(status, Status::Connected { .. } | Status::Unverified);
    if !begin(app, is_connected, Status::Checking) {
        return;
    }
    let app = app.clone();
    thread::spawn(move || {
        let status = disconnected_status(&app);
        set_status(&app, status);
    });
}

fn disconnected_status<R: Runtime>(app: &AppHandle<R>) -> Status {
    let no_keyring = |err: keyring_core::Error| {
        eprintln!("failed to reach the token: {err}");
        Status::Disconnected {
            notice: Some(Notice::NoKeyring),
        }
    };
    let token = match token_store::read() {
        Ok(token) => token,
        Err(err) => return no_keyring(err),
    };
    let mut notice = None;
    if let Some(token) = token
        && let Err(err) = api::revoke(&token)
    {
        eprintln!("failed to revoke the token: {err}");
        notice = Some(Notice::RevokeFailed);
    }
    if let Err(err) = token_store::remove() {
        return no_keyring(err);
    }
    save_connected(app, false);
    Status::Disconnected { notice }
}

/// For the main window's navigation handlers. Returns `true` if `url` is the
/// OAuth callback, which must neither load nor reach the system browser, as
/// it may carry an authorization code.
pub fn intercept<R: Runtime>(app: &AppHandle<R>, url: &Url) -> bool {
    if !oauth::is_callback(url) {
        return false;
    }
    let callback = oauth::parse_callback(url);
    let app = app.clone();
    // Handlers run on the main thread, and this blocks.
    thread::spawn(move || finish(&app, &callback));
    true
}

fn finish<R: Runtime>(app: &AppHandle<R>, callback: &Callback) {
    let Some(pending) = with_state(app, |inner| inner.take_answered(callback)).flatten() else {
        eprintln!("ignored an OAuth callback that answers no attempt");
        return;
    };
    set_status(app, Status::Checking);
    return_main_window(app, pending.return_to);
    let status = match &callback.code {
        Some(code) => redeem(app, code, &pending.request.verifier),
        None => Status::Disconnected {
            notice: Some(Notice::Failed),
        },
    };
    with_state(app, |inner| inner.checked_at = Some(Instant::now()));
    set_status(app, status);
    if let Err(err) = settings_window::open_tab(app, Tab::Settings) {
        eprintln!("failed to open the settings window: {err}");
    }
}

/// Turns the code into a saved token. A token that can't be saved is revoked
/// right away and never written anywhere else.
fn redeem<R: Runtime>(app: &AppHandle<R>, code: &str, verifier: &str) -> Status {
    let token = match api::exchange_code(code, verifier) {
        Ok(token) => token,
        Err(err) => {
            eprintln!("failed to get a token: {err}");
            return Status::Disconnected {
                notice: Some(Notice::Failed),
            };
        }
    };
    if let Err(err) = token_store::write(&token) {
        eprintln!("failed to save the token: {err}");
        if let Err(err) = api::revoke(&token) {
            eprintln!("failed to revoke the unsaved token: {err}");
        }
        return Status::Disconnected {
            notice: Some(Notice::NoKeyring),
        };
    }
    save_connected(app, true);
    status_of_token(app, &token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, to_value};

    fn callback_for(url: &Url) -> Callback {
        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .unwrap();
        let answer = format!("{}?code=c&state={state}", oauth::REDIRECT_URI);
        oauth::parse_callback(&answer.parse().unwrap())
    }

    fn inner_with_pending() -> (Inner, Url) {
        let request = Request::new().unwrap();
        let url = request.authorize_url();
        let inner = Inner {
            status: Status::Connecting,
            pending: Some(Pending {
                request,
                return_to: None,
            }),
            generation: 0,
            checked_at: None,
            stream: None,
        };
        (inner, url)
    }

    #[test]
    fn a_callback_uses_up_the_attempt_it_answers() {
        let (mut inner, url) = inner_with_pending();
        let callback = callback_for(&url);
        assert!(inner.take_answered(&callback).is_some());
        assert!(inner.pending.is_none());
        assert!(inner.take_answered(&callback).is_none(), "replayed");
    }

    #[test]
    fn a_forged_callback_leaves_the_attempt_alone() {
        let (mut inner, _) = inner_with_pending();
        let forged = format!("{}?code=c&state=forged", oauth::REDIRECT_URI);
        let forged = oauth::parse_callback(&forged.parse().unwrap());
        assert!(inner.take_answered(&forged).is_none());
        assert!(inner.pending.is_some());
    }

    #[test]
    fn the_callback_is_not_a_page_of_the_app_and_not_for_the_browser() {
        let url: Url = oauth::REDIRECT_URI.parse().unwrap();
        assert!(!crate::navigation::is_internal(&url));
        assert!(oauth::is_callback(&url));
    }

    #[test]
    fn the_stream_needs_a_token_and_the_setting() {
        let connected = Status::Connected {
            username: "SomeOne".into(),
        };
        assert!(wants_stream(&connected, true));
        assert!(wants_stream(&Status::Unverified, true));
        assert!(!wants_stream(&connected, false));
        assert!(!wants_stream(&Status::Unverified, false));
        for status in [
            Status::Disconnected { notice: None },
            Status::Connecting,
            Status::Checking,
        ] {
            assert!(!wants_stream(&status, true), "{status:?}");
        }
    }

    #[test]
    fn status_reaches_the_page_as_its_state() {
        let state = |status: Status| to_value(status).unwrap();
        assert_eq!(
            state(Status::Disconnected { notice: None }),
            json!({ "state": "disconnected", "notice": null })
        );
        assert_eq!(
            state(Status::Disconnected {
                notice: Some(Notice::NoKeyring)
            }),
            json!({ "state": "disconnected", "notice": "noKeyring" })
        );
        assert_eq!(state(Status::Connecting), json!({ "state": "connecting" }));
        assert_eq!(state(Status::Checking), json!({ "state": "checking" }));
        assert_eq!(state(Status::Unverified), json!({ "state": "unverified" }));
        assert_eq!(
            state(Status::Connected {
                username: "SomeOne".into()
            }),
            json!({ "state": "connected", "username": "SomeOne" })
        );
    }
}
