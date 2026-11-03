---
status: accepted
date: 2026-11-03
---

# Always-on tray icon, with tray-icon's ksni backend on Linux

## Context and Problem Statement

On Linux and Windows the app has no menu bar (ADR 0010), so the tray icon is the only mouse path to app actions (issue #12). A left click should toggle the window and a right click should open the menu, on every platform. We also add a `close_to_tray` setting, off by default, that hides the window on close instead of quitting. On Linux this means choosing a tray backend and deciding what the packages depend on.

Findings (tauri 2.12.5, tauri-cli 2.12.4, tray-icon 0.25.1, ksni 0.3.7, checked on Fedora 44 / KDE Plasma, Wayland):

* Tauri builds tray-icon with its default Linux backend, libappindicator. It emits no click events, and the icon opens its menu on any click ([Tauri system tray guide](https://v2.tauri.app/learn/system-tray/#listen-to-tray-events)). It also logs `libayatana-appindicator is deprecated` on every start.
* `libappindicator-sys` loads `libayatana-appindicator3.so.1`, then `libappindicator3.so.1`, with `dlopen` at runtime and **panics if neither is present** ([source](https://docs.rs/crate/libappindicator-sys/0.9.0/source/src/lib.rs)).
* tray-icon 0.25 also has a `ksni` backend: a StatusNotifierItem over D-Bus, without GTK ([changelog](https://github.com/tauri-apps/tray-icon/blob/dev/CHANGELOG.md#0250)). If the `ksni` feature is on, it takes precedence over libappindicator ([`platform_impl/mod.rs`](https://docs.rs/crate/tray-icon/0.25.1/source/src/platform_impl/mod.rs)). The panel's `Activate` (left click) arrives as `TrayIconEvent::Click` with `MouseButton::Left`, and `ItemIsMenu` is false, so KDE opens the menu on right click ([`ksni/mod.rs`](https://docs.rs/crate/tray-icon/0.25.1/source/src/platform_impl/ksni/mod.rs)). Tauri has no feature for it, but a direct Linux dependency on `tray-icon` with `features = ["ksni"]` turns it on through Cargo feature unification. ksni uses zbus, which `tauri-plugin-opener` and `tauri-plugin-single-instance` already pull in.
* In tray-icon 0.25.1 the ksni backend fails to create the icon when no `org.kde.StatusNotifierWatcher` is on the session bus, for example on GNOME without the AppIndicator extension. 0.26.0 fixes that, but tauri 2.12.5 (the latest 2.x) pins tray-icon 0.25. Because our setup hook returned that error, the app panicked at startup (reproduced with `dbus-run-session`).
* tauri-cli handles packaging whenever the `tray-icon` feature is on, whatever the backend ([`interface/rust.rs`](https://docs.rs/crate/tauri-cli/2.12.4/source/src/interface/rust.rs)). It asks pkg-config for `ayatana-appindicator3-0.1`, then for `appindicator3-0.1`, and panics if it finds neither. The match goes into the deb `Depends` and the rpm `Requires` and is copied into the AppImage. Tauri's [prerequisites](https://v2.tauri.app/start/prerequisites/) list `libayatana-appindicator3-dev` for Debian/Ubuntu, and `libayatana-appindicator-gtk3` provides the soname on Fedora 44.
* Tested on KDE Plasma over D-Bus: the icon registers with Title "Lichess Desktop", left click (`Activate`) toggles the window, and the dbusmenu holds Show/Hide, a separator and Quit. Also tested: close-to-tray, Quit while hidden, relaunching, and no watcher on the bus. GNOME shows StatusNotifierItems only with the AppIndicator extension. Ubuntu enables it by default, vanilla GNOME does not. Not run on GNOME.
* The window-state plugin saves and restores `visible` by default ([source](https://docs.rs/crate/tauri-plugin-window-state/2.5.0/source/src/lib.rs)). It saves on `RunEvent::Exit`, so a Quit from the tray while the window is hidden would start the next launch with no window.

## Considered Options

* tray-icon's ksni backend on Linux, Show/Hide also in the menu
* tray-icon's default libappindicator backend, Show/Hide only in the menu on Linux
* A separate StatusNotifierItem implementation (the `ksni` crate directly) next to Tauri's tray on Windows and macOS
* Tray only on Windows and macOS

## Decision Outcome

Chosen option: "tray-icon's ksni backend on Linux, Show/Hide also in the menu". It gives Linux the same left click and right click as Windows and macOS, through Tauri's own tray API. It doesn't load the deprecated libayatana and needs no second tray implementation.

* The tray icon is always shown when the desktop has a tray. A left click toggles the window and a right click opens the menu. The menu has Show/Hide and Quit, and gains Command palette and Settings once those exist. Show/Hide stays in the menu for panels that only show menus.
* `src-tauri/Cargo.toml` has a Linux-only `tray-icon` dependency with `default-features = false, features = ["ksni"]`. It must stay on the tray-icon version tauri uses.
* If the tray icon can't be created, the app logs the error and runs without one. Closing then always quits, even with `close_to_tray` on, so a window can't be hidden with no way back.
* The CI and release runners still install `libayatana-appindicator3-dev`, because tauri-cli needs an appindicator library to bundle. The app doesn't load it.
* Closing the window quits unless `close_to_tray` is on. Quit in the tray menu always quits. On macOS a Dock click brings back a hidden window.
* The window-state plugin doesn't track visibility (`StateFlags::all() - StateFlags::VISIBLE`). The app shows the window itself after the plugin restores its size and position.
* Like the rest of the native shell, the tray reads nothing from the page and gives it no IPC (ADR 0005, ADR 0008).

### Consequences

* Good, because left click and right click behave the same on all three platforms.
* Good, because the deprecation warning and the libappindicator `dlopen` panic are gone. A missing tray no longer stops the app.
* Bad, because the deb and rpm still depend on libayatana-appindicator, and the AppImage still ships it, although nothing loads it. tauri-cli adds that dependency for the `tray-icon` feature.
* Bad, because with tray-icon 0.25.1 the icon doesn't appear if the panel starts after the app. This matters once the app can autostart. tray-icon 0.26 fixes it, once tauri allows that version.
* Bad, because on GNOME without the AppIndicator extension there is no tray icon, so close-to-tray has no effect there.
