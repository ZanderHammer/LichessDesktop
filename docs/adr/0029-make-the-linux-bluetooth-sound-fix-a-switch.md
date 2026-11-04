---
status: accepted
date: 2026-11-04
---

# Make the Linux Bluetooth sound fix a switch, off by default

## Context and Problem Statement

ADR 0026 sets `pulse.min.req=7200/48000` for everyone on Linux. That keeps sound going on Bluetooth, but every sound comes about 0.5 s late on every output, which matters in fast games, and only an environment variable could remove it. The app can't tell which output is in use, as ADR 0026 explains, so it can't decide this by itself.

## Considered Options

* A Settings switch for the fix, off by default
* A Settings switch for low delay, off by default, which keeps ADR 0026's behaviour for anyone who doesn't touch it
* Keep the fix for everyone and document the variable

## Decision Outcome

Chosen option: "A Settings switch for the fix, off by default", because the delay then only reaches people who need the fix and choose it. The switch is shown on Linux only, as macOS and Windows don't use GStreamer.

The setting has to be applied before any other thread starts, because it changes the environment. The settings store can't open that early, so `audio::configure` reads the store's file itself, from the XDG data directory. Anything unreadable counts as off. A change takes effect at the next start.

The values, the measurements and the way the setting reaches WebKit stay as in ADR 0026, which this replaces. ADR 0028 still applies while the switch is on.

### Consequences

* Good, because nobody gets the delay unless they turn it on.
* Bad, because sound cuts out on Bluetooth by default, about a second into a stream, until the user finds the switch. Anyone who has run a build with the ADR 0026 setting loses it on update.
* Bad, because `audio.rs` repeats the app identifier and where the store keeps its file. A test checks the identifier against the config.
