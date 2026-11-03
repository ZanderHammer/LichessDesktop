//! App actions: what a shortcut, the macOS menu, and later the command palette
//! and the tray can do. Each action is a plain function on the [`AppHandle`],
//! and [`Action`] names it so a table or a menu can refer to it.
//!
//! To add an action, add a line to the `actions!` list below and write its
//! function in this module. Its shortcuts go in [`crate::shortcuts::SHORTCUTS`].

use tauri::{AppHandle, Manager, Runtime, Url, WebviewWindow};

use crate::{MAIN_WINDOW, START_URL, native, navigation, zoom};

/// Declares [`Action`] and the function each variant runs in one place, so
/// the list can't drift from the enum.
macro_rules! actions {
    ($($(#[$doc:meta])* $variant:ident => $function:ident,)*) => {
        /// Something the user can trigger. [`Action::run`] calls the function
        /// of the same name in this module.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Action {
            $($(#[$doc])* $variant,)*
        }

        impl Action {
            /// Every action, in menu order.
            #[allow(dead_code, reason = "used by the macOS menu and the tests")]
            pub const ALL: &[Action] = &[$(Action::$variant,)*];

            /// A stable id, for example for menu items.
            #[allow(dead_code, reason = "used by the macOS menu and the tests")]
            pub const fn id(self) -> &'static str {
                match self {
                    $(Action::$variant => concat!("action.", stringify!($function)),)*
                }
            }

            pub fn run<R: Runtime>(self, app: &AppHandle<R>) {
                match self {
                    $(Action::$variant => $function(app),)*
                }
            }
        }
    };
}

actions! {
    /// Goes back in the main window's history.
    Back => back,
    /// Goes forward in the main window's history.
    Forward => forward,
    /// Reloads the page in the main window.
    Reload => reload,
    /// Opens the lichess.org start page in the main window.
    Home => home,
    /// Copies the address of the lichess page in the main window.
    CopyUrl => copy_url,
    /// Opens the lichess page of the main window in the system browser.
    OpenInBrowser => open_in_browser,
    /// Zooms the page of the main window in one step.
    ZoomIn => zoom_in,
    /// Zooms the page of the main window out one step.
    ZoomOut => zoom_out,
    /// Resets the zoom of the main window's page to 100 %.
    ZoomReset => zoom_reset,
    /// Turns fullscreen on or off for the main window, for this session.
    Fullscreen => toggle_fullscreen,
}

pub fn back<R: Runtime>(app: &AppHandle<R>) {
    with_main_webview(app, native::go_back);
}

pub fn forward<R: Runtime>(app: &AppHandle<R>) {
    with_main_webview(app, native::go_forward);
}

pub fn reload<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = main_window(app)
        && let Err(err) = window.reload()
    {
        eprintln!("failed to reload the page: {err}");
    }
}

pub fn home<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = main_window(app)
        && let Err(err) = window.navigate(START_URL.parse().expect("START_URL is a valid URL"))
    {
        eprintln!("failed to open {START_URL}: {err}");
    }
}

pub fn copy_url<R: Runtime>(app: &AppHandle<R>) {
    let Some((window, url)) = lichess_page(app) else {
        return;
    };
    // The clipboard APIs (GTK in particular) need the main thread. This runs
    // at once when called there.
    let result = app.run_on_main_thread(move || native::copy_text(&window, url.as_str()));
    if let Err(err) = result {
        eprintln!("failed to copy the page address: {err}");
    }
}

pub fn open_in_browser<R: Runtime>(app: &AppHandle<R>) {
    if let Some((_, url)) = lichess_page(app) {
        crate::open_in_browser(app, &url);
    }
}

pub fn zoom_in<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = main_window(app) {
        zoom::change(&window, zoom::step_in);
    }
}

pub fn zoom_out<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = main_window(app) {
        zoom::change(&window, zoom::step_out);
    }
}

pub fn zoom_reset<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = main_window(app) {
        zoom::change(&window, |_| zoom::DEFAULT);
    }
}

pub fn toggle_fullscreen<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = main_window(app)
        && let Err(err) = window
            .is_fullscreen()
            .and_then(|fullscreen| window.set_fullscreen(!fullscreen))
    {
        eprintln!("failed to toggle fullscreen: {err}");
    }
}

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window(MAIN_WINDOW)
}

/// The main window and its address, if it shows a lichess page. The offline
/// page has no address worth copying or opening.
fn lichess_page<R: Runtime>(app: &AppHandle<R>) -> Option<(WebviewWindow<R>, Url)> {
    let window = main_window(app)?;
    let url = window.url().ok()?;
    navigation::is_lichess(&url).then_some((window, url))
}

/// Runs `f` with the platform webview of the main window, on the main thread.
fn with_main_webview<R: Runtime>(
    app: &AppHandle<R>,
    f: impl FnOnce(tauri::webview::PlatformWebview) + Send + 'static,
) {
    if let Some(window) = main_window(app)
        && let Err(err) = window.with_webview(f)
    {
        eprintln!("failed to reach the webview: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = Action::ALL.iter().map(|action| action.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Action::ALL.len());
    }
}
