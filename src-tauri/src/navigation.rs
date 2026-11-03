//! Navigation policy: lichess.org stays in the app, everything else opens in
//! the system browser. See docs/adr/0005-remote-content-security.md.

use tauri::Url;

/// Hosts whose pages may be shown inside the app window.
const ALLOWED_HOST: &str = "lichess.org";

/// Returns `true` if `url` may be displayed inside the app window.
pub fn is_internal(url: &Url) -> bool {
    match url.scheme() {
        "https" => url.host_str().is_some_and(is_allowed_host),
        // In-page helpers (e.g. blob downloads, blank frames) never leave the app.
        "about" | "blob" => true,
        _ => false,
    }
}

fn is_allowed_host(host: &str) -> bool {
    host == ALLOWED_HOST
        || host
            .strip_suffix(ALLOWED_HOST)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        s.parse().unwrap()
    }

    #[test]
    fn allows_lichess_and_subdomains() {
        assert!(is_internal(&url("https://lichess.org/")));
        assert!(is_internal(&url("https://lichess.org/@/someone")));
        assert!(is_internal(&url("https://en.lichess.org/")));
        assert!(is_internal(&url("about:blank")));
    }

    #[test]
    fn rejects_other_hosts_and_schemes() {
        assert!(!is_internal(&url("https://github.com/lichess-org")));
        assert!(!is_internal(&url("https://evillichess.org/")));
        assert!(!is_internal(&url("https://lichess.org.evil.com/")));
        assert!(!is_internal(&url("http://lichess.org/")));
        assert!(!is_internal(&url("mailto:contact@lichess.org")));
    }
}
