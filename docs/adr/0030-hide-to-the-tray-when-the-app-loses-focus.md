---
status: accepted
date: 2026-11-05
---

# Hide to the tray when the app loses focus

## Context and Problem Statement

Some users want the window out of the way as soon as they click elsewhere, with the tray icon as the way back. Tauri reports this as [`WindowEvent::Focused(false)`](https://docs.rs/tauri/2.12.5/tauri/enum.WindowEvent.html), but the same event also fires when the focus moves to our own palette or settings window, and a click on the tray can take the focus before it is reported as a click (see `toggle_main_window`). Hiding on every such event would make those windows and the tray unusable.

## Considered Options

* Hide at once on every focus loss of the main window
* Wait briefly, then hide if no window of the app has the focus

## Decision Outcome

Chosen option: "Wait briefly", because only then can the app tell a move to its own window from a move away from the app. The setting `hide_on_focus_loss` is off by default, like `close_to_tray` (ADR 0014), so an update doesn't change how the window behaves.

* **Delay.** 150 ms after the focus goes, the main window hides unless a window of the app has the focus.
* **Palette.** It closes when it loses focus, so the same check runs for it. Leaving the app from the palette hides both.
* **Settings.** While the settings window is open the main window stays up. It is a child of the main window, and someone reading it next to another app doesn't expect it to go away.
* **Tray click.** For 500 ms after the focus goes, a tray click hides the window instead of bringing it back, so a click that took the focus doesn't undo itself.
* **No tray.** Without a tray icon the setting does nothing, so a window can't be hidden with no way back, as in ADR 0014.

Tested in a private `kwin_wayland --virtual` session with a stand-in `StatusNotifierWatcher`, focus moved by KWin scripts and the tray driven over D-Bus: all the cases above, with the setting on and off. Not run on X11, Windows or macOS.

### Consequences

* Good, because the setting takes effect at once and costs nothing while it is off, beyond reading it on a focus loss.
* Bad, because Alt+Tab can't bring a hidden window back: only the tray, the Dock on macOS or a second launch can.
* Bad, because a native dialog the page opens, such as a file chooser, also takes the focus, and the page behind it hides. Tauri doesn't list these dialogs as windows of the app.
* Bad, because leaving the app from the settings window leaves the main window up.
