//! WebView2 and the Win32 clipboard.

use tauri::webview::PlatformWebview;
use tauri::{Runtime, WebviewWindow};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

pub fn go_back(webview: PlatformWebview) {
    // SAFETY: `with_webview` runs this on the main thread, which owns the
    // controller and its webview.
    let result = unsafe { webview.controller().CoreWebView2().and_then(|w| w.GoBack()) };
    if let Err(err) = result {
        eprintln!("failed to go back: {err}");
    }
}

pub fn go_forward(webview: PlatformWebview) {
    // SAFETY: as in `go_back`.
    let result = unsafe {
        webview
            .controller()
            .CoreWebView2()
            .and_then(|w| w.GoForward())
    };
    if let Err(err) = result {
        eprintln!("failed to go forward: {err}");
    }
}

/// Puts `text` on the clipboard as `CF_UNICODETEXT`.
pub fn copy_text<R: Runtime>(window: &WebviewWindow<R>, text: &str) {
    let result = window
        .hwnd()
        .map_err(|err| err.to_string())
        .and_then(|hwnd| set_clipboard_text(hwnd, text).map_err(|err| err.to_string()));
    if let Err(err) = result {
        eprintln!("failed to copy to the clipboard: {err}");
    }
}

/// The clipboard must be opened with an owner window: after
/// `OpenClipboard(NULL)`, `EmptyClipboard` leaves no owner and
/// `SetClipboardData` fails.
fn set_clipboard_text(owner: HWND, text: &str) -> windows::core::Result<()> {
    let wide: Vec<u16> = text.encode_utf16().chain([0]).collect();
    // SAFETY: the clipboard is closed again on every path. The memory is
    // allocated with the size of `wide` and locked while it is written; once
    // `SetClipboardData` succeeds the system owns it, otherwise it is freed.
    unsafe {
        OpenClipboard(Some(owner))?;
        let result = (|| {
            EmptyClipboard()?;
            let memory = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of_val(wide.as_slice()))?;
            let target = GlobalLock(memory).cast::<u16>();
            if target.is_null() {
                let _ = GlobalFree(Some(memory));
                return Err(windows::core::Error::from_thread());
            }
            target.copy_from_nonoverlapping(wide.as_ptr(), wide.len());
            // Reports an "error" with code 0 once the lock count is zero.
            let _ = GlobalUnlock(memory);
            if let Err(err) = SetClipboardData(u32::from(CF_UNICODETEXT.0), Some(HANDLE(memory.0)))
            {
                let _ = GlobalFree(Some(memory));
                return Err(err);
            }
            Ok(())
        })();
        let _ = CloseClipboard();
        result
    }
}
