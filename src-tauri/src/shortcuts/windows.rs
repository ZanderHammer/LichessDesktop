//! Windows: WebView2's `AcceleratorKeyPressed` event (ADR 0010).
//!
//! Menu accelerators never fire while WebView2 has focus: its input window
//! belongs to the browser process, so Tauri's `TranslateAcceleratorW` never
//! sees the keys (wry#451). `AcceleratorKeyPressed` fires for every key
//! pressed with Ctrl or Alt and for keys like F5, before web content and
//! before the browser's own accelerators. Marking a key handled stops it
//! there, so the page never sees it.

use tauri::{Manager, Runtime, WebviewWindow};
use webview2_com::AcceleratorKeyPressedEventHandler;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_KEY_EVENT_KIND, COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN,
    COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN, COREWEBVIEW2_PHYSICAL_KEY_STATUS,
    ICoreWebView2AcceleratorKeyPressedEventArgs,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VIRTUAL_KEY, VK_ADD, VK_CONTROL, VK_F5, VK_F11, VK_HOME, VK_LEFT, VK_MENU,
    VK_NUMPAD0, VK_OEM_4, VK_OEM_6, VK_OEM_MINUS, VK_OEM_PLUS, VK_RIGHT, VK_SHIFT, VK_SUBTRACT,
};

use super::{Key, Mods};

pub fn init<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let app = window.app_handle().clone();
    window.with_webview(move |webview| {
        let handler =
            AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| match args {
                Some(args) => on_key(&app, &args),
                None => Ok(()),
            }));
        let mut token = 0;
        // SAFETY: `with_webview` runs this on the main thread, which owns the
        // controller. The handler stays registered for the webview's life.
        let result = unsafe {
            webview
                .controller()
                .add_AcceleratorKeyPressed(&handler, &mut token)
        };
        if let Err(err) = result {
            eprintln!("failed to set up the keyboard shortcuts: {err}");
        }
    })
}

/// Marks a key-down that matches a shortcut as handled and runs its action.
/// The browser process waits until this returns, so the action runs on
/// another thread. Auto-repeats are swallowed but don't repeat the action.
fn on_key<R: Runtime>(
    app: &tauri::AppHandle<R>,
    args: &ICoreWebView2AcceleratorKeyPressedEventArgs,
) -> windows::core::Result<()> {
    let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
    let mut virtual_key = 0;
    let mut status = COREWEBVIEW2_PHYSICAL_KEY_STATUS::default();
    // SAFETY: `args` is valid for the duration of the event.
    unsafe {
        args.KeyEventKind(&mut kind)?;
        args.VirtualKey(&mut virtual_key)?;
        args.PhysicalKeyStatus(&mut status)?;
    }
    if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN
        && kind != COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN
    {
        return Ok(());
    }
    let mods = pressed_mods();
    let Some(shortcut) = super::current()
        .find(|shortcut| shortcut.mods == mods && u32::from(code(shortcut.key).0) == virtual_key)
    else {
        return Ok(());
    };
    // SAFETY: as above.
    unsafe { args.SetHandled(true)? };
    if !status.WasKeyDown.as_bool() {
        let app = app.clone();
        let action = shortcut.action;
        tauri::async_runtime::spawn_blocking(move || action.run(&app));
    }
    Ok(())
}

/// The modifiers held with the key. Microsoft's WebView2 sample
/// (`AppWindow::GetAcceleratorKeyFunction`) reads them with `GetKeyState`
/// in this handler too.
fn pressed_mods() -> Mods {
    // SAFETY: plain Win32 call without pointers.
    let is_down = |key: VIRTUAL_KEY| unsafe { GetKeyState(i32::from(key.0)) } < 0;
    Mods {
        primary: is_down(VK_CONTROL),
        shift: is_down(VK_SHIFT),
        alt: is_down(VK_MENU),
    }
}

fn code(key: Key) -> VIRTUAL_KEY {
    match key {
        Key::Left => VK_LEFT,
        Key::Right => VK_RIGHT,
        Key::Home => VK_HOME,
        Key::F5 => VK_F5,
        Key::F11 => VK_F11,
        Key::BracketLeft => VK_OEM_4,
        Key::BracketRight => VK_OEM_6,
        // The virtual-key codes of A–Z are their ASCII capitals.
        Key::Letter(c) => VIRTUAL_KEY(u16::from(c.to_ascii_uppercase() as u8)),
        // The `=`/`+` key has one virtual-key code on Windows.
        Key::Equal | Key::Plus => VK_OEM_PLUS,
        Key::Minus => VK_OEM_MINUS,
        Key::Digit0 => VIRTUAL_KEY(u16::from(b'0')),
        Key::KpAdd => VK_ADD,
        Key::KpSubtract => VK_SUBTRACT,
        Key::Kp0 => VK_NUMPAD0,
    }
}
