use std::fmt::Write as _;
use std::path::Path;
use std::{env, fs};

/// Every app command. Listing them creates the app's ACL manifest, so each
/// command needs an `allow-<command>` permission in a capability (ADR 0012).
const COMMANDS: &[&str] = &[
    "get_strings",
    "get_settings",
    "set_setting",
    "get_app_info",
    "get_shortcuts",
    "settings_ready",
    "close_settings",
    "list_actions",
    "run_action",
    "palette_ready",
    "close_palette",
];

fn main() {
    embed_translations();
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}

/// Writes `$OUT_DIR/translations.rs`: one `(tag, json)` entry per file in
/// `i18n/`, so every language is in the binary and none is read from disk.
fn embed_translations() {
    let dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("i18n");
    println!("cargo:rerun-if-changed={}", dir.display());

    let mut files: Vec<_> = fs::read_dir(&dir)
        .expect("src-tauri/i18n exists")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();

    let mut out = String::from("&[\n");
    for path in files {
        let tag = path.file_stem().unwrap().to_str().unwrap();
        let path = path.to_str().expect("UTF-8 path");
        writeln!(out, "    ({tag:?}, include_str!({path:?})),").unwrap();
    }
    out.push(']');
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("translations.rs"),
        out,
    )
    .unwrap();
}
