//! Checks the bundled frontend in `src-tauri/ui/` (ADR 0012): its assets are
//! embedded, and the CSP and config keep it local and away from lichess.org.

use serde_json::Value;

const CONFIG: &str = include_str!("../tauri.conf.json");

fn config() -> Value {
    serde_json::from_str(CONFIG).expect("tauri.conf.json is valid JSON")
}

#[test]
fn shared_assets_are_embedded() {
    let context = crate::context::<tauri::Wry>();
    for asset in ["app.css", "ipc.js", "strings.js"] {
        assert!(
            context.assets().get(&asset.into()).is_some(),
            "{asset} is not bundled"
        );
    }
}

#[test]
fn palette_assets_are_embedded() {
    let context = crate::context::<tauri::Wry>();
    for asset in ["palette.html", "palette.js", "fuzzy.js"] {
        assert!(
            context.assets().get(&asset.into()).is_some(),
            "{asset} is not bundled"
        );
    }
}

#[test]
fn csp_allows_only_bundled_content_and_ipc() {
    let config = config();
    let csp = config["app"]["security"]["csp"]
        .as_str()
        .expect("app.security.csp is set");
    assert!(csp.starts_with("default-src 'self'"), "{csp}");
    for forbidden in ["'unsafe-inline'", "'unsafe-eval'", "https:", "*"] {
        assert!(!csp.contains(forbidden), "CSP must not contain {forbidden}");
    }
    let http_sources: Vec<_> = csp
        .split([' ', ';'])
        .filter(|source| source.starts_with("http"))
        .collect();
    assert_eq!(http_sources, ["http://ipc.localhost"], "{csp}");
}

#[test]
fn no_script_is_added_to_every_webview() {
    // Both would run in the lichess.org window too (ADR 0008).
    let app = &config()["app"];
    assert_ne!(app["withGlobalTauri"], Value::Bool(true));
    assert_ne!(app["security"]["freezePrototype"], Value::Bool(true));
}
