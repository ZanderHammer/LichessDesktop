//! macOS: the app menu carries the shortcuts (ADR 0010).
//!
//! macOS always shows a menu bar, and AppKit resolves menu key equivalents
//! before the webview gets the key. The app keeps Tauri's default menu (app,
//! File, Edit, View, Window, Help) and adds a Navigation submenu with an item
//! per [`Action`]; the item's accelerator is the action's macOS row of
//! [`super::SHORTCUTS`].

use tauri::menu::{IsMenuItem, Menu, MenuItem, Submenu, WINDOW_SUBMENU_ID};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

use super::{Key, Mods};
use crate::actions::Action;
use crate::i18n::{self, Key as Text};

pub fn init<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let app = window.app_handle();
    app.set_menu(menu(app)?)?;
    // Called for every menu event, the tray's included; those ids don't match.
    app.on_menu_event(|app, event| {
        if let Some(action) = Action::ALL
            .iter()
            .find(|action| action.id() == event.id.as_ref())
        {
            action.run(app);
        }
    });
    Ok(())
}

/// Rebuilds the menu after a language change.
pub fn refresh_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.set_menu(menu(app)?)?;
    Ok(())
}

/// Tauri's default menu with the Navigation submenu before Window.
fn menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let strings = app.state::<i18n::Current>().get();
    let items = Action::ALL
        .iter()
        .map(|&action| {
            MenuItem::with_id(
                app,
                action.id(),
                strings.get(label(action)),
                true,
                accelerator(action).as_deref(),
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let items: Vec<&dyn IsMenuItem<R>> = items.iter().map(|item| item as _).collect();
    let navigation = Submenu::with_items(app, strings.get(Text::Navigation), true, &items)?;

    let menu = Menu::default(app)?;
    let items = menu.items()?;
    let position = items
        .iter()
        .position(|item| item.id().as_ref() == WINDOW_SUBMENU_ID)
        .unwrap_or(items.len());
    menu.insert(&navigation, position)?;
    Ok(menu)
}

fn label(action: Action) -> Text {
    match action {
        Action::Back => Text::Back,
        Action::Forward => Text::Forward,
        Action::Reload => Text::Reload,
        Action::Home => Text::Home,
        Action::CopyUrl => Text::CopyUrl,
        Action::OpenInBrowser => Text::OpenInBrowser,
        Action::ZoomIn => Text::ZoomIn,
        Action::ZoomOut => Text::ZoomOut,
        Action::ZoomReset => Text::ZoomReset,
    }
}

/// The action's macOS shortcut in muda's accelerator syntax.
fn accelerator(action: Action) -> Option<String> {
    let shortcut = super::current().find(|shortcut| shortcut.action == action)?;
    let Mods {
        primary,
        shift,
        alt,
    } = shortcut.mods;
    let mut parts = Vec::new();
    if primary {
        parts.push("CmdOrCtrl".to_owned());
    }
    if shift {
        parts.push("Shift".to_owned());
    }
    if alt {
        parts.push("Alt".to_owned());
    }
    parts.push(match shortcut.key {
        Key::Left => "ArrowLeft".to_owned(),
        Key::Right => "ArrowRight".to_owned(),
        Key::Home => "Home".to_owned(),
        Key::F5 => "F5".to_owned(),
        Key::BracketLeft => "[".to_owned(),
        Key::BracketRight => "]".to_owned(),
        Key::Letter(c) => c.to_ascii_uppercase().to_string(),
        Key::Equal => "=".to_owned(),
        Key::Plus => "+".to_owned(),
        Key::Minus => "-".to_owned(),
        Key::Digit0 => "0".to_owned(),
        Key::KpAdd => "NumPadAdd".to_owned(),
        Key::KpSubtract => "NumPadSubtract".to_owned(),
        Key::Kp0 => "NumPad0".to_owned(),
    });
    Some(parts.join("+"))
}
