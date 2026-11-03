//! Platform calls the actions need that Tauri doesn't wrap: history
//! navigation on the webview and writing to the clipboard. Both are plain
//! host-side calls; nothing runs in the page (ADR 0008).
//!
//! Every module has the same functions:
//!
//! * `go_back(PlatformWebview)` and `go_forward(PlatformWebview)`, called on
//!   the main thread through `with_webview`.
//! * `copy_text(&WebviewWindow, &str)`, called on the main thread.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
