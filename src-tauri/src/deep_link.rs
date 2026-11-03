//! `lichess-desktop://<path>` deep links, mapped to `https://lichess.org/<path>`.
//! The mapped URL must still pass the navigation allowlist (ADR 0005).
//!
//! The scheme is declared in `tauri.conf.json`, so the installers register it
//! (desktop file for deb/rpm, Info.plist on macOS, registry for NSIS/MSI). An
//! AppImage is not installed, so it registers itself at runtime instead.

use tauri::Url;

use crate::navigation;

/// Scheme the app handles. Must match `plugins > deep-link > desktop > schemes`
/// in `tauri.conf.json`.
pub const SCHEME: &str = "lichess-desktop";

const TARGET_HOST: &str = "lichess.org";

/// Maps a deep link to the lichess.org URL it stands for, or `None` if it is
/// not a deep link or does not map to a URL the app may show.
///
/// Everything after `lichess-desktop:` and its slashes becomes the path, query
/// and fragment on lichess.org, so `lichess-desktop://tv` opens
/// `https://lichess.org/tv`.
pub fn to_lichess_url(link: &Url) -> Option<Url> {
    if link.scheme() != SCHEME {
        return None;
    }
    // The serialized link keeps the path as typed. `Url`'s host and path
    // accessors don't, since `<path>` is parsed as an authority.
    let rest = link
        .as_str()
        .strip_prefix(SCHEME)?
        .strip_prefix(':')?
        .trim_start_matches('/');
    let url: Url = format!("https://{TARGET_HOST}/{rest}").parse().ok()?;
    let exact_target = url.host_str() == Some(TARGET_HOST)
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none();
    (exact_target && navigation::is_internal(&url)).then_some(url)
}

/// The URL to open for a batch of deep links: the last one that maps. macOS
/// may deliver several at once, other platforms one.
pub fn target(links: &[Url]) -> Option<Url> {
    links.iter().rev().find_map(to_lichess_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(s: &str) -> Option<String> {
        to_lichess_url(&s.parse().unwrap()).map(String::from)
    }

    #[test]
    fn scheme_matches_the_config() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let schemes = &config["plugins"]["deep-link"]["desktop"]["schemes"];
        assert_eq!(schemes, &serde_json::json!([SCHEME]));
    }

    #[test]
    fn maps_paths_to_lichess() {
        assert_eq!(
            map("lichess-desktop://").as_deref(),
            Some("https://lichess.org/")
        );
        assert_eq!(
            map("lichess-desktop://abcdEFGH").as_deref(),
            Some("https://lichess.org/abcdEFGH")
        );
        // `lichess-desktop://@/name` is not a valid URL (empty host), so
        // profile links need the third slash or no slashes.
        assert!("lichess-desktop://@/SomeOne".parse::<Url>().is_err());
        assert_eq!(
            map("lichess-desktop:///@/SomeOne").as_deref(),
            Some("https://lichess.org/@/SomeOne")
        );
        assert_eq!(
            map("lichess-desktop:@/SomeOne").as_deref(),
            Some("https://lichess.org/@/SomeOne")
        );
        assert_eq!(
            map("lichess-desktop://study/AbC/dEf#3").as_deref(),
            Some("https://lichess.org/study/AbC/dEf#3")
        );
        assert_eq!(
            map("lichess-desktop://analysis?fen=x%20y").as_deref(),
            Some("https://lichess.org/analysis?fen=x%20y")
        );
        assert_eq!(
            map("LICHESS-DESKTOP:///training").as_deref(),
            Some("https://lichess.org/training")
        );
        assert_eq!(
            map("lichess-desktop:tv").as_deref(),
            Some("https://lichess.org/tv")
        );
    }

    #[test]
    fn cannot_leave_lichess() {
        for link in [
            "lichess-desktop://evil.com",
            "lichess-desktop://@evil.com/",
            "lichess-desktop://user:pw@evil.com/",
            "lichess-desktop://lichess.org.evil.com/",
            "lichess-desktop:////evil.com/",
            "lichess-desktop://%5C%5Cevil.com/",
            "lichess-desktop://../../evil.com",
        ] {
            let url: Url = map(link).unwrap().parse().unwrap();
            assert_eq!(url.host_str(), Some("lichess.org"), "{link}");
            assert_eq!(url.port(), None, "{link}");
            assert!(url.username().is_empty(), "{link}");
        }
    }

    #[test]
    fn rejects_other_schemes() {
        assert_eq!(map("https://lichess.org/"), None);
        assert_eq!(map("lichess://abc"), None);
        assert_eq!(map("lichess-desktop-evil://abc"), None);
        assert_eq!(map("file:///etc/passwd"), None);
        assert_eq!(map("javascript:alert(1)"), None);
    }

    #[test]
    fn target_takes_the_last_valid_link() {
        let links: Vec<Url> = [
            "lichess-desktop://a",
            "https://evil.com/",
            "lichess-desktop://b",
        ]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
        assert_eq!(
            target(&links).map(String::from).as_deref(),
            Some("https://lichess.org/b")
        );
        assert_eq!(target(&links[1..2]), None);
        assert_eq!(target(&[]), None);
    }
}
