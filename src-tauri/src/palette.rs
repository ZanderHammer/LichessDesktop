//! The command palette (#16): a small frameless window with bundled HTML in
//! `ui/palette.html`, opened with Ctrl/Cmd+K over the main window. It lists
//! the app actions and a few lichess pages, and closes on Esc or when it
//! loses focus. Like the settings window it has scoped IPC (ADR 0012): its
//! capability, `capabilities/palette.json`, grants exactly the commands
//! below plus `get_strings`.
//!
//! The window is created for every open and destroyed on close, so it costs
//! no memory while it isn't shown.

use serde::Serialize;
use tauri::webview::NewWindowResponse;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, Url, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};

use crate::actions::Action;
use crate::i18n::{self, Key};
use crate::{MAIN_WINDOW, START_URL, bundled, navigation, shortcuts};

pub const LABEL: &str = "palette";

/// The window size, from DESIGN.md.
const WIDTH: f64 = 600.0;
const HEIGHT: f64 = 360.0;

/// A lichess page the palette can open in the main window. Pages are fixed
/// here and picked by id, so the palette never hands Rust a URL (ADR 0012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Destination {
    pub id: &'static str,
    pub label: Key,
    /// Path and query on lichess.org, starting with `/`.
    pub path: &'static str,
}

const fn page(id: &'static str, label: Key, path: &'static str) -> Destination {
    Destination { id, label, path }
}

/// The pages of lichess's top menu (lila `TopNav.scala`) that need no
/// username or game id, and the account pages. `?any` makes the lobby load
/// again, so it opens the dialog in the hash even when it is already shown;
/// lichess's own menu links do the same.
pub const DESTINATIONS: &[Destination] = &[
    page("page.createGame", Key::CreateLobbyGame, "/?any#hook"),
    page(
        "page.challengeFriend",
        Key::ChallengeAFriend,
        "/?any#friend",
    ),
    page("page.playComputer", Key::PlayAgainstComputer, "/?any#ai"),
    page("page.tournaments", Key::Tournaments, "/tournament"),
    page("page.puzzles", Key::Puzzles, "/training"),
    page("page.learn", Key::ChessBasics, "/learn"),
    page("page.study", Key::Study, "/study"),
    page("page.broadcasts", Key::Broadcasts, "/broadcast"),
    page("page.tv", Key::LichessTv, "/tv"),
    page("page.games", Key::CurrentGames, "/games"),
    page("page.analysis", Key::AnalysisBoard, "/analysis"),
    page("page.editor", Key::BoardEditor, "/editor"),
    page("page.inbox", Key::Inbox, "/inbox"),
    page("page.profile", Key::EditProfile, "/account/profile"),
    page(
        "page.preferences",
        Key::Preferences,
        "/account/preferences/display",
    ),
];

impl Destination {
    pub fn url(self) -> Url {
        let base: Url = START_URL.parse().expect("START_URL is a valid URL");
        base.join(self.path)
            .expect("a destination path is a valid URL")
    }
}

/// Opens the palette over the main window, or focuses it if it is open. A new
/// window stays hidden until the page has drawn its content (`palette_ready`).
pub fn open<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.show()?;
        return window.set_focus();
    }
    crate::focus_main_window(app);
    let Some(main) = app.get_webview_window(MAIN_WINDOW) else {
        return Ok(());
    };
    let dev_url = bundled::dev_url(app);
    let title = app
        .state::<i18n::Current>()
        .get()
        .get(Key::CommandPalette)
        .to_owned();
    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("palette.html".into()))
        .title(title)
        .inner_size(WIDTH, HEIGHT)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .decorations(false)
        .skip_taskbar(true)
        .visible(false)
        .parent(&main)?
        // Only the bundled page itself. It has no links.
        .on_navigation(move |url| navigation::is_bundled(url, dev_url.as_ref()))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_page_load(bundled::show_after_fallback);
    // Wayland has no global window positions: there the compositor places
    // the window, usually over its parent.
    builder = match centred_over(&main) {
        Some(position) => builder.position(position.x, position.y),
        None => builder.center(),
    };
    let window = builder.build()?;
    let blur_handle = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            close(&blur_handle);
        }
    });
    Ok(())
}

