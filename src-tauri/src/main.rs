// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod navigation;
mod settings;

use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

const START_URL: &str = "https://lichess.org";
const MAIN_WINDOW: &str = "main";

fn main() {
    let mut builder = tauri::Builder::default();
    // Registered first, as the plugin docs require, so a second launch exits
    // before other plugins run. Its launch arguments arrive in `_args`.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_main_window(app);
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            settings::init(app)?;
            let url = START_URL.parse().expect("START_URL is a valid URL");
            let nav_handle = app.handle().clone();
            let popup_handle = app.handle().clone();

            WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::External(url))
                .title("Lichess Desktop")
                .inner_size(1280.0, 800.0)
                .min_inner_size(480.0, 360.0)
                // Hidden until the window-state plugin restores size and position.
                .visible(false)
                .on_navigation(move |url| {
                    if navigation::is_internal(url) {
                        return true;
                    }
                    open_in_browser(&nav_handle, url);
                    false
                })
                .on_new_window(move |url, _features| {
                    if navigation::is_internal(&url) {
                        // Keep a single window: show lichess "new tab" links in place.
                        if let Some(window) = popup_handle.get_webview_window(MAIN_WINDOW) {
                            let _ = window.navigate(url);
                        }
                    } else {
                        open_in_browser(&popup_handle, &url);
                    }
                    NewWindowResponse::Deny
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Lichess Desktop");
}

/// Brings the main window to the front: unminimizes, shows and focuses it.
#[cfg(desktop)]
fn focus_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if let Err(err) = window
        .unminimize()
        .and_then(|()| window.show())
        .and_then(|()| window.set_focus())
    {
        eprintln!("failed to focus the main window: {err}");
    }
}

fn open_in_browser(app: &AppHandle, url: &Url) {
    if let Err(err) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("failed to open {url} in the system browser: {err}");
    }
}
