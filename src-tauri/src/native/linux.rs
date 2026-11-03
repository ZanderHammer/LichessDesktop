//! WebKitGTK and GTK.

use gtk::gdk;
use tauri::webview::PlatformWebview;
use tauri::{Runtime, WebviewWindow};
use webkit2gtk::WebViewExt;

pub fn go_back(webview: PlatformWebview) {
    webview.inner().go_back();
}

pub fn go_forward(webview: PlatformWebview) {
    webview.inner().go_forward();
}

/// Puts `text` on the CLIPBOARD selection. GTK serves it to other apps while
/// the app runs, on X11 and Wayland alike.
pub fn copy_text<R: Runtime>(_window: &WebviewWindow<R>, text: &str) {
    gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(text);
}
