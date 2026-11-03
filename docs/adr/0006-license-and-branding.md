---
status: accepted
date: 2026-11-03
---

# MIT license and unofficial "Lichess Desktop" branding

## Context and Problem Statement

The project is open source. The [Lichess terms of service](https://lichess.org/terms-of-service) say nothing about third-party apps using the name or logo, and we found no published brand policy.

## Considered Options

* Name "Lichess Desktop", our own icon, clearly marked unofficial
* Neutral name without "Lichess"
* Use the Lichess logo

## Decision Outcome

* License: MIT.
* Name: "Lichess Desktop", with an original pawn icon (`src-tauri/app-icon.svg`). The Lichess logo is not used.
* The README states that the project is unofficial and not affiliated with or endorsed by Lichess.
* We bundle no Lichess source or assets. The app only loads lichess.org at runtime.

### Consequences

* Good, because the license is permissive and the name is recognisable.
* Bad, because if Lichess objects to the name, we rename via a superseding ADR.
