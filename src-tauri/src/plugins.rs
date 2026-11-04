//! Plugins whose default setup adds a script to every webview, lichess.org
//! included. Tauri doesn't look at the URL, so they are registered without the
//! script here (ADR 0023). The app only calls these plugins from Rust.

use tauri::ipc::Invoke;
use tauri::plugin::Plugin;
use tauri::webview::PageLoadPayload;
use tauri::{AppHandle, RunEvent, Runtime, Url, Webview, Window};

/// The opener plugin without its click listener, which would take `target=_blank`
/// and Ctrl or Shift clicks away from the navigation handlers (ADR 0005).
pub fn opener<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_opener::Builder::new()
        .open_js_links_on_click(false)
        .build()
}

/// The notification plugin without its script, which would replace the page's
/// `window.Notification`. The plugin has no option for this.
pub fn notification<R: Runtime>() -> impl Plugin<R> {
    WithoutScript(tauri_plugin_notification::init())
}

/// Forwards every [`Plugin`] method except the two initialization script ones,
/// whose defaults return `None`. Check this list when updating tauri: a new
/// trait method falls back to its default until it is added here.
struct WithoutScript<P>(P);

impl<R: Runtime, P: Plugin<R>> Plugin<R> for WithoutScript<P> {
    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn initialize(
        &mut self,
        app: &AppHandle<R>,
        config: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.0.initialize(app, config)
    }

    fn window_created(&mut self, window: Window<R>) {
        self.0.window_created(window);
    }

    fn webview_created(&mut self, webview: Webview<R>) {
        self.0.webview_created(webview);
    }

    fn on_navigation(&mut self, webview: &Webview<R>, url: &Url) -> bool {
        self.0.on_navigation(webview, url)
    }

    fn on_page_load(&mut self, webview: &Webview<R>, payload: &PageLoadPayload<'_>) {
        self.0.on_page_load(webview, payload);
    }

    fn on_event(&mut self, app: &AppHandle<R>, event: &RunEvent) {
        self.0.on_event(app, event);
    }

    fn extend_api(&mut self, invoke: Invoke<R>) -> bool {
        self.0.extend_api(invoke)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};
    use tauri_plugin_notification::NotificationExt;

    fn adds_no_script(plugin: impl Plugin<MockRuntime>) -> bool {
        plugin.initialization_script().is_none() && plugin.initialization_script_2().is_none()
    }

    /// A plugin update must not bring a script into the page unnoticed. Keep
    /// this in step with the `.plugin()` calls in `main`.
    #[test]
    fn no_registered_plugin_adds_a_script() {
        assert!(adds_no_script(opener()));
        assert!(adds_no_script(notification()));
        assert!(adds_no_script(tauri_plugin_deep_link::init()));
        assert!(adds_no_script(
            tauri_plugin_window_state::Builder::default().build()
        ));
        assert!(adds_no_script(
            tauri_plugin_store::Builder::default().build()
        ));
        assert!(adds_no_script(tauri_plugin_single_instance::init(
            |_, _, _| {}
        )));
    }

    #[test]
    fn the_notification_plugin_still_has_its_script_to_remove() {
        assert!(!adds_no_script(tauri_plugin_notification::init()));
    }

    #[test]
    fn the_notification_plugin_still_works_from_rust() {
        let app = mock_builder()
            .plugin(notification())
            .build(mock_context(noop_assets()))
            .unwrap();
        // Panics if the plugin's setup, which manages this state, didn't run.
        let _ = app.notification().on_action(|_| {});
    }
}
