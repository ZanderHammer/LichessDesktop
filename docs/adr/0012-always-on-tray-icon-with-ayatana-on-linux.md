---
status: accepted
date: 2026-11-03
---

# Always-on tray icon, built against libayatana-appindicator on Linux

## Context and Problem Statement

On Linux and Windows the app has no menu bar (ADR 0010), so the tray icon is the only mouse path to app actions (issue #12). We also add a `close_to_tray` setting, off by default, that hides the window on close instead of quitting. Two questions came up: how the tray behaves on Linux, and which Linux library the packages depend on.

Findings (tauri 2.12.5, tauri-cli 2.12.4, tray-icon 0.25.1, checked on Fedora 44 / KDE Plasma, Wayland):

* Linux tray icons emit no click events. The icon still shows its menu on click ([Tauri system tray guide](https://v2.tauri.app/learn/system-tray/#listen-to-tray-events)). A "left click toggles the window" handler only works on Windows and macOS.
* `libappindicator-sys` loads `libayatana-appindicator3.so.1`, then `libappindicator3.so.1`, with `dlopen` at runtime and **panics if neither is present** ([source](https://docs.rs/crate/libappindicator-sys/0.9.0/source/src/lib.rs)). A missing library crashes the app at startup.
* tauri-cli handles packaging when the `tray-icon` feature is on ([`interface/rust.rs`](https://docs.rs/crate/tauri-cli/2.12.4/source/src/interface/rust.rs)). It asks pkg-config for `ayatana-appindicator3-0.1`, then for `appindicator3-0.1`. Whichever it finds goes into the deb `Depends` (`libayatana-appindicator3-1` or `libappindicator3-1`) and the rpm `Requires` (`libayatana-appindicator3.so.1()(64bit)` or `libappindicator3.so.1()(64bit)`), and gets copied into the AppImage under `/usr/lib/`. So the library on the build runner decides what users must install.
* Our runners installed `libappindicator3-dev`, the unmaintained Canonical library. Tauri's [prerequisites](https://v2.tauri.app/start/prerequisites/) list `libayatana-appindicator3-dev` for Debian/Ubuntu. On Fedora 44 `libayatana-appindicator-gtk3` provides the ayatana soname.
* On KDE Plasma the icon registers as a StatusNotifierItem (`/org/ayatana/NotificationItem/tray_icon_tray_app_main`), and its dbusmenu holds Show/Hide, a separator and Quit. Over D-Bus we tested close-to-tray, Show/Hide, Quit and relaunching. GNOME shows StatusNotifierItems only with the AppIndicator extension. Ubuntu enables it by default, vanilla GNOME does not. Not run on GNOME.
* The window-state plugin saves and restores `visible` by default ([source](https://docs.rs/crate/tauri-plugin-window-state/2.5.0/source/src/lib.rs)). It saves on `RunEvent::Exit`, so a Quit from the tray while the window is hidden would start the next launch with no window.

## Considered Options

* Tray built against libayatana-appindicator, Show/Hide in the menu on every platform
* Tray built against libappindicator (the previous runner package)
* Tray only on Windows and macOS
* Tray shown only while `close_to_tray` is on

## Decision Outcome

Chosen option: "Tray built against libayatana-appindicator, Show/Hide in the menu on every platform". Ayatana is the maintained implementation that Tauri recommends, and the same menu works on Linux, which has no click events.

* The tray icon is always shown. A left click toggles the window on Windows and macOS. The menu has Show/Hide and Quit, and gains Command palette and Settings once those exist.
* The CI and release runners install `libayatana-appindicator3-dev`, so the packages depend on ayatana.
* Closing the window quits unless `close_to_tray` is on. Quit in the tray menu always quits. On macOS a Dock click brings back a hidden window.
* The window-state plugin doesn't track visibility (`StateFlags::all() - StateFlags::VISIBLE`). The app shows the window itself after the plugin restores its size and position.
* Like the rest of the native shell, the tray reads nothing from the page and gives it no IPC (ADR 0005, ADR 0008).

### Consequences

* Good, because Linux users get a working Show/Hide and Quit even though Linux has no click events.
* Good, because deb and rpm installs pull in the library, and the AppImage ships it, so the startup panic can't happen from a normal install.
* Bad, because on GNOME without the AppIndicator extension the icon is invisible. With `close_to_tray` on, the only way back to a hidden window is launching the app again: the single-instance plugin shows the running window.
* Bad, because rpm-based distros without a package that provides the ayatana soname can't install the rpm. They can use the AppImage.
