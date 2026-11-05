---
status: accepted
date: 2026-11-05
---

# A tray switch for hiding on focus loss

## Context and Problem Statement

With `hide_on_focus_loss` on (ADR 0030), a window that the user set to "keep above" in Plasma still hides when it loses focus, though it should stay in view. The natural fix is to skip hiding when the window is on top, but the app can't see that state. [`xdg_toplevel`'s states](https://wayland.app/protocols/xdg-shell) have no entry for it. Tested in a private `kwin_wayland --virtual` session (KWin 6.8.2) with `keepAbove` set by a KWin script, `is_always_on_top` stayed false on Wayland and also on X11 through Xwayland.

## Considered Options

* Ask KWin for the state through a script or a private protocol
* Run the app on X11 to learn the state
* Respect what the app can see, and put the setting in the tray menu

## Decision Outcome

Chosen option: "Respect what the app can see", because it is the only option that works without tying the app to one compositor. Running on X11 wouldn't help, as the test above shows.

* **Tray menu.** A check item shows and switches `hide_on_focus_loss`, with the string the settings window already uses. The menu is rebuilt from the setting after a change on either side, and an open settings window reloads.
* **Always on top.** The window stays up while `is_always_on_top` is true, which is when the app set it itself with the "always on top" action. On X11 this keeps the window up. On Wayland the action has no effect and the flag stays false.
* **Timing.** A panel may report the menu as closed before the click on the item. The delayed hide therefore reads the setting again, or switching it off would still hide the window.

Tested in the same session with a stand-in `StatusNotifierWatcher` and the tray driven over D-Bus. Not tried on a real panel, on Windows or on macOS. The reload of an open settings window wasn't run either.

### Consequences

* Good, because switching the behavior off for a while takes two clicks and no window.
* Bad, because the setting and Plasma's "keep above" still conflict until the user switches the setting off.
* Bad, because hiding unmaps the window, so Plasma forgets "keep above" when it comes back, unless a KWin window rule sets it.
* Bad, because the tray menu is wider: some translations of this string are long.
