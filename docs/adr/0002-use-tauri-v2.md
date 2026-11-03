---
status: accepted
date: 2026-11-03
---

# Use Tauri v2 (pure Rust, system webview) as the app framework

## Context and Problem Statement

Lichess Desktop shows lichess.org in its own window, independent of the user's browser. The main quality goal is low resource usage, especially RAM. It must build for Windows, macOS and Linux.

## Considered Options

* Electron
* Tauri v2 with a JS frontend toolchain
* Tauri v2, pure Rust (no Node, remote URL only)
* Raw `wry` + `tao` (the libraries Tauri is built on)

## Decision Outcome

Chosen option: "Tauri v2, pure Rust", because it uses the OS webview instead of shipping Chromium:

* WebView2 on Windows
* WKWebView on macOS
* WebKitGTK on Linux

It also provides the bundler (deb, rpm, AppImage, msi, nsis, app, dmg), plugins (opener, window-state) and a GitHub Action (`tauri-apps/tauri-action`). Node is only needed for JS frontends ([prerequisites](https://v2.tauri.app/start/prerequisites/)), and we load `https://lichess.org` directly, so the toolchain is Rust only.

Raw `wry` would mean rebuilding the bundling and release tooling ourselves for little RAM gain.

### Consequences

* Good, because binaries and installers are small. Measured on Fedora 44 x86_64: release binary 4.6 MB, `.deb`/`.rpm` about 1.9 MB.
* Good, because there is a single language and toolchain.
* Bad, because RAM savings depend on the platform. On Linux, WebKitGTK is not dramatically lighter than Chromium; see [tauri-apps/tauri#5889](https://github.com/tauri-apps/tauri/issues/5889). Measured with the lichess.org home page loaded (release build, PSS): main process 64 MB, network process 47 MB, web content 204 MB, about 315 MB in total.
* Bad, because rendering differs per OS webview, and old system webviews can lag behind.
* Bad, because the Linux AppImage bundles WebKitGTK and is about 110 MB.
* Neutral: we re-measure RAM per platform when behaviour changes, rather than relying on marketing numbers.
