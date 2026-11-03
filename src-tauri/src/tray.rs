//! The tray icon, which is always shown: on Linux and Windows there is no
//! menu bar, so it is the only mouse path to app actions.
//!
//! A left click toggles the main window and a right click opens the menu. On
//! Linux the icon is a StatusNotifierItem from tray-icon's ksni backend
//! (ADR 0014). Panels report a left click as `Activate`, which arrives here as a click.
//! The menu repeats the toggle as "Show/Hide" for panels that only show menus.

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::actions::Action;
use crate::i18n::{self, Key};
use crate::update;

const TRAY_ID: &str = "main";
const TOGGLE: &str = "toggle";
const PALETTE: &str = "palette";
const SETTINGS: &str = "settings";
const SHORTCUTS: &str = "shortcuts";
const QUIT: &str = "quit";
const UPDATE: &str = "update";

/// Creates the tray icon. The app keeps it alive until exit.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = menu(app)?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Lichess Desktop")
        .menu(&menu)
        // Left click toggles the window; the menu stays on right click.
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event);
    // On Linux the title names the item in the panel and heads its tooltip.
    // On macOS it would be drawn as text next to the icon.
    #[cfg(target_os = "linux")]
    {
        tray = tray.title("Lichess Desktop");
    }
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// The tray menu in the current UI language. A known update comes first.
fn menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let strings = app.state::<i18n::Current>().get();
    let item =
        |id: &str, key: Key| MenuItem::with_id(app, id, strings.get(key), true, None::<&str>);
    let menu = Menu::with_items(
        app,
        &[
            &item(TOGGLE, Key::ShowHide)?,
            &item(PALETTE, Action::CommandPalette.label())?,
            &item(SETTINGS, Action::OpenSettings.label())?,
            &item(SHORTCUTS, Action::ShowShortcuts.label())?,
            &PredefinedMenuItem::separator(app)?,
            &item(QUIT, Action::Quit.label())?,
        ],
    )?;
    if let Some(update) = update::available(app) {
        let text = update.text(&strings);
        menu.prepend_items(&[
            &MenuItem::with_id(app, UPDATE, text, true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
        ])?;
    }
    Ok(menu)
}

/// Rebuilds the menu after a language change or a new update.
pub fn refresh_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    match app.tray_by_id(TRAY_ID) {
        Some(tray) => tray.set_menu(Some(menu(app)?)),
        None => Ok(()),
    }
}

/// Whether the tray icon exists. Without it a hidden window could only come
/// back through a second launch.
pub fn is_shown<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.tray_by_id(TRAY_ID).is_some()
}

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id.as_ref() {
        TOGGLE => toggle_main_window(app),
        PALETTE => Action::CommandPalette.run(app),
        SETTINGS => Action::OpenSettings.run(app),
        SHORTCUTS => Action::ShowShortcuts.run(app),
        QUIT => Action::Quit.run(app),
        UPDATE => update::open_release_page(app),
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
