---
status: accepted
date: 2026-11-03
---

# lichess.org runs without IPC access, behind a navigation allowlist

## Context and Problem Statement

The app window shows remote content that we don't control. Tauri can expose native APIs (IPC) to web content, and a page could also navigate the window anywhere.

## Considered Options

* No IPC for remote content + host allowlist; everything else opens in the system browser
* Grant lichess.org selected capabilities (`remote` in a capability file)

## Decision Outcome

Chosen option: "No IPC for remote content + host allowlist".

* No capability declares `remote.urls`. Per the [Tauri docs](https://v2.tauri.app/security/capabilities/), the API "is only accessible to bundled code" by default. We define no custom commands.
* `src-tauri/src/navigation.rs` allows only `https://lichess.org` and its subdomains (plus `about:` and `blob:`) inside the app.
  * Any other navigation is cancelled and opened in the system browser via `tauri-plugin-opener`.
  * `target=_blank` links to lichess are loaded in the main window; external ones go to the browser.
* Debug tooling that needs IPC from the page is not used. One example is the `tauri-plugin-mcp-bridge` of `@hypothesi/tauri-mcp-server`: it needs `window.__TAURI__` and binds to `0.0.0.0` by default. Adopting it requires a new ADR.

### Consequences

* Good, because a compromised or malicious page cannot reach native APIs or turn the window into a general browser.
* Bad, because native integrations (notifications, for example) need a deliberate new ADR and a scoped capability.
