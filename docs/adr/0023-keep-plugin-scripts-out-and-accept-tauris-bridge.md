---
status: accepted
date: 2026-11-04
---

# Keep plugin scripts out of lichess.org and accept Tauri's own bridge

## Context and Problem Statement

[ADR 0008](0008-no-code-injection-into-lichess-org.md) says the app injects nothing into lichess.org, and [ADR 0021](0021-native-notifications-from-document-title-changes.md) listed a known gap: Tauri adds the init script of every registered plugin to every webview, without looking at the URL ([`manager/webview.rs`](https://github.com/tauri-apps/tauri/blob/tauri-v2.12.5/crates/tauri/src/manager/webview.rs) in 2.12.5). This ADR decides what reaches the page and replaces ADR 0008, whose claim was too broad (#59).

Measured in the running app on Fedora 44 with WebKitGTK 2.54.2, tauri 2.12.5, tauri-plugin-opener 2.7.2 and tauri-plugin-notification 2.8.0. A temporary build evaluated a probe in the page and dispatched synthetic `MouseEvent`s:

| | Default plugin setup | Scripts removed |
| --- | --- | --- |
| `target=_blank`, Ctrl and Shift clicks | The opener script calls `preventDefault()`, its IPC call has no capability, and nothing happens | The navigation handlers run: lichess links load in the window, an external `_blank` link goes to the browser |
| `window.Notification` | The plugin's function | `function Notification() { [native code] }` |
| `Notification.requestPermission()` | Rejects with "not allowed" | Resolves to `denied` |
| IPC from the page | Denied | Denied |
| `isTauri`, `__TAURI_INTERNALS__`, `__TAURI_EVENT_PLUGIN_INTERNALS__`, `ipc` | Present | Present |

Findings from the source:

* `tauri_plugin_opener::Builder::open_js_links_on_click(false)` leaves out the [click listener](https://docs.rs/tauri-plugin-opener/2.7.2/tauri_plugin_opener/struct.Builder.html#method.open_js_links_on_click).
* `tauri_plugin_notification::init()` always registers its script and has no option for it. But [`tauri::plugin::Plugin`](https://docs.rs/tauri/2.12.5/tauri/plugin/trait.Plugin.html) is a public trait, `Builder::plugin` accepts any implementor, and `initialization_script` defaults to `None`.
* The deep-link, single-instance, store and window-state plugins add no script.
* Tauri always defines its bridge and registers its core plugins, with no setting to turn them off. `Builder::invoke_system` only replaces the invoke part. Besides the globals above, they add the event and path APIs, a `mousedown` listener for `data-tauri-drag-region`, a `window.print` replacement on macOS, and a devtools hotkey listener in debug builds.
* Lila checks `'Notification' in window` before using the API ([`notification.ts`](https://github.com/lichess-org/lila/blob/bdd824062f/ui/lib/src/notification.ts)) and asks for permission when the notification bell is clicked ([`topBar.ts`](https://github.com/lichess-org/lila/blob/bdd824062f/ui/site/src/topBar.ts)).

## Considered Options

* Register plugins without their scripts, accept Tauri's bridge
* Call `notify-rust` directly instead of using the notification plugin
* Accept all the scripts and only correct the documents
* Remove Tauri's globals from the page with a script of our own

## Decision Outcome

Chosen option: "Register plugins without their scripts, accept Tauri's bridge", because it fixes the dead links and returns `window.Notification` to the webview's own with a small wrapper. Calling `notify-rust` directly would mean reimplementing the plugin's notification and click handling across platforms. Accepting the scripts would leave links broken. A cleanup script would itself be injected code, and the globals are defined as non-configurable.

* **No scripts of ours or our plugins.** The app adds no JavaScript or CSS to lichess.org and doesn't read the DOM. All signals come from the Rust side (navigation, document title) or from the official [Lichess API](https://lichess.org/api). This part of ADR 0008 stands.
* **Plugins.** `src-tauri/src/plugins.rs` registers the opener with its option off and wraps the notification plugin so it reports no script. A test checks that no registered plugin adds one, so an update can't bring one in unnoticed.
* **Accepted.** Tauri's own bridge and core plugin scripts stay in the page. They can't be turned off, and the page can detect them. IPC from the page stays denied (ADR 0005).

### Consequences

* Good, because `target=_blank`, Ctrl and Shift clicks behave as ADR 0005 describes.
* Good, because lichess.org gets the webview's own notification API, and only Tauri's scripts remain.
* Bad, because the page can still tell it runs in Tauri, so the app is detectable. This is not "lichess.org exactly as a browser sees it".
* Bad, because the wrapper has to forward every `Plugin` method: a method added by a Tauri update falls back to its default until it is added there.
* Bad, because only Linux was measured. On Windows, the bell may now show WebView2's permission prompt, and granting it could show lichess's own notifications next to ours. On macOS, `window.Notification` may no longer exist in WKWebView. Both need checking in the running app.

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
