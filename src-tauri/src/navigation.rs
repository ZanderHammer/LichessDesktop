//! Navigation policy: lichess.org (and the few frames it embeds) stays in the
//! app, everything else opens in the system browser. The bundled offline page
//! is the only local page allowed. See ADR 0005, 0007 and 0011.

use tauri::Url;

use crate::offline;

/// Hosts whose pages may be shown inside the app window.
const ALLOWED_HOST: &str = "lichess.org";

/// Third-party hosts that lichess embeds in iframes and that must load in the
/// app. On Linux and macOS the navigation handler also sees iframe loads, so
/// without this they would be blocked and opened in the system browser.
const FRAME_HOSTS: &[&str] = &[
    // Cloudflare Turnstile, the bot check on the login and signup forms.
    "challenges.cloudflare.com",
];

/// Returns `true` if `url` may be displayed inside the app window.
pub fn is_internal(url: &Url) -> bool {
    if offline::is_page(url) {
        return true;
    }
    match url.scheme() {
        "https" => url
            .host_str()
            .is_some_and(|host| is_allowed_host(host) || FRAME_HOSTS.contains(&host)),
        // In-page helpers (e.g. blob downloads, blank frames) never leave the app.
        "about" | "blob" => true,
        _ => false,
    }
}

/// Returns `true` if `url` is on the origin of the bundled app pages
/// (`frontendDist`): `tauri://localhost` on macOS and Linux,
/// `http://tauri.localhost` on Windows. Bundled windows may show only these.
pub fn is_bundled(url: &Url) -> bool {
    let (scheme, host) = if cfg!(windows) {
        ("http", "tauri.localhost")
    } else {
        ("tauri", "localhost")
    };
    url.scheme() == scheme && url.host_str() == Some(host) && url.port().is_none()
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
    fn allows_the_offline_page_and_no_other_local_page() {
        assert!(is_internal(&offline::page_url()));
        assert!(!is_internal(&url("tauri://localhost/")));
        assert!(!is_internal(&url("http://tauri.localhost/")));
        assert!(!is_internal(&url("http://localhost/")));
        assert!(!is_internal(&url("file:///etc/passwd")));
    }

    #[test]
    fn allows_cloudflare_turnstile_frame() {
        assert!(is_internal(&url(
            "https://challenges.cloudflare.com/cdn-cgi/challenge-platform/h/g/turnstile/f/av0/rch/1i5ib/0x4AAAAAAC_aOMpwtdCnD5y5/auto/fbE/crashed_retry/normal?lang=de-de"
        )));
        assert!(!is_internal(&url("https://cloudflare.com/")));
        assert!(!is_internal(&url(
            "https://evil.challenges.cloudflare.com/"
        )));
    }

    #[test]
    fn rejects_other_hosts_and_schemes() {
        assert!(!is_internal(&url("https://github.com/lichess-org")));
        assert!(!is_internal(&url("https://evillichess.org/")));
        assert!(!is_internal(&url("https://lichess.org.evil.com/")));
        assert!(!is_internal(&url("http://lichess.org/")));
        assert!(!is_internal(&url("mailto:contact@lichess.org")));
    }

    #[test]
    fn recognises_only_the_bundled_origin() {
        if cfg!(windows) {
            assert!(is_bundled(&url("http://tauri.localhost/settings.html")));
            assert!(!is_bundled(&url("https://tauri.localhost/settings.html")));
            assert!(!is_bundled(&url("http://tauri.localhost:8080/")));
        } else {
            assert!(is_bundled(&url("tauri://localhost/settings.html")));
            assert!(!is_bundled(&url("tauri://evil.com/")));
        }
        assert!(!is_bundled(&url("https://lichess.org/")));
        assert!(!is_bundled(&offline::page_url()));
        assert!(!is_bundled(&url("http://localhost/")));
    }
}
