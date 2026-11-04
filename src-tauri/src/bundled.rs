//! What the bundled app windows, settings and the palette, have in common
//! (ADR 0012).

use std::thread;
use std::time::Duration;

use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{AppHandle, Runtime, Url, WebviewWindow, Window};

/// Shows a window anyway if its page never reports that it is ready.
const SHOW_FALLBACK: Duration = Duration::from_millis(1500);

/// The origin of the dev server. `cargo tauri dev` serves `ui/` from its own
/// server instead of the bundled pages.
pub fn dev_url<R: Runtime>(app: &AppHandle<R>) -> Option<Url> {
    app.config()
        .build
        .dev_url
        .clone()
        .filter(|_| tauri::is_dev())
}

/// For `on_page_load`: the page normally shows its window once it has drawn its
/// content, and this covers a page that never does.
pub fn show_after_fallback<R: Runtime>(window: WebviewWindow<R>, payload: PageLoadPayload<'_>) {
    if payload.event() == PageLoadEvent::Finished {
        thread::spawn(move || {
            thread::sleep(SHOW_FALLBACK);
            if !window.is_visible().unwrap_or(true) {
                show(&window.as_ref().window());
            }
        });
    }
}

pub fn show<R: Runtime>(window: &Window<R>) {
    if let Err(err) = window.show().and_then(|()| window.set_focus()) {
        eprintln!("failed to show the {} window: {err}", window.label());
    }
}

#[cfg(test)]
pub mod test_support {
    use tauri::ipc::{CallbackFn, InvokeBody};
    use tauri::test::INVOKE_KEY;
    use tauri::webview::InvokeRequest;

    /// The origin of the bundled pages.
    pub const LOCAL: &str = if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };

    pub fn request(cmd: &str, url: &str) -> InvokeRequest {
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body: InvokeBody::default(),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.into(),
        }
    }
}
