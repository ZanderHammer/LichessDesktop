---
status: accepted
date: 2026-11-04
---

# Import translations from lila and translate the rest with Claude

## Context and Problem Statement

ADR 0015 set up one JSON file per lichess language, and ADR 0009 lets us bundle lila's translations. Every language we offer now needs every string (#19), without Weblate or another external platform, so we need a way to import what lila has, fill in what it doesn't, and keep it complete.

Checked against lila commit [`43e84ade40`](https://github.com/lichess-org/lila/tree/43e84ade40):

* English is in `translation/source/<module>.xml` and there is no `translation/dest/<module>/en-GB.xml`. The other languages are in `translation/dest/<module>/<tag>.xml`, and a file only has the strings translators have done.
* The files are Android string resources: `\"` and `\'` are escaped, and a few strings have stray spaces (`ne-NP`'s `site.puzzles`).
* Of our 26 shared strings, lila has 1914 of the 2132 translations for the 82 languages we offer other than `en-GB`, a gap of 218. The most common gaps are `preferences.generalSettings` and `site.thereAreNoResultsForX`.
* [`roxmltree`](https://docs.rs/roxmltree/0.21.2/roxmltree/) 0.21.2 is a small read-only XML parser. It resolves entities such as `&amp;`, but not Android's backslash escapes.

## Considered Options

* A Rust tool in the workspace that merges lila's strings into the committed files
* A Python or shell script
* Keep lila's strings and our own in separate files and combine them at build time

## Decision Outcome

Chosen option: "A Rust tool in the workspace", because the project is pure Rust and the tool then gets the same `fmt`, `clippy` and test checks as the app. One file per language stays what ADR 0015 describes, and the build needs no extra step.

* **Shared or ours.** A string is `<lila module>.<lila key>` if lila has the same text with the same meaning, and `desktop.<key>` otherwise. "About" stays ours although lila has `site.about`: that one is about the website (Russian "О сайте", "about the site"), and ours is the app's About tab.
* **Languages.** We offer 83 of lichess's 94 languages, which changes the list in ADR 0015. We leave out Avar, Chuvash, Kabyle, Lojban, Malagasy, Pali, Pashto, Rusyn, Somali, Toki Pona and Zulu: lila has few of our strings for most of them, and we can't translate the rest reliably because these languages have little written material to go on. A wrong translation is worse than English here, so a system locale in one of them falls through to the next locale or English. A language can return when a speaker reviews its strings.
* **Import.** `tools/import-lila` reads the keys from `en-GB.json`. English shared strings come from `translation/source`, and a key lila's source doesn't have as a plain string is an error. For the other languages, lila's text wins, then the text already in the file, and anything else is listed as missing. Keys that aren't in `en-GB.json` are dropped. Running it twice changes nothing.
* **The rest.** Strings lila doesn't have were translated by Claude and committed: the 34 `desktop.*` strings for 81 languages (2754) and the 218 gaps. They follow the terms and scripts lila's translators use for that language, such as the Turkish-style Latin spelling of `uz-UZ`. Once lila gets a translation for a gap, the next import replaces ours.
* **Not translated by hand.** `en-US` uses the English `desktop.*` strings. "Lichess TV" stays a brand name in every language. `desktop.settingsMenu` is the language's "Settings" plus "…".
* **Coverage.** Tests in `src-tauri/src/i18n.rs` fail when a language has no file, lacks a key, has an empty string, or has different placeholders than English. CI runs them on every PR. The fallback to English stays as a safety net at runtime.

### Regenerating

```sh
git clone --depth 1 --filter=blob:none --sparse https://github.com/lichess-org/lila.git
git -C lila sparse-checkout set translation
cargo run -p import-lila -- lila
```

Name the lila commit in the commit message. The tool prints the strings that are still missing per language.

* **New string.** Add it to `keys!` in `src-tauri/src/i18n.rs` and to `en-GB.json`, with lila's key if lila has it. Run the import, then translate what it lists.
* **Changed English text of a `desktop.*` string.** Delete the key from every language file except `en-GB.json`, so the import lists it again. Otherwise the old translations stay.
* **New language.** Add it to `i18n::LANGUAGES`, create `src-tauri/i18n/<tag>.json` containing `{}`, and run the import.

### Consequences

* Good, because no language has a missing key, and a new key fails CI until every language has it.
* Good, because the tool stays out of the app: it isn't a dependency of `lichess-desktop` and isn't bundled.
* Good, because a correction by hand to a `desktop.*` string survives every import.
* Bad, because speakers of the 11 languages we leave out get English, though lichess itself offers their language.
* Bad, because no native speaker has reviewed the Claude translations.
* Bad, because the English text of a `desktop.*` string can change without its translations noticing. Nothing detects this.
* Bad, because 83 files add about 260 KB to the binary. Only the UI language and English are parsed at runtime.
* Bad, because lila's own quirks come along, such as a trailing full stop in `ckb-IR`'s `broadcast.broadcasts`. They should be fixed in lila's translations, not here.
