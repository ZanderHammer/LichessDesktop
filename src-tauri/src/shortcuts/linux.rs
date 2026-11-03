//! Linux: a `gtk::AccelGroup` on the main window (ADR 0010).
//!
//! muda's accelerators live on menu items and stop firing once the menu bar
//! is hidden, so the app sets no menu and adds closure accelerators to the
//! window instead. `GtkWindow` activates them before it passes the key on to
//! the focused widget, so the webview never sees these keys.
//!
//! The mouse back/forward buttons are handled here too. WebKitGTK ignores
//! them, and wry fills the gap by running JavaScript in the page that
//! dispatches synthetic mouse events and calls `history.back()`. That is code
//! injection into lichess.org (ADR 0008), so [`init_mouse_buttons`] takes the
//! buttons first.

use gtk::gdk::{self, EventType, ModifierType};
use gtk::glib::Propagation;
use gtk::prelude::*;
use gtk::{AccelFlags, AccelGroup};
use tauri::{Manager, Runtime, WebviewWindow};

use super::{Key, Mods};
use crate::actions::Action;

/// GDK's numbers for the side buttons of a mouse (X11 buttons 8 and 9).
const MOUSE_BACK: u32 = 8;
const MOUSE_FORWARD: u32 = 9;

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
    init_mouse_buttons(window)
}

/// Runs back/forward for the mouse side buttons. The webview's `event`
/// signal comes before `button-press-event` and `button-release-event`, where
/// wry's handlers are, and stopping both press and release keeps them from
/// running at all.
fn init_mouse_buttons<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let app = window.app_handle().clone();
    window.with_webview(move |webview| {
        webview.inner().connect_event(move |_, event| {
            let action = match event.button() {
                Some(MOUSE_BACK) => Action::Back,
                Some(MOUSE_FORWARD) => Action::Forward,
                _ => return Propagation::Proceed,
            };
            if event.event_type() == EventType::ButtonPress {
                action.run(&app);
            }
            Propagation::Stop
        });
    })
}

fn keyval(key: Key) -> gdk::keys::Key {
    use gdk::keys::constants;
    match key {
        Key::Left => constants::Left,
        Key::Right => constants::Right,
        Key::Home => constants::Home,
        Key::F1 => constants::F1,
        Key::F5 => constants::F5,
        Key::F11 => constants::F11,
        Key::Slash => constants::slash,
        Key::Comma => constants::comma,
        Key::BracketLeft => constants::bracketleft,
        Key::BracketRight => constants::bracketright,
        Key::Letter(c) => gdk::keys::Key::from_unicode(c),
        Key::Equal => constants::equal,
        Key::Plus => constants::plus,
        Key::Minus => constants::minus,
        Key::Digit0 => constants::_0,
        Key::KpAdd => constants::KP_Add,
        Key::KpSubtract => constants::KP_Subtract,
        Key::Kp0 => constants::KP_0,
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
