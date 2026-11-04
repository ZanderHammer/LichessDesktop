# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

The app is a Tauri v2 desktop app for Windows, macOS and Linux. Its own UI is bundled HTML/CSS/JS that runs in the OS webview (WebView2, WKWebView, WebKitGTK).

## Stack

Plain static HTML, CSS and JavaScript, served as bundled assets (`frontendDist`). No framework, no build step and no Node, so the repo stays Node-free (issue #14). Windows, IPC and settings live in Rust (`src-tauri/`).

## Users

Regular lichess players who play often and want lichess as an app of its own rather than a browser tab. That means a taskbar/dock entry, a window that doesn't get lost among other tabs, your-turn notifications and low RAM use.

## Product Purpose

Lichess Desktop is a lightweight, unofficial desktop wrapper for lichess.org. It shows lichess.org exactly as served, in its own window, and adds what a browser tab can't do well: tray, notifications, shortcuts, a command palette, settings. Success means the user forgets the wrapper is there: lichess feels native, and the app's own UI is opened briefly and closed.

## Positioning

It uses the OS webview instead of bundling a browser engine, so it stays small (ADR 0002). It never touches the page: no injected JavaScript or CSS and no DOM access (ADR 0008), and lichess.org gets no access to native APIs (ADR 0005, ADR 0012). All integrations come from the Rust side (navigation, document title) or the official Lichess API.

## Operating Context

- The main window is lichess.org itself. The app's own surfaces are small bundled windows next to it: settings/about (#15) and the command palette (#16), plus the offline page (`src-tauri/src/offline.html`).
- They are reached by keyboard shortcut (Ctrl/Cmd+, and Ctrl/Cmd+K), the tray menu or the palette. Use is short: open, change or run one thing, close.
- On Linux and Windows there is no menu bar (ADR 0010). The tray and the palette are the only mouse paths to the app's UI.

## Capabilities and Constraints

- Bundled windows get scoped IPC only through per-window capabilities and a small set of typed commands (ADR 0012). Pages call `__TAURI_INTERNALS__.invoke` through a single `ipc.js` wrapper. `withGlobalTauri` stays off.
- A strict CSP applies to bundled assets: no inline scripts, no remote loads.
- All UI strings go through i18n, with 83 of the lichess languages supported (ADR 0020) and English as the fallback (#18, #19). Strings come from Rust.
- Follows the OS light/dark theme.
- Planned settings: close to tray, check for updates, language, notification toggles, and later a Lichess account connection (v0.5).

## Brand Commitments

- Name: "Lichess Desktop", always presented as **unofficial**, "not affiliated with Lichess" (ADR 0009).
- Icon: an original pawn (`src-tauri/app-icon.svg`). The Lichess logo is never used.
- The app's own UI must not look like lichess's UI.
- Character: an invisible utility. It should feel like part of the OS, with lichess as the star.
- Visual standard: native OS settings, played straight (chosen 2026-11-03). Quality bar: GNOME Settings (libadwaita) and Windows 11 Settings.
- Accent: the OS accent colour, read by Rust. Until that exists, a native blue fallback. CSS `AccentColor` can't be used: WebKit and Firefox return a fixed blue, and Chromium returns the real colour only for installed web apps (MDN browser-compat-data).
- License: AGPL-3.0-or-later. Translations come from lila and are credited in the README.

## Evidence on Hand

- `README.md`, ADRs in `docs/adr/`, roadmap issues #5 to #25 (milestones v0.2 to v0.5).
- App icon: `src-tauri/app-icon.svg` and `src-tauri/icons/`.
- No user research, testimonials or usage numbers exist. Don't invent any.

## Product Principles

1. **lichess is the product.** The wrapper adds native affordances and never changes or competes with the site.
2. **Invisible over expressive.** App surfaces feel native and quick, and get out of the way.
3. **Light by default.** Small binary, low RAM, no framework, no build step.
4. **Least privilege.** Remote content gets no native access. Bundled windows get only the commands they need.
5. **Keyboard first.** Every app action has a shortcut and is reachable from the palette.

## Accessibility & Inclusion

WCAG 2.2 AA for all bundled windows: contrast, visible focus, full keyboard operation, and labelled controls for screen readers. They also respect `prefers-reduced-motion` and OS high-contrast/forced-colors modes. They must work in every language the app offers, including right-to-left ones.
