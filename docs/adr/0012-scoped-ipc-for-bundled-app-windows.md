---
status: accepted
date: 2026-11-03
---

# Scoped IPC for bundled app windows, none for lichess.org

## Context and Problem Statement

v0.3 adds two windows with our own bundled HTML: settings/about (#15) and the command palette (#16). They need to read and change settings and run app actions, so unlike lichess.org they need IPC. [ADR 0005](0005-remote-content-security.md) says "we define no custom commands" and must keep holding for the main window. This ADR extends ADR 0005 and does not replace it.

Checked against tauri 2.12.5, tauri-build 2.7.4, tauri-utils 2.10.2 and tauri-codegen 2.7.4:

* Capabilities target **window labels**, and a capability without `remote` only applies to local content ([capabilities docs](https://v2.tauri.app/security/capabilities/)). Labels may be glob patterns, so `"*"` would include `main`.
* **App commands skip the ACL unless the app has an ACL manifest.** In `Webview::on_message` ([`webview/mod.rs`](https://docs.rs/crate/tauri/2.12.5/source/src/webview/mod.rs)) the ACL is only checked if the command is a plugin command, if `has_app_manifest()` is true, or if the origin is remote. tauri-build only creates that manifest when the app has at least one permission ([`acl.rs`](https://docs.rs/crate/tauri-build/2.7.4/source/src/acl.rs)). The usual way is `tauri_build::AppManifest::new().commands(&[…])`, which generates an `allow-<command>`/`deny-<command>` pair per command ([AppManifest docs](https://docs.rs/tauri-build/2.7.4/tauri_build/struct.AppManifest.html)). Without that, every local page in every window could call every registered command.
* **The offline page counts as local.** `Webview::is_local_url` treats any registered custom scheme as local, so the `offline` scheme from [ADR 0011](0011-offline-page-from-a-startup-probe.md) does too, in the main window. Today only its script-free CSP keeps it away from IPC, and `capabilities/default.json` still grants `core:default` to `main`.
* **`app.withGlobalTauri` reaches lichess.org.** With it on, tauri-codegen adds the plugin API scripts (`window.__TAURI__`) to every webview ([`context.rs`](https://docs.rs/crate/tauri-codegen/2.7.4/source/src/context.rs)), the lichess window included. That would break [ADR 0008](0008-no-code-injection-into-lichess-org.md). Tauri's own `window.__TAURI_INTERNALS__` bridge (with `invoke`) is injected into every webview anyway, and the `@tauri-apps/api` `invoke` is a thin wrapper around it.
* `app.security.csp` only applies to the assets Tauri serves itself ([CSP docs](https://v2.tauri.app/security/csp/)). It doesn't touch lichess.org, or the offline page, which sends its own header.

## Considered Options

* One capability per bundled window, app commands listed in an ACL manifest, and no capability for `main`
* A single capability for all windows (`"windows": ["*"]`)
* No IPC: native-only UI (menus, dialogs) for settings and no palette
* The isolation pattern for the bundled windows

## Decision Outcome

Chosen option: "One capability per bundled window, app commands listed in an ACL manifest, and no capability for `main`".

A glob would include `main`. A native-only UI can't give us a searchable palette or a settings window that screen readers can use. The [isolation pattern](https://v2.tauri.app/concept/inter-process-communication/isolation/) protects against untrusted frontend dependencies, and ours has none: plain HTML/CSS/JS with no build step (#14).

* **Windows.** Bundled windows are opened from Rust with `WebviewUrl::App(…)` and have fixed labels: `settings` and `palette`. Each has its own navigation handler. It allows only that window's own bundled page. Other links (repo, license, lichess.org) open in the system browser, as in ADR 0005, and new windows are denied. A bundled window never shows remote content.
* **Capabilities.** One file per label, `src-tauri/capabilities/settings.json` and `palette.json`. Each names exactly one label: no glob, no `webviews`, never `remote`. It grants only that window's commands, plus `core:event:allow-listen` and `core:event:allow-unlisten` so the page can follow changes pushed from Rust (for example a language switch, #18). `core:default` and other core permissions are added only with a stated reason in the capability's `description`.
* **`main` has no capability.** `capabilities/default.json` is removed. Rust-side APIs need no capability, so nothing in the main window depends on one. lichess.org and the offline page then reach neither plugin nor app commands.
* **Commands.** Every `#[tauri::command]` is listed in `AppManifest::commands` in `src-tauri/build.rs`, so the ACL covers app commands. The set stays small and typed, as planned in #13:
  * `get_settings` and `set_setting`: settings. `set_setting` takes a key/value enum that serde checks, not free-form JSON.
  * `list_actions` and `run_action`: the palette. `run_action` takes an action id from the shared shortcut table (ADR 0010) and runs the same Rust function as the shortcut.

  There is no command that takes a URL, a path or a script. A new command needs an entry in the manifest and in exactly the capabilities that use it.
* **Frontend bridge.** `app.withGlobalTauri` stays off. Bundled pages call `window.__TAURI_INTERNALS__.invoke` through one small shared `ipc.js` wrapper, the same call `@tauri-apps/api/core` makes. Because it is an internal API, the wrapper is the only place that touches it.
* **CSP.** `app.security.csp` is set for the bundled assets: `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src ipc: http://ipc.localhost`. No inline scripts and no remote loads.
* **Check.** A unit test reads `src-tauri/capabilities/` and fails if any capability names `main`, uses a glob or `webviews`, or declares `remote`.

ADR 0005's rule still stands: the web content in the main window has no IPC and stays behind the navigation allowlist.

### Consequences

* Good, because the settings and palette windows get exactly the commands they need, and the runtime authority denies everything else before any command runs.
* Good, because the main window loses the `core:default` grant it never used, so the offline page no longer depends only on its CSP.
* Good, because no extra script reaches lichess.org (ADR 0008).
* Bad, because adding a command takes three edits (handler, `build.rs` manifest, capability). That is on purpose.
* Bad, because `__TAURI_INTERNALS__` is not a stable public API. A Tauri update may need a change in `ipc.js`; the wrapper keeps that change in one place.
