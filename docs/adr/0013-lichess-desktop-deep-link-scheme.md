---
status: accepted
date: 2026-11-03
---

# `lichess-desktop://` deep links mapped onto lichess.org

## Context and Problem Statement

Links from outside the app (notes, chat, scripts) should be able to open a lichess page in the app instead of the browser (#10). This was checked against tauri-plugin-deep-link 2.6.3, tauri-plugin-single-instance 2.5.5 and the [deep linking guide](https://v2.tauri.app/plugin/deep-linking/):

* Schemes in `plugins > deep-link > desktop > schemes` are picked up by the bundler: `MimeType=x-scheme-handler/...` in the deb/rpm desktop file ([`freedesktop/mod.rs`](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle/linux/freedesktop/mod.rs)), `CFBundleURLTypes` in the macOS bundle, and registry keys from the NSIS and MSI installers.
* An AppImage is not installed, so its desktop file is only seen with an AppImage launcher. The plugin's `register_all` writes `~/.local/share/applications/<bin>-handler.desktop` (`NoDisplay=true`, `Exec` set to `$APPIMAGE`) and runs `xdg-mime default`. Because the path is absolute, it has to run again after the AppImage moves. On Windows it writes `HKCU\Software\Classes\<scheme>`. macOS has no runtime registration.
* On Linux and Windows the link arrives as the only argument of a new process. With its `deep-link` feature, single-instance hands the second launch's arguments to the deep-link plugin, which emits the URL to `on_open_url`. On a cold start `get_current` returns it during setup. On macOS it always arrives through `RunEvent::Opened`, which the plugin also emits to `on_open_url`.
* The plugin only matches the scheme. Anyone can start the app with an arbitrary `lichess-desktop:` argument, so the URL has to be validated by us.

## Considered Options

* A custom `lichess-desktop://<path>` scheme that maps to `https://lichess.org/<path>`
* Claim `https://lichess.org` links from the browser
* A custom scheme that carries a full URL (`lichess-desktop://open?url=...`)

## Decision Outcome

Chosen option: "a custom scheme mapped to lichess.org", because it needs no verification and no web server, and the target host is fixed by construction.

* `src-tauri/src/deep_link.rs` takes everything after `lichess-desktop:` and its leading slashes and appends it to `https://lichess.org/`. The result must have exactly the host `lichess.org`, no port and no credentials, and must pass `navigation::is_internal` (ADR 0005). Anything else is dropped. A unit test pins the scheme to the one in `tauri.conf.json`.
* `lichess-desktop://@/name` is not a valid URL (an empty host after `@`), so profile links are written `lichess-desktop:///@/name` or `lichess-desktop:@/name`.
* Cold start: the mapped URL replaces the start URL, unless the offline probe failed (ADR 0011). While running: the main window navigates to it and is focused. If several links arrive at once (macOS), the last valid one wins.
* The app registers the scheme itself only where no installer did: in an AppImage, on every start (in a background thread, as it spawns `xdg-mime`), and in debug builds on Linux and Windows so `cargo tauri dev` can be tested.
* No `https://lichess.org` links are claimed. Desktop has no verified links, and taking over the browser's lichess links would surprise users.
* The plugin is used from Rust only. The main window has no capability (ADR 0012), so its commands stay unreachable.

### Consequences

* Good, because any lichess page can be opened in the app from outside, cold or while running.
* Good, because a deep link can't load anything the allowlist would block.
* Bad, because a link opened while offline at startup only shows the offline page, and the target is lost.
* Bad, because a debug run on Linux or Windows makes the dev binary the scheme handler until an AppImage start or a reinstall takes it back.
* Bad, because the AppImage leaves a hidden handler desktop file behind when it is deleted.
