---
status: accepted
date: 2026-11-03
---

# Native clipboard calls instead of the clipboard plugin

## Context and Problem Statement

The "Copy current URL" action (issue #5) writes one string to the system clipboard from Rust. The page never needs the clipboard API (ADR 0005), so only the host side needs it. Tauri doesn't wrap the clipboard itself. How should the app write to it?

Findings (checked on Fedora 44, KDE Plasma, Wayland):

* [`tauri-plugin-clipboard-manager`](https://docs.rs/tauri-plugin-clipboard-manager/2.5.0/) 2.5.0 is the official plugin. On desktop it wraps [`arboard`](https://docs.rs/arboard/3.7.0/) 3.6 with the `wayland-data-control` feature and its default `image-data` feature. The plugin creates an `arboard::Clipboard` when it is initialised ([`desktop.rs`](https://docs.rs/crate/tauri-plugin-clipboard-manager/2.5.0/source/src/desktop.rs)), and it registers image commands that keep the image code in the binary.
* Adding it to the lockfile brought in 36 new crates, among them `image` 0.25, `tiff`, `zune-jpeg`, `moxcms`, `wl-clipboard-rs`, `wayland-client` and `x11rb`. That is a lot for one string, in an app whose goal is a small binary and low RAM use (PRODUCT.md).
* Each platform's own API already covers this:
  * **Linux:** `gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text()` ([gtk 0.18](https://docs.rs/gtk/0.18.2/gtk/struct.Clipboard.html)). The window is already a GTK window, so this works on X11 and on Wayland with no extra protocol. GTK serves the text while the app runs. Tested: after Ctrl+Shift+C the clipboard held the page URL.
  * **Windows:** `OpenClipboard` with the main window as owner, `EmptyClipboard`, then `SetClipboardData(CF_UNICODETEXT)` with `GlobalAlloc` memory ([Using the Clipboard](https://learn.microsoft.com/en-us/windows/win32/dataxchg/using-the-clipboard)). The owner window matters: after `OpenClipboard(NULL)`, `EmptyClipboard` leaves the clipboard without an owner and `SetClipboardData` fails ([`EmptyClipboard`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-emptyclipboard)). The `windows` 0.62 crate, which tauri already uses, has these behind the `Win32_System_DataExchange`, `Win32_System_Memory` and `Win32_System_Ole` features.
  * **macOS:** `NSPasteboard::generalPasteboard()`, `clearContents()` and `setString_forType(…, NSPasteboardTypeString)` from [`objc2-app-kit`](https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSPasteboard.html) 0.3.2, which wry already uses.

## Considered Options

* `tauri-plugin-clipboard-manager`
* `arboard` directly
* The platform clipboard APIs, in `src-tauri/src/native/`

## Decision Outcome

Chosen option: "The platform clipboard APIs", because writing one string takes a few lines per platform, adds no crates beyond features of ones we already build, and keeps no clipboard object alive in the background. Using `arboard` directly would pull in the same crates as the plugin unless its default features were turned off, and it would still be a second clipboard stack next to GTK's on Linux.

* `native::copy_text` runs on the main thread (GTK requires it). Actions call it through `AppHandle::run_on_main_thread`.
* If we ever need to read the clipboard or copy images, look at this again.

### Consequences

* Good, because the binary and the dependency tree stay small, and nothing clipboard-related runs until the user copies.
* Good, because on Linux the text goes through the toolkit the window already uses, on X11 and Wayland.
* Bad, because there are three small `unsafe`/FFI code paths to maintain. The Windows and macOS ones are not yet tested on a real machine.

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
