---
status: accepted
date: 2026-11-03
---

# No code injection into lichess.org

## Context and Problem Statement

The app window shows lichess.org, a page we don't control. Tauri and the underlying webviews can inject code into loaded pages (initialization scripts, user styles, `eval`), and a page's DOM can also be read from the Rust side. Should we use any of that to integrate with lichess.org?

This complements [ADR 0005](0005-remote-content-security.md), which already keeps lichess.org without IPC access.

## Considered Options

* Never inject JS or CSS into lichess.org pages and never read the DOM; take all signals from the Rust side (navigation events, document title) or from the official Lichess API
* Inject an initialization script and/or user style to customize or observe the page

## Decision Outcome

Chosen option: "No code injection, Rust-side and API signals only".

* The app never injects JavaScript or CSS into lichess.org pages: no initialization scripts, no user styles, no DOM reading. `src-tauri/src/main.rs` builds the webview with default settings and registers no script or style hooks.
* All signals come from the Rust side (navigation, document title) or from the official [Lichess API](https://lichess.org/api).

Why:

* The [Lichess terms of service](https://lichess.org/terms-of-service) reserve the right to ban "certain software or extensions at our discretion". Injected code runs in the same JS context as the page, so it cannot be guaranteed undetectable.
* Reading the DOM breaks more often than the document title format does.
* It keeps ADR 0005 meaningful: remote content stays fully opaque to us, and we never reach into it.

### Consequences

* Good, because there is no injected code that lichess could detect and ban the app for.
* Good, because integrations rely on stable surfaces (navigation, title, official API) instead of page internals.
* Bad, because in-page customization or features that need page internals are off the table unless the official API grows them.
