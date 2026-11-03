// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod navigation;

use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

const START_URL: &str = "https://lichess.org";
const MAIN_WINDOW: &str = "main";

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let url = START_URL.parse().expect("START_URL is a valid URL");
            let nav_handle = app.handle().clone();
            let popup_handle = app.handle().clone();

            WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::External(url))
                .title("Lichess Desktop")
                .inner_size(1280.0, 800.0)
                .min_inner_size(480.0, 360.0)
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

fn open_in_browser(app: &AppHandle, url: &Url) {
    if let Err(err) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("failed to open {url} in the system browser: {err}");
    }
}