/// Where the palette goes to sit in the middle of `main`.
fn centred_over<R: Runtime>(main: &tauri::WebviewWindow<R>) -> Option<LogicalPosition<f64>> {
    let scale = main.scale_factor().ok()?;
    let position = main.outer_position().ok()?.to_logical(scale);
    let size = main.outer_size().ok()?.to_logical(scale);
    Some(centred(position, size))
}

fn centred(position: LogicalPosition<f64>, size: LogicalSize<f64>) -> LogicalPosition<f64> {
    LogicalPosition::new(
        position.x + (size.width - WIDTH) / 2.0,
        position.y + (size.height - HEIGHT) / 2.0,
    )
}

/// Closes the palette if it is open.
pub fn close<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(LABEL)
        && let Err(err) = window.close()
    {
        eprintln!("failed to close the command palette: {err}");
    }
}

/// The page has drawn its content: show the window.
#[tauri::command]
pub fn palette_ready<R: Runtime>(webview: tauri::Webview<R>) {
    bundled::show(&webview.window());
}

/// Esc in the page.
#[tauri::command]
pub fn close_palette<R: Runtime>(app: AppHandle<R>) {
    close(&app);
}

/// Which list an entry is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    /// App actions.
    App,
    /// lichess.org pages.
    Lichess,
}

/// One entry of the palette.
#[derive(Debug, Serialize, PartialEq)]
pub struct Entry {
    id: &'static str,
    label: String,
    group: Group,
    /// The first shortcut of an action as its key caps, e.g. `["Ctrl", "R"]`.
    /// Empty for pages and actions without one.
    keys: Vec<String>,
}

/// Every entry, in the UI language: the app actions (with their shortcut on
/// this platform), then the lichess pages.
#[tauri::command]
pub fn list_actions(strings: tauri::State<'_, i18n::Current>) -> Vec<Entry> {
    entries(&strings.get(), cfg!(target_os = "macos"))
}

fn entries(strings: &i18n::Strings, macos: bool) -> Vec<Entry> {
    let actions = Action::ALL
        .iter()
        // It is open already.
        .filter(|&&action| action != Action::CommandPalette)
        .map(|&action| Entry {
            id: action.id(),
            label: strings.get(action.label()).to_owned(),
            group: Group::App,
            keys: shortcuts::action_key_caps(action, macos)
                .into_iter()
                .next()
                .unwrap_or_default(),
        });
    let pages = DESTINATIONS.iter().map(|destination| Entry {
        id: destination.id,
        label: strings.get(destination.label).to_owned(),
        group: Group::Lichess,
        keys: Vec::new(),
    });
    actions.chain(pages).collect()
}

/// What an entry id stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Action(Action),
    Page(Destination),
}

fn find(id: &str) -> Option<Target> {
    Action::ALL
        .iter()
        .find(|action| action.id() == id)
        .map(|&action| Target::Action(action))
        .or_else(|| {
            DESTINATIONS
                .iter()
                .find(|destination| destination.id == id)
                .map(|&destination| Target::Page(destination))
        })
}

/// Closes the palette and runs the entry `id`: an action runs the same
/// function as its shortcut, a page opens in the main window. Async so that
/// it doesn't run on the main thread: opening a window from a sync command
/// can deadlock on Windows.
#[tauri::command]
pub async fn run_action<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), String> {
    let target = find(&id).ok_or_else(|| format!("unknown palette entry {id:?}"))?;
    close(&app);
    crate::focus_main_window(&app);
    match target {
        Target::Action(action) => action.run(&app),
        Target::Page(destination) => open_page(&app, destination),
    }
    Ok(())
}

