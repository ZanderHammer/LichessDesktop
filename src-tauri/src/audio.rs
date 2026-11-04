//! Keeps page audio audible on Bluetooth outputs on Linux. On PipeWire,
//! GStreamer's `pulsesink` goes silent about a second into a stream with the
//! small buffer it asks for by default, while the page still sees a running
//! `AudioContext`. WebKitGTK creates that sink itself, so the only way in is the
//! environment its web process inherits. See ADR 0026.

use std::env;

const VARIABLE: &str = "PULSE_PROP";
const KEY: &str = "pulse.min.req";
/// 150 ms. Smaller values fail on Bluetooth: up to 3600 frames still cut out,
/// and 4800 leaves gaps in the sound.
const VALUE: &str = "7200/48000";

/// Sets the minimum PulseAudio request size for this process and its children.
/// Must run before any other thread starts, as it changes the environment.
pub fn configure() {
    // A value that isn't Unicode isn't ours to rewrite.
    let current = match env::var(VARIABLE) {
        Ok(current) => Some(current),
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => return,
    };
    if let Some(value) = with_min_request(current.as_deref()) {
        // SAFETY: `main` calls this first, while no other thread exists that
        // could read the environment at the same time.
        unsafe { env::set_var(VARIABLE, value) };
    }
}

/// The new value of the variable, or `None` if the user already chose a request
/// size. Properties in the variable are separated by whitespace.
fn with_min_request(current: Option<&str>) -> Option<String> {
    let setting = format!("{KEY}={VALUE}");
    match current.map(str::trim) {
        None | Some("") => Some(setting),
        Some(current) if current.contains(KEY) => None,
        Some(current) => Some(format!("{current} {setting}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn leaves_a_chosen_request_size_alone() {
        assert_eq!(
            with_min_request(Some("pulse.min.req=1024/48000 media.role=game")),
            None
        );
    }
}
