---
status: accepted
date: 2026-11-04
---

# Native notifications from document title changes

## Context and Problem Statement

Players should learn that it's their turn or that a game ended while the app is in the background (#20, #21, #23). [ADR 0005](0005-remote-content-security.md) keeps lichess.org without IPC and says native integrations need a deliberate new ADR and a scoped capability. [ADR 0008](0008-no-code-injection-into-lichess-org.md) rules out injected code and DOM reading. This ADR refines the note in ADR 0005 for notifications: where they are sent from, and how the app knows when to send one.

Findings:

* Lila sets ``document.title = `${text} - ${initialTitle}` `` in [`ui/round/src/title.ts`](https://github.com/lichess-org/lila/blob/bdd824062f/ui/round/src/title.ts), where `text` is `site.yourTurn`, `site.waitingForOpponent` or `site.gameOver` in the user's lichess language. For a spectator it returns without setting the title.
* The lichess language is a setting of the site, separate from the app's UI language (ADR 0015), so a title can be in any language lila has.
* Checked against lila commit [`bdd824062f`](https://github.com/lichess-org/lila/tree/bdd824062f): `translation/dest/site` has 149 files. 123 have `yourTurn`, 122 have `waitingForOpponent`, 123 have `gameOver`, and 122 have all three. In these files no text stands for two different states, and no text contains ` - `, stray whitespace or bidi control characters.
* [`WebviewWindowBuilder::on_document_title_changed`](https://docs.rs/tauri/2.12.5/tauri/webview/struct.WebviewWindowBuilder.html#method.on_document_title_changed) reports the title on the Rust side, and "may run before or after the navigation event". In wry 0.57 it uses the webview's own title notification (WebKitGTK `title` property, WebView2 `DocumentTitleChanged`, a WKWebView observer). No script is involved.
* [`tauri-plugin-notification`](https://docs.rs/tauri-plugin-notification/2.8.0/) 2.8 is already in the app (ADR 0017), and its Rust API needs no capability. `src-tauri/capabilities/` grants the page nothing.
* The plugin's `silent()` only applies to iOS. On desktop, silence means not setting a sound:
  * Windows: `notify-rust` 4.18 passes no sound to `tauri-winrt-notification` 0.8, which writes `<audio silent="true"/>`.
  * macOS: no sound is set on the notification.
  * Linux: no `sound-name` hint is sent. The plugin forwards only that hint, so it can't send [`suppress-sound`](https://specifications.freedesktop.org/notification/latest-single/), and the notification server decides.

## Considered Options

* Send from Rust only, detect from title changes matched against a bundled lila table
* Grant lichess.org a notification capability (`remote.urls`) and let the page send them
* Detect with an injected script or by reading the DOM
* Detect with the official Lichess API only

## Decision Outcome

Chosen option: "Send from Rust only, detect from title changes", because it keeps the page without any capability, needs no code of ours in the page, and works without a Lichess account. A capability would give a page we don't control a native API (ADR 0005), injection is ruled out (ADR 0008), and the [API](https://lichess.org/api)'s event stream (`/api/stream/event`) needs a connected account, so it stays an addition for account events (#25).

* **Sending.** Notifications are sent only from Rust, with `tauri-plugin-notification`. No capability grants the lichess page anything, and ADR 0005 stays as it is.
* **Detecting.** `on_document_title_changed` on the main window is the only signal for turn state. The app adds no script or style of its own for this and doesn't read the DOM.
* **Matching.** The title is matched against a bundled table of the three strings in every language lila has them, plus the English source strings. The table is separate from the UI strings, because the 83 languages we offer (ADR 0020) don't limit what the page can show. A title that doesn't match means no state and never an alert.
* **Unfocused only.** A notification is shown only if the main window isn't focused when the state changes.
* **Silent.** Notifications set no sound, because lichess plays its own.

Built in other issues: the table and parser (#21), a watcher for changes to lila's title format (#22), the notifications themselves, their settings and what a click does (#23).

### Known gap: plugin scripts reach the page

This ADR doesn't make the page free of injected code. Tauri adds the init script of every registered plugin to every webview it builds, without looking at the URL ([`manager/webview.rs`](https://github.com/tauri-apps/tauri/blob/tauri-v2.12.5/crates/tauri/src/manager/webview.rs) in 2.12.5). This is from reading the source and wasn't observed in the running app. In the main window that means:

* `tauri_plugin_notification::init()` always registers `init-iife.js`, which replaces `window.Notification` with a function that calls `plugin:notification|notify`. The app has done this since ADR 0017. The page has no capability, so the call should be denied, and lichess's own browser notifications then do nothing.
* `tauri_plugin_opener::init()` registers a click listener by default (`open_js_links_on_click`), for `target=_blank` links and for Ctrl or Shift clicks.
* Tauri's own scripts define `window.isTauri` and `window.__TAURI_INTERNALS__`.

The first two run in the page's JS context, which ADR 0008 set out to avoid, and the page can detect them. Whether to work around them is a separate decision. It would change ADR 0008 or how we send notifications, so it isn't decided here.

### Consequences

* Good, because a compromised page still can't reach a native API, and notifications add no capability.
* Good, because detection needs no account and no code of ours in the page.
* Good, because an unknown title or a changed format means a missed alert and not a wrong one.
* Bad, because a change to lila's title format silently stops notifications until #22 catches it and the code is updated.
* Bad, because a page whose own title starts with one of the strings followed by ` - ` looks like a game.
* Bad, because a turn that begins while the window is focused gives no notification when the user then leaves the window.
* Bad, because a Linux notification server may still play a sound of its own.
* Bad, because webviews can throttle or unload a [hidden or minimized view](https://docs.rs/tauri/2.12.5/tauri/webview/struct.WebviewWindowBuilder.html#method.background_throttling) after about 5 minutes, so titles may stop changing there. Tauri's policy setting only works on macOS 14+ and iOS 17+. #23 needs to test this.

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
