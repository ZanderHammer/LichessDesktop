# Lichess Desktop

A lightweight, unofficial desktop app for [lichess.org](https://lichess.org) on Windows, macOS and Linux.

It is built with [Tauri v2](https://v2.tauri.app) and uses your operating system's webview instead of bundling a browser engine, so it stays small. See [ADR 0002](docs/adr/0002-use-tauri-v2.md) for measured numbers.

> **Unofficial.** This project is not affiliated with, endorsed by, or connected to Lichess. "Lichess" is used only to describe what the app opens.

## Features

- lichess.org in its own window
- Links to other sites open in your default browser
- Remembers window size and position
- Tray icon to show, hide or quit the app; closing the window can hide it to the tray instead of quitting (off by default)
- Settings and About window from the tray menu: close to tray, update checks, language, notifications; the only app window with (scoped) native access ([ADR 0012](docs/adr/0012-scoped-ipc-for-bundled-app-windows.md))
- Shows a simple offline page with a Retry link if lichess.org is unreachable at startup ([ADR 0011](docs/adr/0011-offline-page-from-a-startup-probe.md))
- Opens `lichess-desktop://<path>` links as `https://lichess.org/<path>`, e.g. `lichess-desktop://tv` ([ADR 0013](docs/adr/0013-lichess-desktop-deep-link-scheme.md))
- No access to native APIs for the web page ([ADR 0005](docs/adr/0005-remote-content-security.md))
- Does not inject or modify anything on lichess.org: no JavaScript, no CSS, no DOM access — the page is shown exactly as served ([ADR 0008](docs/adr/0008-no-code-injection-into-lichess-org.md))

## Known limitations

- **Linux (Wayland):** only the window size is restored, not its position. Wayland doesn't let apps read or set their window position; the compositor places windows. On KDE you can use a window rule to remember the position.
- **Linux (GNOME):** the tray icon only appears with the [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/) (Ubuntu enables it by default). Without it there is no tray, and closing the window always quits.

## Install

Download the installer for your platform from [Releases](https://github.com/ZanderHammer/LichessDesktop/releases):

| Platform | Files |
| --- | --- |
| Windows | `.exe` (NSIS), `.msi` |
| macOS (Apple Silicon / Intel) | `.dmg` |
| Linux | `.deb`, `.rpm`, `.AppImage` |

Builds are not code-signed yet, so your OS may show a warning on first launch.

## Build from source

Requirements: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS, Rust (stable) and the Tauri CLI:

```sh
cargo install tauri-cli --version "^2.0.0" --locked
cargo tauri dev     # run in development
cargo tauri build   # build installers into target/release/bundle/
```

## Contributing

- Trunk-based development on `main`; releases are cut by pushing a `vX.Y.Z` tag ([ADR 0003](docs/adr/0003-trunk-based-development-and-tag-releases.md)).
- Use [Conventional Commits](https://www.conventionalcommits.org): small commits, one concern each ([ADR 0004](docs/adr/0004-conventional-commits-and-semver.md)).
- Architecture decisions live in [`docs/adr`](docs/adr).
- Release notes are published on GitHub Releases; there is no changelog file.

## License

[GNU AGPL-3.0-or-later](LICENSE) ([ADR 0009](docs/adr/0009-agpl-license-and-bundled-lila-translations.md)).

## Credits

Translations come from [lila](https://github.com/lichess-org/lila) (`translation/dest`), the software behind lichess.org. They are copyright the [lila authors](https://github.com/lichess-org/lila/graphs/contributors) and the Lichess translators, and licensed under the [GNU AGPL-3.0-or-later](https://github.com/lichess-org/lila/blob/master/COPYING.md).
