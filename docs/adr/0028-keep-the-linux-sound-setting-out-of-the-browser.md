---
status: accepted
date: 2026-11-04
---

# Keep the Linux sound setting out of the browser

## Context and Problem Statement

ADR 0026 sets `pulse.min.req` through `PULSE_PROP` for the app's whole process, because WebKitGTK's web process can only be reached through the environment it inherits. Programs the app starts inherit it too. If no browser is running, a link opened from the app can start one with the larger request size, which then applies to all its sound until it closes. This adds to ADR 0026, whose setting stays.

## Considered Options

* Start `xdg-open` from the app with the variable as the user had it
* Remove the variable after startup
* Add the `open` crate as a direct dependency and use its list of launchers

## Decision Outcome

Chosen option: "Start `xdg-open` from the app", because removing the variable would miss web processes that WebKit starts later, and the one launcher that nearly every desktop has doesn't justify a new direct dependency. `audio::open_url` restores the original value. If `xdg-open` can't be started, the app falls back to `tauri-plugin-opener`.

### Consequences

* Good, because a browser started from the app isn't affected by a fault it doesn't have.
* Bad, because `gio` and the other launchers of the opener are only reached if `xdg-open` is missing, not when it runs and fails.
* Bad, because the opener's fallback still passes the setting on.
