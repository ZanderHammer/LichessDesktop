// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(test)]
mod capabilities;
mod deep_link;
mod navigation;
mod offline;
mod settings;
#[cfg(desktop)]
mod tray;

use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Runtime, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_window_state::StateFlags;

const START_URL: &str = "https://lichess.org";
const MAIN_WINDOW: &str = "main";

fn main() {
    // Runs while Tauri starts up, so a reachable lichess.org costs no extra time.
    let probe = offline::spawn_probe();

    let mut builder = tauri::Builder::default();
    // Registered first, as the plugin docs require, so a second launch exits
    // before other plugins run. Its `deep-link` feature has already passed a
    // deep link in `_args` on to the deep-link plugin when this runs.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_main_window(app);
        }));
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            // Visibility is not restored: quitting from the tray while the
            // window is hidden must not start the next launch hidden.
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() - StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .register_uri_scheme_protocol(offline::SCHEME, |_ctx, _request| offline::response())
        .setup(move |app| {
            settings::init(app)?;
            #[cfg(desktop)]
            tray::init(app.handle())?;
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            register_deep_links(app.handle());
            // Set on Linux and Windows when this launch came from a deep link.
            // macOS delivers it later, through `on_open_url`.
            let start_link = app
                .deep_link()
                .get_current()?
                .and_then(|links| deep_link::target(&links));
            let url = if offline::wait_for_probe(&probe) {
                WebviewUrl::External(
                    start_link
                        .unwrap_or_else(|| START_URL.parse().expect("START_URL is a valid URL")),
                )
            } else {
                WebviewUrl::CustomProtocol(offline::page_url())
            };
            let nav_handle = app.handle().clone();
            let popup_handle = app.handle().clone();

            let window = WebviewWindowBuilder::new(app, MAIN_WINDOW, url)
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

            let link_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                let Some(url) = deep_link::target(&event.urls()) else {
                    return;
                };
                if let Some(window) = link_handle.get_webview_window(MAIN_WINDOW)
                    && let Err(err) = window.navigate(url)
                {
                    eprintln!("failed to open deep link: {err}");
                }
                #[cfg(desktop)]
                focus_main_window(&link_handle);
            });
            window.show()?;
            window.set_focus()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Lichess Desktop");
}

/// Brings the main window to the front: unminimizes, shows and focuses it.
#[cfg(desktop)]
fn focus_main_window<R: Runtime>(app: &AppHandle<R>) {
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

/// Makes this executable the handler for the deep link scheme where no
/// installer did: an AppImage (which may also have moved since the last run)
/// and debug builds. Runs in the background, as it spawns `xdg-mime` on Linux.
/// macOS only supports the scheme from the bundle's Info.plist.
#[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
fn register_deep_links(app: &AppHandle) {
    #[cfg(all(target_os = "linux", not(debug_assertions)))]
    if app.env().appimage.is_none() {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(err) = app.deep_link().register_all() {
            eprintln!("failed to register the {} scheme: {err}", deep_link::SCHEME);
        }
    });
}

fn open_in_browser(app: &AppHandle, url: &Url) {
    if let Err(err) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("failed to open {url} in the system browser: {err}");
    }
}
