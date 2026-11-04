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

It uses your operating system's built-in webview instead of shipping a whole browser, so the installer is about **2 MB** on Linux (`.deb`/`.rpm`). The app adds no scripts of its own to the page, no restyling, no ads and no tracking. What you see is lichess.org as served, plus the small bridge Tauri puts in every webview, which the page can't use to reach anything native.

## Support Lichess, not this project

> [!TIP]
> **Lichess is free for everyone, forever. No ads, no paywalls, no premium tier.** It's open source and run as a non-profit, paid for by donations from players like you.
>
> This wrapper is only a window. All the real work (the servers, the engine analysis, the puzzles, the tournaments, the community) is done by Lichess. If you enjoy this app, please **[donate to Lichess](https://lichess.org/patron)**.

## Features

- **Its own window.** lichess.org outside your browser, with its own taskbar or dock entry.
- **Tray icon.** Show, hide or quit from the system tray. Closing the window can hide it to the tray instead of quitting (off by default, set in Settings).
- **Command palette.** Ctrl/Cmd+K (or the tray menu) opens a search box over the window. Type a few letters to run any app action or jump to a lichess page (puzzles, analysis board, study, TV, your preferences…), then press Enter. Esc closes it.
- **Settings, Shortcuts and About window.** Open it with Ctrl/Cmd+, or from the tray menu to choose close to tray, update checks, language and notifications; changes apply at once and Esc closes it. It and the palette are the only windows with (scoped) native access ([ADR 0012](docs/adr/0012-scoped-ipc-for-bundled-app-windows.md)).
- **Keyboard shortcuts.** Back and forward (Alt+←/→, Cmd+[ / Cmd+] on macOS), reload (F5, Ctrl/Cmd+R), lichess.org home (Alt+Home), copy the page address (Ctrl/Cmd+Shift+C), open the page in your browser (Ctrl/Cmd+Shift+O) and zoom the page in (Ctrl/Cmd + `=`/`+`), out (Ctrl/Cmd + `-`) or back to 100 % (Ctrl/Cmd + `0`; the level is remembered across restarts), and quit (Ctrl+Q, Cmd+Q on macOS). The mouse back/forward buttons work too. Ctrl/Cmd+/ or F1 (or the tray menu) opens a list of all shortcuts. On macOS they are also in the app, Navigation and Help menus ([ADR 0010](docs/adr/0010-window-level-shortcuts-without-a-menu-bar.md)).
- **Your-turn and game-over alerts.** While the window isn't focused, you get a silent native notification and taskbar attention when it's your turn or a game ends. Clicking the notification opens the game. Each alert can be turned off in Settings. During your turn the tray icon also shows a dot. The app reads only the page title and gives the page no native access ([ADR 0021](docs/adr/0021-native-notifications-from-document-title-changes.md), [ADR 0022](docs/adr/0022-alerts-per-game-and-clicks-that-open-the-game.md)).
- **Connect your Lichess account (optional).** Settings can connect your account through Lichess's own authorization page, asking only to read your incoming challenges. The token is kept only in your system keyring, never in a file: without a keyring, the app says it can't connect securely. Disconnecting revokes the token ([ADR 0024](docs/adr/0024-connect-a-lichess-account-with-oauth-pkce-and-the-os-keyring.md)).
- **Challenge and game-start notifications.** With a connected account, the app keeps Lichess's event stream open from Rust. While the window isn't focused, you get a notification for an incoming challenge or a game that starts, such as a tournament pairing or an accepted challenge, and clicking it opens that page. Settings can turn this off, which also closes the stream ([ADR 0025](docs/adr/0025-challenge-and-game-start-notifications-from-the-event-stream.md)).
- **Update notice.** Once a day at most, the app asks GitHub for the latest release. If there's a newer one you get one notification, and the tray menu and the About tab link to its release page. The About tab always shows the current status. Nothing is installed for you, and the check can be turned off in Settings ([ADR 0017](docs/adr/0017-update-notice-from-the-github-releases-api.md), [ADR 0019](docs/adr/0019-update-status-and-check-from-the-setting.md)).
- **Remembers your window.** Size and position are restored on the next launch.
- **External links open in your browser.** Only lichess.org loads inside the app.
- **Offline page.** If lichess.org can't be reached at startup, you get a simple page with a Retry link ([ADR 0011](docs/adr/0011-offline-page-from-a-startup-probe.md)).
- **Deep links.** `lichess-desktop://<path>` opens `https://lichess.org/<path>`, e.g. `lichess-desktop://tv` ([ADR 0013](docs/adr/0013-lichess-desktop-deep-link-scheme.md)).
- **Hands off the page.** The app and its plugins add no JavaScript or CSS to lichess.org and don't read the DOM. Only Tauri's own bridge is there, and it can't be turned off ([ADR 0023](docs/adr/0023-keep-plugin-scripts-out-and-accept-tauris-bridge.md)).
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

- **Linux on Wayland:** only the window size is restored, not its position. Wayland doesn't let apps read or set their window position. Always on top (Ctrl+Shift+T) is also a no-op under Wayland because `xdg_shell` does not support client-controlled window stacking. On KDE Plasma, a window rule can configure both position and always-on-top.
- **Linux on GNOME:** the tray icon needs the [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/) (Ubuntu enables it by default). Without it there is no tray, and closing the window always quits.
- **Linux AppImage:** it bundles WebKitGTK, so it's about 110 MB. Prefer `.deb` or `.rpm` if you can.
- **Linux sound delay:** sounds play up to about 0.4 s late, because shorter audio buffers go silent on Bluetooth ([ADR 0026](docs/adr/0026-keep-linux-bluetooth-audio-with-a-larger-pulse-request-size.md)). To try a smaller buffer, start the app with your own value, such as `PULSE_PROP=pulse.min.req=1024/48000`.

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
- **Translations** of the strings the app shares with lichess come from [lila](https://github.com/lichess-org/lila) (`translation/dest`), the software behind lichess.org. They are copyright the [lila authors](https://github.com/lichess-org/lila/graphs/contributors) and the Lichess translators, and licensed under the [GNU AGPL-3.0-or-later](https://github.com/lichess-org/lila/blob/master/COPYING.md). Strings lila doesn't have are translated by Claude and haven't been reviewed by native speakers, so corrections to `src-tauri/i18n/` are welcome. [ADR 0020](docs/adr/0020-import-lila-translations-and-translate-the-rest.md) explains how to regenerate them.
