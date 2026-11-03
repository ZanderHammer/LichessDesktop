<div align="center">

<img src="src-tauri/icons/128x128.png" alt="Lichess Desktop icon: a white pawn on a dark square" width="96">

# Lichess Desktop

**lichess.org in a window of its own. Small, quiet, and hands-off.**

[![CI](https://github.com/ZanderHammer/LichessDesktop/actions/workflows/ci.yml/badge.svg)](https://github.com/ZanderHammer/LichessDesktop/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/ZanderHammer/LichessDesktop)](https://github.com/ZanderHammer/LichessDesktop/releases/latest)
[![License: AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)](LICENSE)
![Platforms: Windows, macOS, Linux](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)

[Download](#install) · [Features](#features) · [Build from source](#build-from-source) · [Support Lichess](#support-lichess-not-this-project)

</div>

> [!IMPORTANT]
> **This is an unofficial, community project.** It is **not** made by, affiliated with, endorsed by or connected to Lichess or the Lichess team in any way. "Lichess" is used only to describe the website this app opens. Please don't send bug reports about this app to Lichess. [Open an issue here](https://github.com/ZanderHammer/LichessDesktop/issues) instead.

If you play on lichess every day, you've probably lost the tab more than once. Lichess Desktop gives lichess.org its own window, its own taskbar or dock entry and a tray icon, so your games are always one click away.

It uses your operating system's built-in webview instead of shipping a whole browser, so the installer is about **2 MB** on Linux (`.deb`/`.rpm`). The page itself is left completely alone: no injected scripts, no restyling, no ads, no tracking. What you see is lichess.org exactly as served.

## Support Lichess, not this project

> [!TIP]
> **Lichess is free for everyone, forever. No ads, no paywalls, no premium tier.** It's open source and run as a non-profit, paid for by donations from players like you.
>
> This wrapper is only a window. All the real work (the servers, the engine analysis, the puzzles, the tournaments, the community) is done by Lichess. If you enjoy this app, please **[donate to Lichess](https://lichess.org/patron)**.

## Features

- **Its own window.** lichess.org outside your browser, with its own taskbar or dock entry.
- **Tray icon.** Show, hide or quit from the system tray. Closing the window can hide it to the tray instead of quitting (off by default, set in Settings).
- **Settings and About window.** Open it from the tray menu to choose close to tray, update checks, language and notifications. It's the only window with (scoped) native access ([ADR 0012](docs/adr/0012-scoped-ipc-for-bundled-app-windows.md)).
- **Remembers your window.** Size and position are restored on the next launch.
- **External links open in your browser.** Only lichess.org loads inside the app.
- **Offline page.** If lichess.org can't be reached at startup, you get a simple page with a Retry link ([ADR 0011](docs/adr/0011-offline-page-from-a-startup-probe.md)).
- **Deep links.** `lichess-desktop://<path>` opens `https://lichess.org/<path>`, e.g. `lichess-desktop://tv` ([ADR 0013](docs/adr/0013-lichess-desktop-deep-link-scheme.md)).
- **Hands off the page.** No JavaScript, CSS or DOM access on lichess.org ([ADR 0008](docs/adr/0008-no-code-injection-into-lichess-org.md)).
- **Locked down.** The web page gets no access to native APIs ([ADR 0005](docs/adr/0005-remote-content-security.md)).

## Install

Grab the installer for your platform from the [latest release](https://github.com/ZanderHammer/LichessDesktop/releases/latest):

| Platform | Files |
| --- | --- |
| Windows | `.exe` (NSIS), `.msi` |
| macOS (Apple Silicon and Intel) | `.dmg` |
| Linux | `.deb`, `.rpm`, `.AppImage` |

> [!WARNING]
> Builds are not code-signed yet, so Windows SmartScreen or macOS Gatekeeper may warn you on first launch.

### Known limitations

- **Linux on Wayland:** only the window size is restored, not its position. Wayland doesn't let apps read or set their window position. On KDE, a window rule can remember it for you.
- **Linux on GNOME:** the tray icon needs the [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/) (Ubuntu enables it by default). Without it there is no tray, and closing the window always quits.
- **Linux AppImage:** it bundles WebKitGTK, so it's about 110 MB. Prefer `.deb` or `.rpm` if you can.

## Build from source

You need the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS, stable Rust, and the Tauri CLI. No Node.js required.

```sh
cargo install tauri-cli --version "^2.0.0" --locked
cargo tauri dev     # run in development
cargo tauri build   # build installers into target/release/bundle/
```

## How it's built

- [Tauri v2](https://v2.tauri.app) in pure Rust, using the system webview (WebView2, WKWebView or WebKitGTK). See [ADR 0002](docs/adr/0002-use-tauri-v2.md) for measured sizes and RAM use.
- Every design decision is recorded as an ADR in [`docs/adr`](docs/adr).
- Released by pushing a `vX.Y.Z` tag; release notes live on [GitHub Releases](https://github.com/ZanderHammer/LichessDesktop/releases).

## Credits

- **[Lichess](https://lichess.org)** and its team and community, for building the best free chess site there is. Remember to [support them](https://lichess.org/patron).
- **Translations** come from [lila](https://github.com/lichess-org/lila) (`translation/dest`), the software behind lichess.org. They are copyright the [lila authors](https://github.com/lichess-org/lila/graphs/contributors) and the Lichess translators, and licensed under the [GNU AGPL-3.0-or-later](https://github.com/lichess-org/lila/blob/master/COPYING.md).
