---
status: accepted
date: 2026-11-03
---

# Update notice from the GitHub releases API

## Context and Problem Statement

Releases are GitHub releases made from tags (ADR 0003), and the app has a "Check for updates" setting (on by default). Users should learn that a newer version exists without visiting the repository (issue #17). How does the app find out, and what does it do then?

Findings:

* GitHub's [latest release endpoint](https://docs.github.com/en/rest/releases/releases#get-the-latest-release), `GET /repos/ZanderHammer/LichessDesktop/releases/latest`, returns the newest published release that is neither a draft nor a prerelease, with its `tag_name` (`vX.Y.Z`). It needs no token; [unauthenticated requests](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api) get 60 per hour per IP, and a `User-Agent` header is [required](https://docs.github.com/en/rest/using-the-rest-api/getting-started-with-the-rest-api#user-agent).
* [`tauri-plugin-updater`](https://v2.tauri.app/plugin/updater/) downloads and installs updates, but needs signed update artifacts, a signing key in CI and an update manifest. Linux packages from `deb`/`rpm` are updated by the package manager, not by the app.
* The main window is lichess.org without IPC (ADR 0005), so the check can't run in the page. A Rust HTTP client is needed. [`ureq`](https://docs.rs/ureq/3.5.1/) 3.5 is a small blocking client; with `default-features = false, features = ["rustls", "json"]` it added 14 crates to the lockfile (`rustls`, `ring`, `webpki-roots` and their helpers), plus an older `windows-sys` for `ring` on Windows. `reqwest` would bring an async runtime (`tokio`, `hyper`) for one request per day.
* Native notifications need [`tauri-plugin-notification`](https://docs.rs/tauri-plugin-notification/2.8.0/) 2.8, which wraps `notify-rust` (D-Bus via `zbus`, already in the tree for the tray, ADR 0014) and added 8 crates. Its Rust API needs no capability, so the lichess page still gets none. `Notification::on_action` reports a click on the notification (`tap`).
* Version tags are SemVer. The [`semver`](https://docs.rs/semver/1.0.28/) crate was already in the tree through Tauri.

## Considered Options

* Notify only: query the latest GitHub release from Rust, and point to its release page
* Auto-update with `tauri-plugin-updater`
* No check: users watch the repository's releases themselves

## Decision Outcome

Chosen option: "Notify only", because it needs no signing infrastructure, works the same for every package format, and leaves installing to the user or their package manager. Auto-updating can be looked at again once releases are signed.

* At startup, if "Check for updates" is on and the last successful check is 24 hours old or more, a background thread fetches the latest release with `ureq` (10 s timeout, `User-Agent: LichessDesktop/<version>`, no other identifying data). The result is cached in `update.json` in the app data dir, so a restart within 24 hours uses the cached version and sends no request.
* If the release is newer than the running version (SemVer order), the tray menu, the command palette (once it exists) and the About tab show "Update available: vX.Y.Z", which opens the release page in the browser. The release URL is built from the repository and the parsed version, not taken from the response.
* One native notification is shown per new version; the notified version is stored. Clicking it opens the release page.
* A failed check (offline, rate limit, bad response) is only logged and retried at the next start.
* With "Check for updates" off, nothing is fetched and no update entry is shown.

### Consequences

* Good, because the app contacts only `api.github.com`, at most once a day, and only when the setting is on.
* Good, because a missed or failed check never gets in the user's way.
* Bad, because the binary now carries a TLS stack (`rustls` with `ring`) of its own, next to the webview's.
* Bad, because a running app only checks at startup; an app that stays open for weeks learns about a release at its next start.

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
