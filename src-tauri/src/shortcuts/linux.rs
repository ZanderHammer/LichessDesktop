//! Linux: a `gtk::AccelGroup` on the main window (ADR 0010).
//!
//! muda's accelerators live on menu items and stop firing once the menu bar
//! is hidden, so the app sets no menu and adds closure accelerators to the
//! window instead. `GtkWindow` activates them before it passes the key on to
//! the focused widget, so the webview never sees these keys.

use gtk::gdk::{self, ModifierType};
use gtk::prelude::*;
use gtk::{AccelFlags, AccelGroup};
use tauri::{Manager, Runtime, WebviewWindow};

use super::{Key, Mods};

pub fn init<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let group = AccelGroup::new();
    for shortcut in super::current() {
        let app = window.app_handle().clone();
        let action = shortcut.action;
        group.connect_accel_group(
            *keyval(shortcut.key),
            modifiers(shortcut.mods),
            AccelFlags::VISIBLE,
            move |_, _, _, _| {
                action.run(&app);
                true
            },
        );
    }
    window.gtk_window()?.add_accel_group(&group);
    Ok(())
}

fn keyval(key: Key) -> gdk::keys::Key {
    use gdk::keys::constants;
    match key {
        Key::Left => constants::Left,
        Key::Right => constants::Right,
        Key::Home => constants::Home,
        Key::F5 => constants::F5,
        Key::BracketLeft => constants::bracketleft,
        Key::BracketRight => constants::bracketright,
        Key::Letter(c) => gdk::keys::Key::from_unicode(c),
    }
}

fn modifiers(mods: Mods) -> ModifierType {
    let mut out = ModifierType::empty();
    if mods.primary {
        out |= ModifierType::CONTROL_MASK;
    }
    if mods.shift {
        out |= ModifierType::SHIFT_MASK;
    }
    if mods.alt {
        out |= ModifierType::MOD1_MASK;
    }
    out
}
