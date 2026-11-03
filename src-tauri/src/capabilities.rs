//! Checks the capability files against ADR 0005 and ADR 0012: the main window
//! (lichess.org and the offline page) gets no capability, and no capability
//! reaches remote content.

use std::fs;

use serde_json::Value;

use crate::MAIN_WINDOW;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/capabilities");

/// Returns the reasons why `capability` breaks the rules, if any.
fn violations(capability: &Value) -> Vec<String> {
    let mut found = Vec::new();
    if capability.get("remote").is_some() {
        found.push("declares `remote`".to_owned());
    }
    if capability.get("webviews").is_some() {
        found.push("targets `webviews` instead of one window label".to_owned());
    }
    match capability.get("windows").and_then(Value::as_array) {
        Some(windows) if windows.len() == 1 => {
            let label = windows[0].as_str().unwrap_or_default();
            if label.is_empty() || label.contains(['*', '?', '[', ']', '{', '}', '!']) {
                found.push(format!("window label {label:?} is empty or a glob"));
            } else if label == MAIN_WINDOW {
                found.push(format!("grants the {MAIN_WINDOW:?} window"));
            }
        }
        _ => found.push("must name exactly one window label".to_owned()),
    }
    found
}

#[test]
fn no_capability_reaches_the_main_window_or_remote_content() {
    let entries = match fs::read_dir(DIR) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
        Err(err) => panic!("cannot read {DIR}: {err}"),
    };
    for entry in entries {
        let path = entry.unwrap().path();
        assert_eq!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("json"),
            "{}: only JSON capability files are checked, so use JSON",
            path.display()
        );
        let capability: Value = serde_json::from_slice(&fs::read(&path).unwrap())
            .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        let found = violations(&capability);
        assert!(found.is_empty(), "{}: {}", path.display(), found.join("; "));
    }
}

#[test]
fn flags_capabilities_that_break_the_rules() {
    use serde_json::json;

    assert!(violations(&json!({ "windows": ["settings"] })).is_empty());
    for bad in [
        json!({ "windows": ["main"] }),
        json!({ "windows": ["*"] }),
        json!({ "windows": ["ma?n"] }),
        json!({ "windows": ["settings", "palette"] }),
        json!({ "windows": [] }),
        json!({}),
        json!({ "windows": ["settings"], "webviews": ["main"] }),
        json!({ "windows": ["settings"], "remote": { "urls": ["https://lichess.org"] } }),
    ] {
        assert!(!violations(&bad).is_empty(), "accepted {bad}");
    }
}
