---
status: proposed
date: 2026-11-03
---

# Window-level keyboard shortcuts instead of hidden menu accelerators

## Context and Problem Statement

We don't show a menu bar on Linux and Windows. ADR 0008 rules out injected code, so the app can only get keyboard shortcuts on the native side. The plan was to attach a muda menu, call `WebviewWindow::hide_menu()` and rely on its accelerators. Spike #4 checked this on Fedora 44, KDE Plasma (Wayland session), with tauri 2.12.5, muda 0.20.0 and wry 0.57.1. A throwaway harness fed real `GdkEventKey`s through `gtk_main_do_event` into the window, then logged menu events and the page's `keydown` events.

Findings:

* **Linux (GTK): accelerators stop working once the menu is hidden.** muda registers each accelerator with `gtk_widget_add_accelerator("activate", …)` on the menu item. `hide_for_gtk_window` only calls `gtk::MenuBar::hide()` ([muda source](https://docs.rs/crate/muda/0.20.0/source/src/platform_impl/gtk/mod.rs)). GTK only activates a widget's accelerator while the widget is drawable (visible and mapped), so after `hide_menu()` none of the 14 tested shortcuts fired. The keys went to the page instead. With the menu visible, all 14 fired.
* A zero-height menubar is not a workaround. GTK keeps the menubar's minimum height, so an empty bar stays visible.
* A `gtk::AccelGroup` with closure accelerators (`AccelGroup::connect_accel_group`) on the `gtk::ApplicationWindow` needs no menu widget. All 14 shortcuts fired. The webview never saw them, because `GtkWindow` activates accelerators before it passes a key to the focused widget. This toolkit path is the same for X11 and Wayland, but only Wayland was run.
* **Windows: menu accelerators don't fire while WebView2 has focus, whether the menu is hidden or not.** Tauri runs `TranslateAcceleratorW` in the host thread's message hook ([tauri `app.rs`](https://docs.rs/crate/tauri/2.12.5/source/src/app.rs)). WebView2's input window belongs to the browser process, so the host never gets those keys. Known upstream: [wry#451](https://github.com/tauri-apps/wry/issues/451), open since 2021. `SetMenu(NULL)` in `hide_for_hwnd` doesn't affect the accelerator table either way. WebView2 has a host-side hook for this, [`ICoreWebView2Controller::add_AcceleratorKeyPressed`](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2controller#add_acceleratorkeypressed). It fires for every key pressed with Ctrl or Alt and for non-character keys (F-keys, Esc). It runs before web content and browser accelerators, and `put_Handled(TRUE)` stops the key there. Tauri exposes the controller through `PlatformWebview::controller()`, and `webview2_com::AcceleratorKeyPressedEventHandler` (0.39.1, already in the lockfile) wraps the callback. Not yet run on Windows.
* **macOS:** the standard app menu stays visible, and AppKit resolves menu key equivalents before the webview. No change is needed there. Not run in the spike.
* **Mouse buttons 4/5 (X11 buttons 8/9).** WebKitGTK does nothing with them natively. wry catches them and calls `run_javascript` ([`synthetic_mouse_events.rs`](https://docs.rs/crate/wry/0.57.1/source/src/webkitgtk/synthetic_mouse_events.rs)). That JavaScript dispatches untrusted `mousedown`/`mouseup` events (`button` 3/4) into the page and then calls `history.back()`/`forward()`. The harness recorded these events with `isTrusted === false` on the page. This is code injection into lichess.org that ADR 0008 rules out, and it is on by default. A handler on the webview's GTK `event` signal runs before `button-press-event`. Our handler calls `WebView::go_back()`/`go_forward()` and returns `Stop`, so wry's handler never runs. Back and forward worked, and no event reached the page. WebView2 is Chromium, which handles XButton1/2 as back/forward natively. Not yet run on Windows.

## Considered Options

* Hidden muda menu accelerators on every platform (the original plan)
* Zero-height or otherwise collapsed menubar on Linux
* `tauri-plugin-global-shortcut`
* Window-level handlers per platform behind one shared shortcut table

## Decision Outcome

Chosen option: "Window-level handlers per platform behind one shared shortcut table". The hidden menu doesn't work on Linux or on Windows, and a collapsed menubar stays visible. Global shortcuts would grab keys system-wide even when the app is in the background, and on Wayland they depend on the compositor's portal.

* One Rust table maps each shortcut to an action function, which the palette and tray reuse (issue #5).
* **Linux:** one `gtk::AccelGroup` on the main `gtk::ApplicationWindow` with a closure per shortcut. The app sets no muda menu on Linux.
* **Windows:** an `AcceleratorKeyPressed` handler on the main webview's controller matches key-down events (`KEY_DOWN`/`SYSTEM_KEY_DOWN`, ignoring auto-repeat) against the table. On a match it calls `put_Handled(TRUE)` and runs the action. The app sets no muda menu on Windows. Keep the handler short: until it returns, the browser process is blocked.
* **macOS:** the standard app menu with muda accelerators, as before.
* **Mouse back/forward on Linux:** a GTK `event` handler on the webview handles buttons 8/9 natively and stops wry's synthetic-event injection. On Windows, keep WebView2's native handling.
* All of this is host-side and reads nothing from the page, so ADR 0005 and ADR 0008 still hold.

Shortcut notes from the spike:

* Window-level shortcuts take precedence over the page, so text fields never receive these keys. None of the planned shortcuts is a text-editing key on Linux or Windows.
* lichess (lila `ui/`, commit 1f0f21b) binds only these modifier shortcuts: `mod+z` (study shape undo), `ctrl+shift+1…8` (study glyphs), and in the screen-reader UI `ctrl+arrows` and `alt+shift+a/d`. None of these collide with Ctrl/Cmd+K, Ctrl+R, F5, Alt+←/→, Alt+Home, Ctrl+Shift+C/O, Ctrl+=/−/0, F11, Ctrl+, or Ctrl+Shift+T. Ctrl/Cmd+K is free.
* On layouts where `=` needs Shift (German: Ctrl+Shift+0), `<Control>equal` still matches. Ctrl plus the `+` key does not, so zoom-in also needs `<Control>plus` and `<Control>KP_Add`.
* Ctrl+Shift+T is free in lichess and was tested, so it's the candidate for always-on-top (issue #7).

### Consequences

* Good, because shortcuts work with no visible menu bar and no code in the page.
* Good, because the wry mouse-button injection on Linux is removed, which ADR 0008 implies.
* Bad, because there are three code paths (GTK, WebView2, macOS menu) with `cfg` blocks, and direct `gtk` and `webview2-com` dependencies that must stay on the versions Tauri uses.
* Bad, because the Windows path and Windows mouse 4/5 handling are taken from the docs and still need a run on Windows before issue #5 is done.
