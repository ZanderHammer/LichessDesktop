---
status: accepted
date: 2026-11-03
---

# Command palette as a short-lived frameless child window

## Context and Problem Statement

Linux and Windows have no menu bar (ADR 0010), so the command palette (#16) is how the keyboard reaches every app action and the common lichess pages. Ctrl/Cmd+K opens it over the main window, and Esc or losing focus closes it. We had to decide where it lives, how long its window lives, and what it may ask Rust to do. The product brief puts low memory use first (PRODUCT.md).

Checked against tauri 2.12.5, tao 0.37.2 and the Tauri docs:

* An overlay inside the lichess.org page would need injected script, which [ADR 0008](0008-no-code-injection-into-lichess-org.md) rules out. The palette has to be its own bundled window with scoped IPC ([ADR 0012](0012-scoped-ipc-for-bundled-app-windows.md)).
* Tao can't read or set window positions on Wayland (`Window::outer_position` / `set_outer_position`: "Linux(Wayland): Unsupported", tao `src/window.rs`). The palette can only be centred by hand on X11, Windows and macOS. On Wayland the compositor places it.
* Building a window from a synchronous command or event handler deadlocks on Windows (tauri `WebviewWindowBuilder::new` docs, [wry#583](https://github.com/tauri-apps/wry/issues/583)), and commands without `async` run on the main thread ([Calling Rust](https://v2.tauri.app/develop/calling-rust/)). Running an entry can open the settings window.

## Considered Options

* A new frameless child window for every open, destroyed on close
* One palette window built at startup (or on first use) and hidden between uses
* An overlay inside the lichess.org page

## Decision Outcome

Chosen option: "A new frameless child window for every open, destroyed on close". A closed palette uses no webview memory, and every open starts with fresh strings and an empty query, so the window needs no event listeners and no reset logic. The overlay breaks ADR 0008. A hidden window would open a bit faster, but it would keep a webview alive for the rest of the session.

* **Window.** Label `palette`, `ui/palette.html`, 600×360, no decorations, not in the taskbar, not resizable, a child of the main window. It is placed in the middle of the main window, worked out from the main window's outer position and size. Where that isn't available (Wayland) the compositor places it. It stays hidden until the page reports `palette_ready`, with a 1.5 s fallback as in the settings window. It closes on Esc, on Ctrl/Cmd+K, on `WindowEvent::Focused(false)` and after an entry runs.
* **Entries.** The app actions in `Action::ALL` except the palette itself, each with its first shortcut for this platform, then a fixed list of lichess pages in `palette::DESTINATIONS` (taken from lila's top menu, plus the account pages that need no username). Labels are i18n keys, lila keys where lichess has the string.
* **Commands.** `list_actions`, `run_action`, `palette_ready` and `close_palette`, plus the shared `get_strings`. They are granted only in `capabilities/palette.json`, with no event permissions. `run_action` takes an entry id, never a URL. An action id runs the same function as its shortcut. A page id opens the page's fixed URL in the main window, and only if `navigation::is_internal` accepts it. `run_action` is `async`, so it runs off the main thread and can open the settings window on Windows.
* **Page.** The search field is a WAI-ARIA combobox over a listbox, with `aria-activedescendant` for the selection. Focus never leaves the field. Fuzzy matching (`ui/fuzzy.js`) ignores case, accents and spaces and ranks word starts and adjacent letters first.

### Consequences

* Good, because the palette costs no memory while it is closed, and its window can't hold stale strings or state.
* Good, because lichess.org stays untouched, and the palette page can only run fixed entries by id.
* Bad, because every open builds a webview, so the palette appears after a short delay. Keys typed before it shows still go to the main window.
* Bad, because on Wayland the compositor decides where the palette goes, and a compositor that refuses it focus leaves it open until it is clicked or Esc is pressed in it.
* Bad, because Windows placement and focus behaviour are taken from the docs and still need a run on Windows.
