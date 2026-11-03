---
status: accepted
date: 2026-11-03
---

# Offline page chosen by a startup reachability probe

## Context and Problem Statement

If lichess.org can't be reached, the window shows the webview's raw error page (#11). We want a small bundled page instead: "Can't reach lichess.org" plus a Retry link. To do that, the Rust side has to know that the main-frame load failed. This was checked against tauri 2.12.5 and wry 0.57.1:

* Tauri's `on_page_load` passes a [`PageLoadPayload`](https://docs.rs/tauri/2.12.5/tauri/webview/struct.PageLoadPayload.html) that holds only the URL and `Started`/`Finished`, with no error status.
* **Windows:** wry fires `Finished` from WebView2's `NavigationCompleted` and drops its args, including `IsSuccess` ([`webview2/mod.rs`](https://docs.rs/crate/wry/0.57.1/source/src/webview2/mod.rs)). A failed load looks the same as a successful one.
* **Linux:** wry maps only WebKitGTK's `load-changed` `Committed`/`Finished` and doesn't connect `load-failed` ([`webkitgtk/mod.rs`](https://docs.rs/crate/wry/0.57.1/source/src/webkitgtk/mod.rs)).
* **macOS:** wry's `WKNavigationDelegate` implements only `didCommitNavigation` and `didFinishNavigation` ([`wry_navigation_delegate.rs`](https://docs.rs/crate/wry/0.57.1/source/src/wkwebview/class/wry_navigation_delegate.rs)). A provisional failure fires nothing. wry owns the delegate, so we can't add `didFailProvisionalNavigation` without replacing it.
* Upstream [tauri#9689](https://github.com/tauri-apps/tauri/issues/9689) asks for exactly this and is still open.

ADR 0008 also rules out detecting the failure from inside the page.

## Considered Options

* Probe lichess.org from Rust at startup and load the bundled page instead if it is unreachable
* Hook native load-failure events per platform through `with_webview` (WebKitGTK `load-failed`, WebView2 `NavigationCompleted`)
* Drop the feature until Tauri reports load errors

## Decision Outcome

Chosen option: "Probe lichess.org from Rust at startup". It works the same on every platform, needs no new dependency and no access to the webview internals. The native-hook option needs direct `webkit2gtk`/`webview2-com` dependencies pinned to wry's versions and still has no clean path on macOS.

* `src-tauri/src/offline.rs` starts a thread at the top of `main`. It resolves `lichess.org` and opens a TCP connection to port 443, and sends nothing. Setup waits for the result for at most 3 s. This runs while Tauri starts up, so it adds no visible delay when online.
* If the probe fails, the main window opens the bundled page `src-tauri/src/offline.html` instead of `https://lichess.org`. A probe that takes longer than 3 s counts as reachable, so slow networks still get the real page.
* The page is served from a custom URI scheme, `offline` (`offline://localhost/`, or `http://offline.localhost/` on Windows; see [`register_uri_scheme_protocol`](https://docs.rs/tauri/2.12.5/tauri/struct.Builder.html#method.register_uri_scheme_protocol)). Its response sets `Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'`. The page has no scripts and none can run. Tauri treats custom schemes as local, so this CSP is what keeps the page away from IPC.
* The navigation allowlist (ADR 0005) allows that one origin and no other local URL.
* Retry is a plain link to `https://lichess.org/`. The page follows the OS light/dark theme through `prefers-color-scheme` and has no lichess branding.
* Only the startup load is covered. lichess.org handles its own websocket reconnects. A navigation that fails later in the session still shows the webview's error page.

### Consequences

* Good, because users who start the app offline see a clear page with a way back instead of a raw engine error.
* Good, because the web content still has no IPC and the allowlist gains a single local origin.
* Bad, because the probe is a heuristic. Behind a proxy that blocks direct connections it can report "offline" while the webview could load. Retry then works, at the cost of one click.
* Bad, because a failure after startup still shows the raw error page. Revisit this when Tauri exposes load errors (tauri#9689).
