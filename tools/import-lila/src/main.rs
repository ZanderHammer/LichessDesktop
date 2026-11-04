//! Imports the lichess translations from a lila checkout into `src-tauri/i18n/`
//! and lists the strings lila doesn't have. See ADR 0020.
//!
//! `cargo run -p import-lila -- <path to lila>`
//!
//! `en-GB.json` decides the keys. Every key outside `desktop.` is a lila string
//! (`<module>.<key>`). English comes from `translation/source/<module>.xml`,
//! the other languages from `translation/dest/<module>/<tag>.xml`. A string
//! lila doesn't have for a language keeps the value already in its file.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;
use std::{env, fs};

const I18N_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../src-tauri/i18n");
const BASE: &str = "en-GB";
const OUR_MODULE: &str = "desktop";

type Strings = BTreeMap<String, String>;

fn main() -> Result<(), Box<dyn Error>> {
    let lila = env::args()
        .nth(1)
        .ok_or("usage: import-lila <path to lila>")?;
    let translation = Path::new(&lila).join("translation");
    let i18n = Path::new(I18N_DIR);

    let mut base = read_json(&i18n.join(format!("{BASE}.json")))?;
    for (id, text) in &mut base {
        if let Some((module, key)) = lila_key(id) {
            let path = translation.join("source").join(format!("{module}.xml"));
            *text = read_module(&path)?
                .remove(key)
                .ok_or_else(|| format!("{id} is not a plain string in lila's source"))?;
        }
    }
    write_json(&i18n.join(format!("{BASE}.json")), &base)?;

    let mut missing_total = 0;
    for entry in fs::read_dir(i18n)? {
        let path = entry?.path();
        let tag = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        if tag == BASE || path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let mut lila_strings = Strings::new();
        for id in base.keys() {
            if let Some((module, key)) = lila_key(id) {
                let file = translation
                    .join("dest")
                    .join(module)
                    .join(format!("{tag}.xml"));
                if let Some(text) = read_module(&file)?.remove(key) {
                    lila_strings.insert(id.clone(), text);
                }
            }
        }
        let (merged, missing) = merge(&base, &lila_strings, &read_json(&path)?);
        write_json(&path, &merged)?;
        if !missing.is_empty() {
            println!("{tag}: {}", missing.join(" "));
            missing_total += missing.len();
        }
    }
    println!("{missing_total} strings still need a translation");
    Ok(())
}

/// Splits a key id into lila's module and key, or `None` for our own strings.
fn lila_key(id: &str) -> Option<(&str, &str)> {
    id.split_once('.')
        .filter(|(module, _)| *module != OUR_MODULE)
}

/// The strings of one language: for each base key, lila's text, else the text
/// already in the file. Also returns the keys that have neither.
fn merge(base: &Strings, lila: &Strings, existing: &Strings) -> (Strings, Vec<String>) {
    let mut merged = Strings::new();
    let mut missing = Vec::new();
    for id in base.keys() {
        match lila.get(id).or_else(|| existing.get(id)) {
            Some(text) => {
                merged.insert(id.clone(), text.clone());
            }
            None => missing.push(id.clone()),
        }
    }
    (merged, missing)
}

/// The plain strings of a lila XML file. lila has no file for a language
/// nobody translated that module into, which counts as no strings.
fn read_module(path: &Path) -> Result<Strings, Box<dyn Error>> {
    if !path.exists() {
        return Ok(Strings::new());
    }
    parse_module(&fs::read_to_string(path)?).map_err(|err| format!("{path:?}: {err}").into())
}

/// Reads the `<string>` elements. `<plurals>` are skipped: none of our strings
/// use one yet (ADR 0015).
fn parse_module(xml: &str) -> Result<Strings, roxmltree::Error> {
    let doc = roxmltree::Document::parse(xml)?;
    Ok(doc
        .root_element()
        .children()
        .filter(|node| node.has_tag_name("string"))
        .filter_map(|node| {
            let text = unescape(node.text()?);
            let name = node.attribute("name")?;
            (!text.is_empty()).then(|| (name.to_owned(), text))
        })
        .collect())
}

/// lila's files are Android string resources, which escape quotes with a
/// backslash. Some translations also have stray spaces around them.
fn unescape(text: &str) -> String {
    text.trim().replace("\\\"", "\"").replace("\\'", "'")
}

fn read_json(path: &Path) -> Result<Strings, Box<dyn Error>> {
    serde_json::from_str(&fs::read_to_string(path)?)
        .map_err(|err| format!("{path:?}: {err}").into())
}

/// Writes pretty JSON with sorted keys, so a new import has a minimal diff.
fn write_json(path: &Path, strings: &Strings) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(strings)?;
    fs::write(path, json + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(pairs: &[(&str, &str)]) -> Strings {
        pairs
            .iter()
            .map(|(id, text)| ((*id).to_owned(), (*text).to_owned()))
            .collect()
    }

    #[test]
    fn reads_plain_strings_and_undoes_the_escapes() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
            <resources>
              <string name="quote" comment="a hint">Keine für \"%s\"</string>
              <string name="apostrophe">Juga cuentra l\'ordinador</string>
              <string name="spaces"> पजल </string>
              <string name="entity">Lichess &amp; DGT</string>
              <string name="empty"></string>
              <plurals name="many"><item quantity="one">one</item></plurals>
            </resources>"#;
        assert_eq!(
            parse_module(xml).unwrap(),
            strings(&[
                ("apostrophe", "Juga cuentra l'ordinador"),
                ("entity", "Lichess & DGT"),
                ("quote", "Keine für \"%s\""),
                ("spaces", "पजल"),
            ])
        );
    }

    #[test]
    fn only_our_own_module_is_not_a_lila_key() {
        assert_eq!(lila_key("site.search"), Some(("site", "search")));
        assert_eq!(lila_key("desktop.quit"), None);
    }

    #[test]
    fn lila_wins_then_the_existing_text_and_the_rest_is_missing() {
        let base = strings(&[
            ("desktop.quit", "Quit"),
            ("site.gap", "Gap"),
            ("site.none", "None"),
            ("site.search", "Search"),
        ]);
        let lila = strings(&[("site.search", "Suchen")]);
        let existing = strings(&[
            ("desktop.quit", "Beenden"),
            ("site.gap", "Lücke"),
            ("site.removed", "Weg"),
            ("site.search", "Alt"),
        ]);
        let (merged, missing) = merge(&base, &lila, &existing);
        assert_eq!(
            merged,
            strings(&[
                ("desktop.quit", "Beenden"),
                ("site.gap", "Lücke"),
                ("site.search", "Suchen"),
            ])
        );
        assert_eq!(missing, ["site.none"]);
    }
}
