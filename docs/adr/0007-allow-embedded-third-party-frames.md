---
status: accepted
date: 2026-11-03
---

# Allow named third-party iframe hosts (Cloudflare Turnstile) inside the app

## Context and Problem Statement

Lichess runs [Cloudflare Turnstile](https://developers.cloudflare.com/turnstile/) on its login form. The widget is an iframe served from `https://challenges.cloudflare.com/...`.

The navigation allowlist (ADR 0005) only allows lichess.org. On Linux and macOS, wry's navigation handler also fires for iframe loads:

* WebKitGTK: `decide-policy` with `NavigationAction`
* WKWebView: `decidePolicyForNavigationAction`

The handler receives only the URL, without frame information ([wry 0.57 source](https://docs.rs/crate/wry/0.57.1/source/src/webkitgtk/mod.rs)). So the iframe was blocked and opened in the system browser. The check failed there, and nobody could log in. On Windows, WebView2's `NavigationStarting` only fires for the main frame, so Windows was not affected.

## Considered Options

* Keep a short, explicit list of exact third-party hosts that may load in the app
* Allow every host in the app and open only top-level navigations externally (wry gives no frame information, so this cannot be done reliably)
* Allow `*.cloudflare.com`

## Decision Outcome

Chosen option: "explicit list of exact hosts". `src-tauri/src/navigation.rs` has `FRAME_HOSTS`, which for now holds only `challenges.cloudflare.com`. Subdomains are not included. Every new entry needs a reason in the code comment.

The rest of ADR 0005 still applies. Remote content gets no IPC.

### Consequences

* Good, because login works on Linux and macOS.
* Good, because the exception is narrow and auditable.
* Bad, because the handler cannot tell frames from top-level navigations. A top-level navigation to `challenges.cloudflare.com` also stays in the app, which is harmless because it gets no IPC.
* Bad, because other iframes that lichess embeds (for example video embeds) still open externally on Linux and macOS. Each one needs its own entry.
