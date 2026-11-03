//! WKWebView and the general pasteboard.

use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;
use objc2_web_kit::WKWebView;
use tauri::webview::PlatformWebview;
use tauri::{Runtime, WebviewWindow};

/// The `WKWebView` behind `webview`.
///
/// # Safety
///
/// `PlatformWebview::inner` is the app's `WKWebView`, alive while
/// `with_webview` runs the caller on the main thread.
unsafe fn wk_webview(webview: &PlatformWebview) -> &WKWebView {
    unsafe { &*webview.inner().cast::<WKWebView>() }
}

pub fn go_back(webview: PlatformWebview) {
    // SAFETY: see `wk_webview`. `goBack` returns nil without a back item.
    unsafe {
        let _ = wk_webview(&webview).goBack();
    }
}

pub fn go_forward(webview: PlatformWebview) {
    // SAFETY: see `wk_webview`.
    unsafe {
        let _ = wk_webview(&webview).goForward();
    }
}

pub fn copy_text<R: Runtime>(_window: &WebviewWindow<R>, text: &str) {
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    // SAFETY: an AppKit constant, valid for the life of the process.
    let string_type = unsafe { NSPasteboardTypeString };
    if !pasteboard.setString_forType(&NSString::from_str(text), string_type) {
        eprintln!("failed to copy to the pasteboard");
    }
}
