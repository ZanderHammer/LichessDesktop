---
status: accepted
date: 2026-11-04
---

# Update status in the About tab, and a check when the setting is turned on

## Context and Problem Statement

The settings window has to apply changes immediately and show the update status in the About tab (#15). [ADR 0017](0017-update-notice-from-the-github-releases-api.md) only checks at startup and only logs a failed check. So "Check for updates" took effect at the next start, and the About tab said nothing unless a newer release was known. This ADR extends ADR 0017 and does not replace it.

## Considered Options

* Apply the setting at once, and keep a status for the About tab
* Show only "Update available" and say the setting applies after a restart
* Add a "Check now" button

## Decision Outcome

Chosen option: "Apply the setting at once, and keep a status for the About tab", because it meets the issue with no new control and no extra requests.

* Turning the setting on starts the same background check as a start of the app. The 24-hour limit and the cached result from ADR 0017 still apply, so turning it off and on again within a day sends no new request once a check has succeeded.
* Turning the setting off removes the update entry from the tray and the About tab at once, as ADR 0017 says for "off". A check still running when that happens has no effect.
* The app keeps one status: off, checking, up to date, failed or available. The About tab always shows it. A failed check is still retried only at the next start or when the setting is turned on again, but it is now shown as well as logged.

### Consequences

* Good, because the setting no longer needs a restart, and "Up to date" tells the user that the check ran.
* Good, because the app still contacts `api.github.com` at most once a day after a successful check.
* Bad, because "Up to date" can be up to 24 hours old.
* Bad, because switching the setting off and on after a failed check sends a new request each time. GitHub's limit of 60 requests per hour per IP still applies.

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
