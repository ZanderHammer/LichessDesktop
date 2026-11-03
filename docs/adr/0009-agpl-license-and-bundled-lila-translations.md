---
status: accepted
date: 2026-11-03
---

# AGPL-3.0-or-later license and bundled lila translations

## Context and Problem Statement

ADR 0006 chose MIT and said we bundle no Lichess source or assets. We now want to bundle the lichess translations from lila (`translation/dest/*`): to detect "your turn" in the page title in every language, and to translate our own UI strings. lila's [`COPYING.md`](https://github.com/lichess-org/lila/blob/master/COPYING.md) puts every file that is not listed as an exception under the [GNU Affero General Public License, version 3 or later](https://www.gnu.org/licenses/agpl-3.0.html), copyright the lila authors. The translations are not listed as an exception. Shipping AGPL-licensed files inside an MIT-licensed app would not be compatible, so the app as a whole has to be AGPL.

This ADR supersedes ADR 0006. Name and branding do not change.

## Considered Options

* Relicense to AGPL-3.0-or-later and bundle the lila translations
* Stay MIT and write our own translations and turn-detection strings
* Stay MIT and fetch the translations from lichess.org at runtime

## Decision Outcome

Chosen option: "Relicense to AGPL-3.0-or-later and bundle the lila translations", because it reuses translations that already cover every Lichess language and match the strings the site shows, and AGPL is the license of the project we wrap.

* License: AGPL-3.0-or-later (SPDX `AGPL-3.0-or-later`) in `LICENSE`, `src-tauri/Cargo.toml` and `bundle.license` in `src-tauri/tauri.conf.json`.
* We bundle translation strings from lila `translation/dest/*`, copyright the lila authors and translators, under AGPL-3.0-or-later. The README credits them. Apart from these strings we bundle no Lichess source or assets, and the app still loads lichess.org at runtime.
* Name: "Lichess Desktop", with an original pawn icon (`src-tauri/app-icon.svg`). The Lichess logo is not used.
* The README states that the project is unofficial and not affiliated with or endorsed by Lichess.

### Consequences

* Good, because we get complete and correct translations without maintaining them ourselves.
* Good, because the license matches lila's, so we can reuse more of its AGPL files later.
* Bad, because the license is copyleft: anyone who distributes a modified version must publish its source under the AGPL.
* Bad, because if Lichess objects to the name, we still rename via a superseding ADR.
