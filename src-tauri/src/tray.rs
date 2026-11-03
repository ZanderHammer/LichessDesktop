//! The tray icon, which is always shown: on Linux and Windows there is no
//! menu bar, so it is the only mouse path to app actions.
//!
//! A left click toggles the main window on Windows and macOS. Linux (libappindicator)
//! emits no click events and opens the menu on any click, so the menu
//! repeats the toggle as "Show/Hide".

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

const TRAY_ID: &str = "main";
const TOGGLE: &str = "toggle";
const QUIT: &str = "quit";

/// Creates the tray icon. The app keeps it alive until exit.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, TOGGLE, "Show/Hide", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, QUIT, "Quit", true, None::<&str>)?,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Lichess Desktop")
        .menu(&menu)
        // Left click toggles the window; the menu stays on right click.
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Whether the tray icon exists. Without it a hidden window could only come
/// back through a second launch.
pub fn is_shown<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.tray_by_id(TRAY_ID).is_some()
}

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id.as_ref() {
        TOGGLE => toggle_main_window(app),
        QUIT => app.exit(0),
        _ => {}
    }
}

fn on_tray_icon_event<R: Runtime>(tray: &TrayIcon<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        toggle_main_window(tray.app_handle());
    }
}

/// Hides the main window if it is on screen, otherwise brings it to the front.
fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(crate::MAIN_WINDOW) else {
        return;
    };
    let on_screen = window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false);
    if on_screen {
        crate::hide_main_window(app);
    } else {
        crate::focus_main_window(app);
    }
}
