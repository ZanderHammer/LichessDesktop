---
status: superseded by ADR-0029
date: 2026-11-04
---

# Keep Linux Bluetooth audio with a larger PulseAudio request size

## Context and Problem Statement

On Linux with a Bluetooth output, Lichess sounds were silent. KDE showed a speaker on the window and the stream was unmuted, at 100% and on the right device. The page's `AudioContext` reported `running` and its clock advanced.

WebKitGTK plays Web Audio through GStreamer and asks [`autoaudiosink`](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/platform/audio/gstreamer/AudioDestinationGStreamer.cpp) for a sink, which picks `pulsesink`. WebKit creates that sink itself, so the app can't set its properties. It can only change the environment the web process inherits.

Measured on Fedora 44 with WebKitGTK 2.54.2, GStreamer 1.28.8, PipeWire 1.6.11 and an A2DP SBC output at 48 kHz, by recording the sink's monitor:

* Plain `gst-launch-1.0 audiotestsrc is-live=true ! ... ! pulsesink`, without WebKit, plays about 1 s and then goes silent. `paplay` plays the same tone in full. So this is not specific to the app, to Lichess or to WebKit.
* A stream only plays for about its first second. Lichess stays quiet until a move, so its sounds came too late. A context that plays at the moment it is created is audible, which looked like "only new contexts work".
* The same sound plays on a null sink and on the laptop speakers.
* Sample rate and format make no difference. With `gst-launch`, `sync=false`, `slave-method=none` or `buffer-time=1000000 latency-time=100000` each fix it, and `buffer-time=200000 latency-time=20000` doesn't. So the trigger is the timing of `pulsesink` against the Bluetooth sink. The exact fault in GStreamer or PipeWire is unknown.

Others report the same symptom: [Strata #1409](https://github.com/lgse/strata/issues/1409) and [chopdeck #3](https://github.com/mcinnisdev/chopdeck/pull/3).

## Considered Options

* `pulse.min.req` of PipeWire's Pulse server, set through `PULSE_PROP`
* `GST_PLUGIN_FEATURE_RANK=pipewiresink:MAX`, as Strata and chopdeck do
* `alsasink` through `pipewire-alsa`
* `WEBKIT_GST_ENABLE_AUDIO_MIXER=1`
* `PULSE_LATENCY_MSEC`
* Leave it to the user to add a `pulse.rules` entry to their PipeWire configuration

## Decision Outcome

Chosen option: "`pulse.min.req` set through `PULSE_PROP`", because it was the only option that gave complete and clean sound, and it needs no sink swap, no code in the page (ADR 0008) and no change outside the app.

On Linux, `main` adds `pulse.min.req=7200/48000` (150 ms) to `PULSE_PROP` before anything else runs. Other properties in the variable stay, and a `pulse.min.req` the user already set wins. [`pulse.min.req`](https://docs.pipewire.org/page_module_protocol_pulse.html) is the lower limit for a client's playback request and is 256/48000 by default. libpulse sends [`PULSE_PROP`](https://wiki.freedesktop.org/www/Software/PulseAudio/Documentation/Developer/Clients/ApplicationProperties/) as client properties. PipeWire's page documents the property for its own `pulse.rules`, not this route. That it works this way is measured: the stream's quantum changes, and the sound with it.

A 440 Hz tone through WebKit:

| Setting | Output | Result |
| --- | --- | --- |
| Default | Bluetooth | Under 1 s of sound, then silence |
| `pipewiresink` | Null sink | 98% of the output is not the tone, at a fifth of the level |
| `alsasink` | Bluetooth | Under 1 s of sound, then silence |
| Audio mixer | Null sink | 99.7% of the output is not the tone. On Bluetooth, real Lichess pages stayed mostly silent |
| `PULSE_LATENCY_MSEC=200` | Bluetooth | No effect |
| `pulse.min.req` 1200, 2400 or 3600 | Bluetooth | Cuts out after 0.25 to 2 s |
| `pulse.min.req` 4800 | Bluetooth | Continuous, with gaps in nearly every 100 ms window |
| `pulse.min.req` 7200 | Bluetooth | Continuous, 0.00% distortion, no gaps over 14 s |
| `pulse.min.req` 9600 | Bluetooth | Continuous, no gaps over 14 s, one brief blip |

A 60 s run at 7200 had no gaps but some distorted windows. It may have overlapped a call on the same sink, which also distorted a reference tone played right after it, so it says nothing either way. The user then tried the real app on Bluetooth at 7200 and reported that it worked flawlessly. A debug build with the variable unset passed it on to the web process and played sound on a null sink.

### Consequences

* Good, because sound works on Bluetooth with no setup and no change to the page.
* Good, because the user keeps control through their own `PULSE_PROP`.
* Bad, because every sound comes later on every output: on a null sink, from about 79 ms to about 460 ms. The setting can't tell Bluetooth from other outputs, and the output can change while the app runs.
* Bad, because the value is a measurement from one machine and one Bluetooth device, not a known limit. Another device may need more.
* Bad, because it works around a fault we don't understand. Revisit it when GStreamer or PipeWire change, and when reporting upstream gives an answer.
* Neutral, because macOS and Windows don't use GStreamer, so they don't get the setting. A plain PulseAudio server and the AppImage are untested.

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
