//! Keeps page audio audible on Bluetooth outputs on Linux. On PipeWire,
//! GStreamer's `pulsesink` goes silent about a second into a stream with the
//! small buffer it asks for by default, while the page still sees a running
//! `AudioContext`. WebKitGTK creates that sink itself, so the only way in is the
//! environment its web process inherits. It delays all sound on every output,
//! so it is a switch in Settings, off by default. See ADR 0026 and ADR 0029.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::thread;

use crate::settings::{STORE_FILE, Settings};

/// The app identifier from `tauri.conf.json`, which names its data directory.
const IDENTIFIER: &str = "de.itshammer.lichessdesktop";

const VARIABLE: &str = "PULSE_PROP";
const KEY: &str = "pulse.min.req";
/// 150 ms. Smaller values fail on Bluetooth: up to 3600 frames still cut out,
/// and 4800 leaves gaps in the sound.
const VALUE: &str = "7200/48000";

/// The variable before [`configure`] changed it. Not set if it didn't.
static ORIGINAL: OnceLock<Option<String>> = OnceLock::new();

/// Sets the minimum PulseAudio request size for this process and its children,
/// if the setting is on. Must run before any other thread starts, as it changes
/// the environment, so the settings store isn't open yet.
pub fn configure() {
    if !is_enabled() {
        return;
    }
    // A value that isn't Unicode isn't ours to rewrite.
    let current = match env::var(VARIABLE) {
        Ok(current) => Some(current),
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => return,
    };
    if let Some(value) = with_min_request(current.as_deref()) {
        let _ = ORIGINAL.set(current);
        // SAFETY: `main` calls this first, while no other thread exists that
        // could read the environment at the same time.
        unsafe { env::set_var(VARIABLE, value) };
    }
}

/// Whether the setting is on, from the file the store keeps it in.
fn is_enabled() -> bool {
    settings_file(env::var_os("XDG_DATA_HOME"), env::home_dir())
        .and_then(|path| fs::read(path).ok())
        .is_some_and(|bytes| Settings::from_file(&bytes).bluetooth_audio)
}

/// Where the store plugin keeps the settings: its app data directory, which on
/// Linux is the XDG data directory (a relative `XDG_DATA_HOME` is ignored).
fn settings_file(xdg_data_home: Option<OsString>, home: Option<PathBuf>) -> Option<PathBuf> {
    let data = xdg_data_home
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| Some(home?.join(".local/share")))?;
    Some(Path::new(&data).join(IDENTIFIER).join(STORE_FILE))
}

/// Opens `url` with the variable as the user had it, so a browser this starts
/// doesn't keep the setting for as long as it runs (ADR 0028).
///
/// shortcut: only `xdg-open`, and only a failure to start it reports an error,
/// add the opener's other launchers if a system without a working one turns up.
pub fn open_url(url: &str) -> io::Result<()> {
    let mut command = Command::new("xdg-open");
    command
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    restore(&mut command, ORIGINAL.get());
    let mut child = command.spawn()?;
    // It may wait for the browser, and the app must collect it when it ends.
    thread::spawn(move || child.wait());
    Ok(())
}

fn restore(command: &mut Command, original: Option<&Option<String>>) {
    match original {
        Some(Some(value)) => {
            command.env(VARIABLE, value);
        }
        Some(None) => {
            command.env_remove(VARIABLE);
        }
        None => {}
    }
}

/// The new value of the variable, or `None` if the user already chose a request
/// size. Properties in the variable are separated by whitespace.
fn with_min_request(current: Option<&str>) -> Option<String> {
    let setting = format!("{KEY}={VALUE}");
    match current.map(str::trim) {
        None | Some("") => Some(setting),
        Some(current) if current.split_whitespace().any(is_min_request) => None,
        Some(current) => Some(format!("{current} {setting}")),
    }
}

fn is_min_request(property: &str) -> bool {
    property
        .strip_prefix(KEY)
        .is_some_and(|value| value.starts_with('='))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_settings_in_the_xdg_data_directory() {
        let home = || Some(PathBuf::from("/home/someone"));
        let in_home = "/home/someone/.local/share/de.itshammer.lichessdesktop/settings.json";
        assert_eq!(settings_file(None, home()), Some(in_home.into()));
        assert_eq!(
            settings_file(Some("relative".into()), home()),
            Some(in_home.into())
        );
        assert_eq!(
            settings_file(Some("/data".into()), home()),
            Some("/data/de.itshammer.lichessdesktop/settings.json".into())
        );
        assert_eq!(settings_file(None, None), None);
    }

    #[test]
    fn the_identifier_is_the_one_in_the_config() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["identifier"], IDENTIFIER);
    }

    fn env_after_restore(original: Option<&Option<String>>) -> Vec<(String, Option<String>)> {
        let mut command = Command::new("xdg-open");
        restore(&mut command, original);
        command
            .get_envs()
            .map(|(key, value)| {
                let text = |text: &std::ffi::OsStr| text.to_str().unwrap().to_owned();
                (text(key), value.map(text))
            })
            .collect()
    }

    #[test]
    fn programs_the_app_starts_get_the_variable_as_it_was() {
        assert_eq!(
            env_after_restore(Some(&None)),
            [(VARIABLE.to_owned(), None)]
        );
        let theirs = "media.role=game".to_owned();
        assert_eq!(
            env_after_restore(Some(&Some(theirs.clone()))),
            [(VARIABLE.to_owned(), Some(theirs))]
        );
        // The variable wasn't changed, so it is inherited as it is.
        assert!(env_after_restore(None).is_empty());
    }

    #[test]
    fn sets_the_request_size_when_the_variable_is_unset_or_empty() {
        let expected = Some("pulse.min.req=7200/48000".to_string());
        assert_eq!(with_min_request(None), expected);
        assert_eq!(with_min_request(Some("  ")), expected);
    }

    #[test]
    fn keeps_other_properties() {
        assert_eq!(
            with_min_request(Some("media.role=game")).as_deref(),
            Some("media.role=game pulse.min.req=7200/48000")
        );
    }

    #[test]
    fn a_property_that_only_mentions_the_key_is_not_a_chosen_size() {
        assert_eq!(
            with_min_request(Some("x.pulse.min.req.note=1")).as_deref(),
            Some("x.pulse.min.req.note=1 pulse.min.req=7200/48000")
        );
    }

    #[test]
    fn leaves_a_chosen_request_size_alone() {
        assert_eq!(
            with_min_request(Some("pulse.min.req=1024/48000 media.role=game")),
            None
        );
    }
}
