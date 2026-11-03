//! Bundled page shown when lichess.org is unreachable at startup. Tauri can't
//! report main-frame load failures, so we probe the host before the first
//! load instead. See ADR 0011.

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use tauri::Url;
use tauri::http::{Response, header};

/// URI scheme the page is served from. Registered with
/// `Builder::register_uri_scheme_protocol`.
pub const SCHEME: &str = "offline";

const PAGE: &str = include_str!("offline.html");

/// The page has no scripts, so none may run. This also keeps it away from IPC,
/// even though Tauri treats custom schemes as local.
const CSP: &str =
    "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'";

const PROBE_HOST: (&str, u16) = ("lichess.org", 443);

/// Total time the startup probe may take, DNS lookup included.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// URL of the page. Custom schemes are `<scheme>://localhost/` on macOS and
/// Linux, and `http://<scheme>.localhost/` on Windows (see the docs of
/// `Builder::register_uri_scheme_protocol`).
pub fn page_url() -> Url {
    let url = if cfg!(windows) {
        format!("http://{SCHEME}.localhost/")
    } else {
        format!("{SCHEME}://localhost/")
    };
    url.parse().expect("offline page URL is valid")
}

/// Returns `true` if `url` is the offline page (any path on its origin).
pub fn is_page(url: &Url) -> bool {
    let page = page_url();
    url.scheme() == page.scheme() && url.host_str() == page.host_str() && url.port().is_none()
}

/// Response for every request to [`SCHEME`]. The page has no subresources.
pub fn response() -> Response<Vec<u8>> {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CONTENT_SECURITY_POLICY, CSP)
        .header(header::CACHE_CONTROL, "no-store")
        .body(PAGE.as_bytes().to_vec())
        .expect("static offline response is valid")
}

/// Starts probing lichess.org in the background. The receiver yields `true`
/// once a TCP connection to port 443 succeeds, or `false` if it fails. Nothing
/// is sent over the connection.
pub fn spawn_probe() -> Receiver<bool> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(lichess_reachable(PROBE_TIMEOUT));
    });
    rx
}

/// Waits for the probe started by [`spawn_probe`]. A probe that doesn't finish
/// within [`PROBE_TIMEOUT`] counts as reachable, so a slow network still gets
/// the real page instead of the offline page.
pub fn wait_for_probe(rx: &Receiver<bool>) -> bool {
    rx.recv_timeout(PROBE_TIMEOUT).unwrap_or(true)
}

fn lichess_reachable(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    // The system resolver has no timeout; `wait_for_probe` bounds it instead.
    let Ok(addrs) = PROBE_HOST.to_socket_addrs() else {
        return false;
    };
    for addr in addrs {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        if TcpStream::connect_timeout(&addr, left).is_ok() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        s.parse().unwrap()
    }

    #[test]
    fn recognises_only_the_offline_origin() {
        assert!(is_page(&page_url()));
        if cfg!(windows) {
            assert!(is_page(&url("http://offline.localhost/index.html")));
            assert!(!is_page(&url("https://offline.localhost/")));
            assert!(!is_page(&url("http://offline.localhost:8080/")));
            assert!(!is_page(&url("http://localhost/")));
        } else {
            assert!(is_page(&url("offline://localhost/index.html")));
            assert!(!is_page(&url("offline://evil.com/")));
            assert!(!is_page(&url("tauri://localhost/")));
        }
        assert!(!is_page(&url("https://lichess.org/")));
    }

    #[test]
    #[ignore = "needs network access"]
    fn probe_reaches_lichess_when_online() {
        assert!(wait_for_probe(&spawn_probe()));
    }

    #[test]
    fn response_forbids_scripts() {
        let response = response();
        let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap();
        assert!(csp.starts_with("default-src 'none'"));
        assert!(!csp.contains("script-src"));
        assert!(!PAGE.contains("<script"));
    }

    #[test]
    fn page_links_back_to_lichess_only() {
        assert!(PAGE.contains(r#"href="https://lichess.org/""#));
        assert_eq!(PAGE.matches("href=").count(), 1);
    }
}
