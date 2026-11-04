// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
#[cfg(test)]
mod capabilities;
mod deep_link;
mod i18n;
mod native;
mod navigation;
#[cfg(desktop)]
mod notifications;
mod offline;
#[cfg(desktop)]
mod palette;
#[cfg(test)]
mod pinned_deps;
mod plugins;
mod settings;
#[cfg(desktop)]
mod settings_window;
mod shortcuts;
#[cfg(desktop)]
mod tray;
#[cfg(desktop)]
mod turn;
#[cfg(test)]
mod ui_assets;
#[cfg(desktop)]
mod update;
mod zoom;

use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Runtime, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry};
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
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                focus_main_window(app);
            }))
            .invoke_handler(invoke_handler());
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(plugins::notification())
        .plugin(plugins::opener())
        .plugin(
            // Visibility and fullscreen are not restored: quitting from the
            // tray while hidden must not start hidden, and fullscreen is
            // session-only.
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() - StateFlags::VISIBLE - StateFlags::FULLSCREEN)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .register_uri_scheme_protocol(offline::SCHEME, |ctx, _request| {
            offline::response(&ctx.app_handle().state::<i18n::Current>().get())
        })
        .setup(move |app| {
            settings::init(app)?;
            let language = app.state::<settings::SettingsStore<Wry>>().get().language;
            app.manage(i18n::Current::new(i18n::Strings::load(i18n::resolve(
                &language,
                sys_locale::get_locales(),
            ))));
            // Without a tray (e.g. GNOME without a StatusNotifierWatcher) the
            // app still runs; close then always quits.
            #[cfg(desktop)]
            if let Err(err) = tray::init(app.handle()) {
                eprintln!("failed to create the tray icon: {err}");
            }
            // After the tray, so a quick result can update its menu.
            #[cfg(desktop)]
            update::init(app.handle());
            #[cfg(desktop)]
            notifications::init(app.handle());
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

            let builder = WebviewWindowBuilder::new(app, MAIN_WINDOW, url)
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
                });
            #[cfg(desktop)]
            let builder = builder.on_document_title_changed(notifications::title_changed);
            let window = builder.build()?;
            #[cfg(desktop)]
            notifications::watch_focus(&window);
            if let Err(err) = zoom::apply_saved(&window) {
                eprintln!("failed to apply the saved zoom level: {err}");
            }
            // Without them the app still works, through the page and the tray.
            if let Err(err) = shortcuts::init(&window) {
                eprintln!("failed to set up the keyboard shortcuts: {err}");
            }

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
            #[cfg(desktop)]
            let close_handle = app.handle().clone();
            #[cfg(desktop)]
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event
                    && tray::is_shown(&close_handle)
                    && close_handle
                        .state::<settings::SettingsStore<Wry>>()
                        .get()
                        .close_to_tray
                {
                    api.prevent_close();
                    hide_main_window(&close_handle);
                }
            });
            window.show()?;
            window.set_focus()?;
            Ok(())
        })
        .build(context())
        .expect("error while building Lichess Desktop")
        .run(|_app, _event| {
            // macOS: a Dock click brings back a window hidden to the tray.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } = _event
            {
                focus_main_window(_app);
            }
        });
}

/// The app context. `generate_context!` may only expand once per binary: on
/// macOS it embeds the Info.plist under a fixed symbol, so tests call this too.
fn context<R: Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

/// The handler for every app command. Keep it in step with `COMMANDS` in
/// `build.rs` and with the capabilities in `capabilities/` (ADR 0012).
#[cfg(desktop)]
fn invoke_handler<R: Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        settings_window::get_strings,
        settings_window::get_settings,
        settings_window::set_setting,
        settings_window::get_app_info,
        settings_window::get_shortcuts,
        settings_window::settings_ready,
        settings_window::close_settings,
        palette::list_actions,
        palette::run_action,
        palette::palette_ready,
        palette::close_palette,
    ]
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

/// Brings the main window to the front after a click outside it. KWin refuses a
/// focus request without an activation token and only marks the window in the
/// taskbar. The token of a tray or notification click doesn't reach us through
/// Tauri and its plugins, but a window shown anew does come to the front.
/// Call it on the main thread, as it asks GTK for the display.
#[cfg(desktop)]
fn raise_main_window<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "linux")]
    if let Some(window) = app.get_webview_window(MAIN_WINDOW)
        && window.is_visible().unwrap_or(false)
        && !window.is_focused().unwrap_or(false)
        && is_wayland()
    {
        hide_main_window(app);
    }
    focus_main_window(app);
}

#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    use gtk::gdk::prelude::DisplayExtManual;
    gtk::gdk::Display::default().is_some_and(|display| display.backend().is_wayland())
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

/// Hides the main window. The tray icon brings it back.
fn hide_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if let Err(err) = window.hide() {
        eprintln!("failed to hide the main window: {err}");
    }
}

fn open_in_browser<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    if let Err(err) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("failed to open {url} in the system browser: {err}");
    }
}
