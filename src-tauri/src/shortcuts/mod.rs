//! Keyboard shortcuts of the main window (ADR 0010).
//!
//! [`SHORTCUTS`] is the only place a shortcut is defined. [`init`] turns the
//! rows for the current platform into native handlers:
//!
//! * Linux: closures on a `gtk::AccelGroup` of the window (`linux.rs`).
//! * Windows: WebView2's `AcceleratorKeyPressed` event (`windows.rs`).
//! * macOS: app menu items with muda accelerators (`macos.rs`).
//!
//! Each runs before the page gets the key, so shortcuts work while a lichess
//! text field has focus and the page never sees them.
//!
//! To add a shortcut, add a row to [`SHORTCUTS`]. A new [`Key`] needs its
//! name on every platform; the `match` in each platform module won't compile
//! until it has one.

use tauri::{Runtime, WebviewWindow};

use crate::actions::Action;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::refresh_menu;
#[cfg(windows)]
mod windows;

/// Modifier keys. `primary` is Ctrl on Linux and Windows and Cmd on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mods {
    pub primary: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Mods {
    pub const NONE: Self = Self {
        primary: false,
        shift: false,
        alt: false,
    };
    pub const PRIMARY: Self = Self {
        primary: true,
        ..Self::NONE
    };
    pub const ALT: Self = Self {
        alt: true,
        ..Self::NONE
    };
    pub const PRIMARY_SHIFT: Self = Self {
        primary: true,
        shift: true,
        alt: false,
    };
}

/// A key on the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Home,
    F5,
    F11,
    BracketLeft,
    BracketRight,
    /// A letter key, given as an ASCII lowercase letter.
    Letter(char),
    Equal,
    Plus,
    Minus,
    Digit0,
    KpAdd,
    KpSubtract,
    Kp0,
}

/// The platforms a row applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platforms {
    All,
    MacOs,
    NotMacOs,
}

impl Platforms {
    const fn includes(self, macos: bool) -> bool {
        match self {
            Platforms::All => true,
            Platforms::MacOs => macos,
            Platforms::NotMacOs => !macos,
        }
    }
}

/// One row of [`SHORTCUTS`]: `mods` + `key` runs `action` on `platforms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    pub mods: Mods,
    pub key: Key,
    pub action: Action,
    pub platforms: Platforms,
}

const fn row(mods: Mods, key: Key, action: Action, platforms: Platforms) -> Shortcut {
    Shortcut {
        mods,
        key,
        action,
        platforms,
    }
}

/// Every shortcut. An action may have several on Linux and Windows, but at
/// most one on macOS, where it is the accelerator of the action's menu item.
///
/// None of these collides with a lichess shortcut (lila `ui/`, checked in
/// spike #4). Before adding one, check lila again.
pub const SHORTCUTS: &[Shortcut] = {
    use Action as A;
    use Key as K;
    use Platforms::{All, MacOs, NotMacOs};
    &[
        // Browser conventions: Alt+arrows, and Cmd+[ / Cmd+] on macOS.
        row(Mods::ALT, K::Left, A::Back, NotMacOs),
        row(Mods::PRIMARY, K::BracketLeft, A::Back, MacOs),
        row(Mods::ALT, K::Right, A::Forward, NotMacOs),
        row(Mods::PRIMARY, K::BracketRight, A::Forward, MacOs),
        row(Mods::NONE, K::F5, A::Reload, NotMacOs),
        row(Mods::PRIMARY, K::Letter('r'), A::Reload, All),
        row(Mods::ALT, K::Home, A::Home, All),
        row(Mods::PRIMARY_SHIFT, K::Letter('c'), A::CopyUrl, All),
        row(Mods::PRIMARY_SHIFT, K::Letter('o'), A::OpenInBrowser, All),
        // Browser conventions: Ctrl/Cmd + =/+ zooms in, - zooms out, 0
        // resets. On layouts where `=` needs Shift the `equal` keyval still
        // matches, the `+` key needs its own row (ADR 0010, spike #4).
        row(Mods::PRIMARY, K::Equal, A::ZoomIn, All),
        row(Mods::PRIMARY, K::Plus, A::ZoomIn, NotMacOs),
        row(Mods::PRIMARY, K::KpAdd, A::ZoomIn, NotMacOs),
        row(Mods::PRIMARY, K::Minus, A::ZoomOut, All),
        row(Mods::PRIMARY, K::KpSubtract, A::ZoomOut, NotMacOs),
        row(Mods::PRIMARY, K::Digit0, A::ZoomReset, All),
        row(Mods::PRIMARY, K::Kp0, A::ZoomReset, NotMacOs),
        row(Mods::NONE, K::F11, A::Fullscreen, NotMacOs),
        row(Mods::PRIMARY_SHIFT, K::Letter('t'), A::AlwaysOnTop, All),
    ]
};

/// Actions that Tauri's default macOS menu already has, so they get no row
/// and no menu item of ours there. View > Toggle Full Screen is AppKit's
/// `toggleFullScreen:` on Ctrl+Cmd+F.
#[cfg(any(target_os = "macos", test))]
pub const MACOS_BUILT_IN: &[Action] = &[Action::Fullscreen];

/// The rows of [`SHORTCUTS`] for this platform.
pub fn current() -> impl Iterator<Item = &'static Shortcut> {
    for_platform(cfg!(target_os = "macos"))
}

fn for_platform(macos: bool) -> impl Iterator<Item = &'static Shortcut> {
    SHORTCUTS
        .iter()
        .filter(move |shortcut| shortcut.platforms.includes(macos))
}

/// Wires [`SHORTCUTS`] to the main window.
pub fn init<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    #[cfg(target_os = "linux")]
    linux::init(window)?;
    #[cfg(windows)]
    self::windows::init(window)?;
    #[cfg(target_os = "macos")]
    macos::init(window)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_key_is_bound_twice_on_a_platform() {
        for macos in [false, true] {
            let rows: Vec<_> = for_platform(macos).collect();
            for (i, a) in rows.iter().enumerate() {
                for b in &rows[i + 1..] {
                    assert!(
                        (a.mods, a.key) != (b.mods, b.key),
                        "{:?} and {:?} share a key (macOS: {macos})",
                        a.action,
                        b.action
                    );
                }
            }
        }
    }

    #[test]
    fn every_action_has_at_most_one_shortcut_on_macos() {
        for action in Action::ALL {
            let count = for_platform(true)
                .filter(|shortcut| shortcut.action == *action)
                .count();
            assert!(count <= 1, "{action:?} has {count} macOS shortcuts");
        }
    }

    #[test]
    fn macos_built_in_actions_have_no_macos_row() {
        for shortcut in for_platform(true) {
            assert!(
                !MACOS_BUILT_IN.contains(&shortcut.action),
                "{shortcut:?} duplicates a macOS menu item"
            );
        }
    }

    #[test]
    fn letters_are_ascii_lowercase() {
        for shortcut in SHORTCUTS {
            if let Key::Letter(c) = shortcut.key {
                assert!(c.is_ascii_lowercase(), "{c:?} in {shortcut:?}");
            }
        }
    }

    #[test]
    fn every_action_has_a_shortcut_on_every_platform() {
        for macos in [false, true] {
            for action in Action::ALL {
                if macos && MACOS_BUILT_IN.contains(action) {
                    continue;
                }
                assert!(
                    for_platform(macos).any(|shortcut| shortcut.action == *action),
                    "{action:?} has no shortcut (macOS: {macos})"
                );
            }
        }
    }
}
