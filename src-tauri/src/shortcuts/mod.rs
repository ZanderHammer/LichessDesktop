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
use crate::i18n;

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
    F1,
    F5,
    F11,
    Slash,
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
        // lichess binds plain `/` (search), but no modifier with it, nor F1.
        // `/` needs Shift on some layouts (German: Shift+7), so F1 is the
        // layout-independent one.
        row(Mods::PRIMARY, K::Slash, A::ShowShortcuts, All),
        row(Mods::NONE, K::F1, A::ShowShortcuts, NotMacOs),
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

/// The rows of [`SHORTCUTS`] for macOS or for Linux and Windows.
pub fn for_platform(macos: bool) -> impl Iterator<Item = &'static Shortcut> {
    SHORTCUTS
        .iter()
        .filter(move |shortcut| shortcut.platforms.includes(macos))
}

/// The key caps of `shortcut` for the shortcuts overview, in the order the
/// platform prints them: Ctrl, Alt, Shift on Linux and Windows, and ⌥ ⇧ ⌘
/// on macOS, then the key.
pub fn key_caps(shortcut: &Shortcut, macos: bool) -> Vec<String> {
    let Mods {
        primary,
        shift,
        alt,
    } = shortcut.mods;
    let mods: [(bool, &str); 3] = if macos {
        [(alt, "⌥"), (shift, "⇧"), (primary, "⌘")]
    } else {
        [(primary, "Ctrl"), (alt, "Alt"), (shift, "Shift")]
    };
    let mut caps: Vec<String> = mods
        .into_iter()
        .filter(|(held, _)| *held)
        .map(|(_, cap)| cap.to_owned())
        .collect();
    caps.push(match shortcut.key {
        Key::Left => "←".to_owned(),
        Key::Right => "→".to_owned(),
        Key::Home => "Home".to_owned(),
        Key::F1 => "F1".to_owned(),
        Key::F5 => "F5".to_owned(),
        Key::F11 => "F11".to_owned(),
        Key::Slash => "/".to_owned(),
        Key::BracketLeft => "[".to_owned(),
        Key::BracketRight => "]".to_owned(),
        Key::Letter(c) => c.to_ascii_uppercase().to_string(),
        Key::Equal => "=".to_owned(),
        Key::Plus => "+".to_owned(),
        Key::Minus => "-".to_owned(),
        Key::Digit0 => "0".to_owned(),
        Key::KpAdd => "Num +".to_owned(),
        Key::KpSubtract => "Num -".to_owned(),
        Key::Kp0 => "Num 0".to_owned(),
    });
    caps
}

/// The key caps of the built-in macOS menu item that serves `action`
/// ([`MACOS_BUILT_IN`]), for the overview.
#[cfg(any(target_os = "macos", test))]
pub fn macos_built_in_key_caps(action: Action) -> Option<Vec<String>> {
    match action {
        // AppKit's View > Enter Full Screen.
        Action::Fullscreen => Some(vec!["⌃".to_owned(), "⌘".to_owned(), "F".to_owned()]),
        _ => None,
    }
}

/// The mouse button that also runs `action`, for the overview. The side
/// buttons go back and forward on Linux (`linux.rs`) and Windows (WebView2
/// itself); macOS isn't handled.
pub fn mouse_button(action: Action) -> Option<i18n::Key> {
    if cfg!(target_os = "macos") {
        return None;
    }
    match action {
        Action::Back => Some(i18n::Key::MouseBack),
        Action::Forward => Some(i18n::Key::MouseForward),
        _ => None,
    }
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
    fn macos_built_in_actions_have_key_caps() {
        for action in MACOS_BUILT_IN {
            assert!(
                macos_built_in_key_caps(*action).is_some(),
                "{action:?} has no key caps"
            );
        }
    }

    #[test]
    fn prints_key_caps_in_platform_order() {
        let copy = row(
            Mods::PRIMARY_SHIFT,
            Key::Letter('c'),
            Action::CopyUrl,
            Platforms::All,
        );
        assert_eq!(key_caps(&copy, false), ["Ctrl", "Shift", "C"]);
        assert_eq!(key_caps(&copy, true), ["⇧", "⌘", "C"]);
        let back = row(Mods::ALT, Key::Left, Action::Back, Platforms::NotMacOs);
        assert_eq!(key_caps(&back, false), ["Alt", "←"]);
        let reload = row(Mods::NONE, Key::F5, Action::Reload, Platforms::NotMacOs);
        assert_eq!(key_caps(&reload, false), ["F5"]);
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
