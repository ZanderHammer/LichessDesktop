---
status: accepted
date: 2026-11-03
---

# UI strings as one JSON file per lichess language, with lila's keys

## Context and Problem Statement

All app strings (tray, offline page, settings/about, palette, notifications) must be translated into every language lichess offers, with English as the fallback (#18). Strings that lichess already has are imported from lila (#19, ADR 0009), and the rest are translated into every language and committed.

Checked against the sources:

* lila keeps one Android-style XML resource file per module and language in `translation/dest/<module>/<tag>.xml` (for example `site/de-DE.xml`), with English in `translation/source/`. Strings use `%s`/`%1$s` placeholders, and some are CLDR `<plurals>`.
* The site offers 94 languages, listed with their native names in lila [`modules/i18n/src/main/LangList.scala`](https://github.com/lichess-org/lila/blob/master/modules/i18n/src/main/LangList.scala). `translation/dest` has more tags (149), some without a site entry. lila maps a bare language code to a default variant (`defaultRegions`: `de` → `de-DE`, `en` → `en-US`, `pt` → `pt-BR`, `zh` → `zh-CN`).
* [`rust-i18n`](https://docs.rs/rust-i18n/4.2.4/rust_i18n/) 4.2.4 compiles YAML/JSON/TOML into the binary and keeps one global locale. Its docs describe no plural rules.
* [Fluent](https://docs.rs/fluent-bundle/0.16.0/fluent_bundle/) (`fluent-bundle` 0.16) handles plurals and grammar. But every lila string would have to be converted to FTL, and `%s` rewritten to Fluent variables.
* [`sys-locale`](https://docs.rs/sys-locale/0.3.2/sys_locale/) 0.3.2 returns the user's preferred locales as BCP 47 tags (it turns POSIX `de_DE.UTF-8` into `de-DE`). On Linux it reads `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` and `LANG`.
* [`intl_pluralrules`](https://docs.rs/intl_pluralrules/7.0.2/intl_pluralrules/) 7.0.2 has the CLDR plural rules compiled in. It is the plural engine Fluent itself uses.

## Considered Options

* A small module of our own: one JSON file per lila language tag, lila's keys and placeholders, typed keys in Rust
* `rust-i18n`
* Fluent (`fluent-bundle` or `fluent-templates`)

## Decision Outcome

Chosen option: "A small module of our own". The strings come from lila, and keeping lila's keys, tags and placeholder syntax makes the import (#19) a copy with no translation step. We need lookup, fallback and later plurals, and none of the grammar that justifies Fluent. `rust-i18n` brings a global locale and no documented plurals.

* **Files.** `src-tauri/i18n/<lila tag>.json` (for example `de-DE.json`), a flat map from key to string. `build.rs` embeds every file in the binary. At runtime only the UI language and the base language are parsed.
* **Base.** `en-GB.json` is lila's source language and has every key. A missing key or language falls back to it.
* **Keys.** `<lila module>.<lila key>` for strings shared with lichess, for example `site.gameOver`. `desktop.<key>` for our own; `desktop` because lila already has an `app` module. In Rust, the `Key` enum is generated together with the key ids, so a typo fails to compile and a key missing from `en-GB.json` fails a test.
* **Placeholders and plurals.** Strings keep lila's `%s`/`%1$s`. A plural string will be a JSON object of CLDR categories (`{"one": "…", "other": "…"}`), selected with `intl_pluralrules`. The crate is added with the first string that needs a plural.
* **Languages.** `i18n::LANGUAGES` is lila's `LangList` (94 tags with native names, in lichess's order). The Language setting offers "System" plus these. A tag that isn't on the list is ignored.
* **Choosing the language.** If the setting names a lichess language, it wins. Otherwise the first system locale from `sys-locale` that matches one wins, else English. A locale matches its exact language and region (`pt-PT`), else lila's default region (`de-AT` → `de-DE`), else the first variant of that language (`fr-CA` → `fr-FR`). Chinese goes by script or region (`zh-Hant`, `zh-HK` → `zh-TW`, otherwise `zh-CN`). Bare `no` means Bokmål (`nb-NO`).
* **Direction.** Arabic, Sorani, Persian, Hebrew, Pashto and Urdu render right-to-left. Pages get `lang` and `dir` from Rust.
* **Where strings are used.** Rust uses them directly: tray menu, and the offline page through placeholders filled and HTML-escaped on each request. Bundled windows will get the resolved strings of their language from Rust through a typed command (ADR 0012), added with #15. A language change will rebuild the tray menu and emit an event to open bundled windows; that is wired up with the settings window (#15), which is the first place the setting can change.
* **Coverage.** `i18n/` may only contain lichess languages and known keys. #19 adds the check that every language has every key.

### Consequences

* Good, because lila strings drop in unchanged and our keys show where each string comes from.
* Good, because the dependency is small (`sys-locale`, later `intl_pluralrules`) and lookups are plain map reads.
* Bad, because grammar beyond plurals (gender, case) is not supported. lila doesn't use it either.
* Bad, because the language list is a copy of lila's. The weekly lila watcher (#22) should compare it.
