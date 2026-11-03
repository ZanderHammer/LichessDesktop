//! Checks that the crates we pin to tauri's and wry's versions stay on them.
//!
//! We depend on these crates directly only to reach the same copy tauri uses:
//! to pick tray-icon's ksni backend through feature unification (ADR 0014),
//! or to call the platform webview (ADR 0010). If our version and theirs
//! drift apart, Cargo builds two copies and ours silently does nothing (#50).
//! Two versions of one of them in `Cargo.lock` means exactly that.

const LOCKFILE: &str = include_str!("../../Cargo.lock");

/// Our direct dependencies that must be on tauri's or wry's version.
const PINNED: &[&str] = &[
    "tray-icon",
    "gtk",
    "webkit2gtk",
    "webview2-com",
    "windows",
    "objc2-app-kit",
    "objc2-foundation",
    "objc2-web-kit",
];

/// The locked versions of the package `name`.
fn locked_versions<'a>(lockfile: &'a str, name: &str) -> Vec<&'a str> {
    let wanted = format!("name = \"{name}\"");
    let mut lines = lockfile.lines();
    let mut versions = Vec::new();
    while let Some(line) = lines.next() {
        if line == wanted
            && let Some(version) = lines
                .next()
                .and_then(|line| line.strip_prefix("version = \""))
                .and_then(|rest| rest.strip_suffix('"'))
        {
            versions.push(version);
        }
    }
    versions
}

#[test]
fn pinned_crates_have_one_locked_version() {
    for name in PINNED {
        let versions = locked_versions(LOCKFILE, name);
        assert_eq!(
            versions.len(),
            1,
            "{name} is locked as {versions:?}: pin it in src-tauri/Cargo.toml \
             to the version tauri/wry use"
        );
    }
}

#[test]
fn reads_every_locked_version() {
    let lockfile = "[[package]]\nname = \"tray-icon\"\nversion = \"0.25.1\"\n\n\
                    [[package]]\nname = \"tray-icon\"\nversion = \"0.26.1\"\n\n\
                    [[package]]\nname = \"tray-icon-extra\"\nversion = \"1.0.0\"\n";
    assert_eq!(locked_versions(lockfile, "tray-icon"), ["0.25.1", "0.26.1"]);
    assert!(locked_versions(lockfile, "gtk").is_empty());
}
