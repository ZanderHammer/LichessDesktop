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

/// Returns `true` if `url` may be handed to the system's default handler. The
/// page chooses what it navigates to, and the system would run the handler of
/// any other scheme or file without asking (ADR 0027).
pub fn may_open_externally(url: &Url) -> bool {
    matches!(url.scheme(), "https" | "http" | "mailto")
}

/// Returns `true` if `url` is a lichess.org page (not the offline page or a
/// third-party frame).
pub fn is_lichess(url: &Url) -> bool {
    url.scheme() == "https" && url.host_str().is_some_and(is_allowed_host)
}

/// Returns `true` if `url` is on the origin of the bundled app pages
/// (`frontendDist`): `tauri://localhost` on macOS and Linux,
/// `http://tauri.localhost` on Windows. Bundled windows may show only these.
/// `dev_url` is the dev server origin under `cargo tauri dev`; pass `None` in
/// release builds.
pub fn is_bundled(url: &Url, dev_url: Option<&Url>) -> bool {
    if dev_url.is_some_and(|dev| url.origin() == dev.origin()) {
        return true;
    }
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
    fn only_web_and_mail_links_leave_the_app() {
        for allowed in [
            "https://github.com/lichess-org",
            "http://example.com/",
            "mailto:contact@lichess.org",
        ] {
            assert!(may_open_externally(&url(allowed)), "{allowed}");
        }
        for refused in [
            "file:///home/someone/Downloads/x.desktop",
            "smb://example.com/share",
            "ms-msdt:/id",
            "search-ms:query=x",
            "javascript:alert(1)",
            "data:text/html,x",
            "de.itshammer.lichessdesktop://oauth/callback?code=c",
        ] {
            assert!(!may_open_externally(&url(refused)), "{refused}");
        }
    }

    #[test]
    fn recognises_lichess_pages_only() {
        assert!(is_lichess(&url("https://lichess.org/")));
        assert!(is_lichess(&url("https://lichess.org/abcd1234")));
        assert!(is_lichess(&url("https://en.lichess.org/")));
        assert!(!is_lichess(&url("http://lichess.org/")));
        assert!(!is_lichess(&url("https://evillichess.org/")));
        assert!(!is_lichess(&url("https://challenges.cloudflare.com/")));
        assert!(!is_lichess(&url("about:blank")));
        assert!(!is_lichess(&offline::page_url()));
    }

    fn is_bundled_url(url: &Url) -> bool {
        is_bundled(url, None)
    }

    #[test]
    fn allows_the_dev_server_origin_only_when_given() {
        let dev = url("http://127.0.0.1:1430/");
        assert!(is_bundled(
            &url("http://127.0.0.1:1430/settings.html"),
            Some(&dev)
        ));
        assert!(!is_bundled(
            &url("http://127.0.0.1:1431/settings.html"),
            Some(&dev)
        ));
        assert!(!is_bundled(&url("https://lichess.org/"), Some(&dev)));
        assert!(!is_bundled(
            &url("http://127.0.0.1:1430/settings.html"),
            None
        ));
    }

    #[test]
    fn recognises_only_the_bundled_origin() {
        if cfg!(windows) {
            assert!(is_bundled_url(&url("http://tauri.localhost/settings.html")));
            assert!(!is_bundled_url(&url(
                "https://tauri.localhost/settings.html"
            )));
            assert!(!is_bundled_url(&url("http://tauri.localhost:8080/")));
        } else {
            assert!(is_bundled_url(&url("tauri://localhost/settings.html")));
            assert!(!is_bundled_url(&url("tauri://evil.com/")));
        }
        assert!(!is_bundled_url(&url("https://lichess.org/")));
        assert!(!is_bundled_url(&offline::page_url()));
        assert!(!is_bundled_url(&url("http://localhost/")));
    }
}