/// Opens `destination` in the main window, if the navigation allowlist
/// lets it in (ADR 0005).
fn open_page<R: Runtime>(app: &AppHandle<R>, destination: Destination) {
    let url = destination.url();
    if !navigation::is_internal(&url) {
        eprintln!("refused to open {url}: not on the navigation allowlist");
        return;
    }
    if let Some(window) = app.get_webview_window(MAIN_WINDOW)
        && let Err(err) = window.navigate(url)
    {
        eprintln!("failed to open {}: {err}", destination.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundled::test_support::{LOCAL, request};
    use tauri::test::{MockRuntime, get_ipc_response, mock_builder};

    /// Runs real IPC requests through the app's ACL (ADR 0012): only the
    /// palette's bundled page may call the palette commands, and the palette
    /// may call no settings command but `get_strings`.
    #[test]
    fn only_the_palette_page_may_call_palette_commands() {
        let app = mock_builder()
            .invoke_handler(crate::invoke_handler::<MockRuntime>())
            .build(crate::context())
            .unwrap();
        app.manage(i18n::Current::new(i18n::Strings::load(i18n::BASE)));
        let palette = WebviewWindowBuilder::new(&app, LABEL, Default::default())
            .build()
            .unwrap();
        let settings =
            WebviewWindowBuilder::new(&app, crate::settings_window::LABEL, Default::default())
                .build()
                .unwrap();
        let main = WebviewWindowBuilder::new(&app, MAIN_WINDOW, Default::default())
            .build()
            .unwrap();

        for cmd in ["get_strings", "list_actions"] {
            assert!(
                get_ipc_response(&palette, request(cmd, LOCAL)).is_ok(),
                "{cmd} denied"
            );
        }
        for cmd in [
            "list_actions",
            "run_action",
            "palette_ready",
            "close_palette",
        ] {
            assert!(
                get_ipc_response(&main, request(cmd, LOCAL)).is_err(),
                "{cmd} allowed in main"
            );
            assert!(
                get_ipc_response(&settings, request(cmd, LOCAL)).is_err(),
                "{cmd} allowed in settings"
            );
            assert!(
                get_ipc_response(&palette, request(cmd, "https://lichess.org/")).is_err(),
                "{cmd} allowed for remote content"
            );
        }
        for cmd in [
            "get_settings",
            "set_setting",
            "get_shortcuts",
            "plugin:event|listen",
        ] {
            assert!(
                get_ipc_response(&palette, request(cmd, LOCAL)).is_err(),
                "{cmd} allowed in the palette"
            );
        }
    }

    #[test]
    fn lists_every_action_but_itself_then_every_page() {
        let strings = i18n::Strings::load(i18n::BASE);
        let entries = entries(&strings, false);
        assert_eq!(entries.len(), Action::ALL.len() - 1 + DESTINATIONS.len());
        assert!(
            !entries
                .iter()
                .any(|entry| entry.id == Action::CommandPalette.id())
        );
        let reload = entries
            .iter()
            .find(|entry| entry.label == "Reload")
            .unwrap();
        assert_eq!(reload.group, Group::App);
        assert_eq!(reload.keys, ["F5"]);
        let about = entries
            .iter()
            .find(|entry| entry.id == Action::About.id())
            .unwrap();
        assert!(about.keys.is_empty());
        let puzzles = entries
            .iter()
            .find(|entry| entry.label == "Puzzles")
            .unwrap();
        assert_eq!(puzzles.group, Group::Lichess);
        let first_page = entries
            .iter()
            .position(|entry| entry.group == Group::Lichess)
            .unwrap();
        assert!(
            entries[first_page..]
                .iter()
                .all(|entry| entry.group == Group::Lichess)
        );
    }

    #[test]
    fn finds_actions_and_pages_by_id_only() {
        assert_eq!(find("action.reload"), Some(Target::Action(Action::Reload)));
        assert_eq!(find("page.puzzles"), Some(Target::Page(DESTINATIONS[4])));
        for id in ["", "reload", "https://lichess.org/", "page.", "action.nope"] {
            assert_eq!(find(id), None, "{id:?}");
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = Action::ALL
            .iter()
            .map(|action| action.id())
            .chain(DESTINATIONS.iter().map(|destination| destination.id))
            .collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count);
    }

    #[test]
    fn every_page_is_on_lichess() {
        for destination in DESTINATIONS {
            let url = destination.url();
            assert!(destination.path.starts_with('/'), "{destination:?}");
            assert!(navigation::is_lichess(&url), "{url}");
            assert_eq!(url.host_str(), Some("lichess.org"), "{url}");
        }
    }

    #[test]
    fn centres_over_the_main_window() {
        let position = centred(
            LogicalPosition::new(100.0, 50.0),
            LogicalSize::new(1280.0, 800.0),
        );
        assert_eq!(position, LogicalPosition::new(440.0, 270.0));
    }
}
