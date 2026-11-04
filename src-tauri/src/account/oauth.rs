//! The OAuth 2.0 authorization code flow with PKCE that lichess.org offers
//! unregistered public clients. Pure: no network and no app state.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use tauri::Url;

const AUTHORIZE_URL: &str = "https://lichess.org/oauth";

/// Lichess takes any client id from a public client.
pub const CLIENT_ID: &str = "lichess-desktop";

/// The only scope the app asks for (ADR 0024).
const SCOPE: &str = "challenge:read";

/// Lila only accepts a custom scheme with a dot in it, so this is not the
/// `lichess-desktop` deep link scheme. It is the app identifier from
/// `tauri.conf.json`. The OS doesn't know it: only the main window's
/// navigation handlers do (ADR 0024).
const CALLBACK_SCHEME: &str = "de.itshammer.lichessdesktop";
const CALLBACK_HOST: &str = "oauth";
const CALLBACK_PATH: &str = "/callback";
pub const REDIRECT_URI: &str = "de.itshammer.lichessdesktop://oauth/callback";

/// The secrets of one authorization attempt.
pub struct Request {
    pub verifier: String,
    state: String,
}

impl Request {
    pub fn new() -> Result<Self, getrandom::Error> {
        Ok(Self {
            verifier: random_string()?,
            state: random_string()?,
        })
    }

    /// The page where the user allows access.
    pub fn authorize_url(&self) -> Url {
        let mut url: Url = AUTHORIZE_URL.parse().expect("AUTHORIZE_URL is a valid URL");
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", CLIENT_ID)
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("scope", SCOPE)
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", &challenge(&self.verifier))
            .append_pair("state", &self.state);
        url
    }

    /// Whether a callback answers this attempt. Anything in the main window
    /// can navigate to the callback URL, but only lichess.org knows the state.
    pub fn answered_by(&self, callback: &Callback) -> bool {
        callback.state == self.state
    }
}

/// 32 random bytes as 43 URL-safe characters, the shortest verifier RFC 7636
/// and lila allow.
fn random_string() -> Result<String, getrandom::Error> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier))
}

/// What lichess.org sent back. Without a `code`, the attempt failed.
#[derive(Debug, PartialEq, Eq)]
pub struct Callback {
    state: String,
    pub code: Option<String>,
}

pub fn is_callback(url: &Url) -> bool {
    url.scheme() == CALLBACK_SCHEME
        && url.host_str() == Some(CALLBACK_HOST)
        && url.path() == CALLBACK_PATH
}

/// Reads a callback URL, which [`is_callback`] has accepted.
pub fn parse_callback(url: &Url) -> Callback {
    let param = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    Callback {
        state: param("state").unwrap_or_default(),
        code: param("code").filter(|code| !code.is_empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn callback(s: &str) -> Callback {
        parse_callback(&s.parse().unwrap())
    }

    #[test]
    fn challenge_matches_the_rfc_7636_example() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn verifier_and_state_are_long_enough_and_fresh() {
        let (a, b) = (Request::new().unwrap(), Request::new().unwrap());
        assert_eq!(a.verifier.len(), 43);
        assert_eq!(a.state.len(), 43);
        assert_ne!(a.verifier, b.verifier);
        assert_ne!(a.verifier, a.state);
    }

    #[test]
    fn authorize_url_asks_for_one_scope_and_hides_the_verifier() {
        let request = Request::new().unwrap();
        let url = request.authorize_url();
        assert_eq!(url.host_str(), Some("lichess.org"));
        assert_eq!(url.path(), "/oauth");
        let param = |name: &str| {
            url.query_pairs()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.into_owned())
        };
        assert_eq!(param("scope").as_deref(), Some("challenge:read"));
        assert_eq!(param("response_type").as_deref(), Some("code"));
        assert_eq!(param("code_challenge_method").as_deref(), Some("S256"));
        assert_eq!(param("redirect_uri").as_deref(), Some(REDIRECT_URI));
        assert_eq!(param("code_challenge"), Some(challenge(&request.verifier)));
        assert_eq!(param("state").as_deref(), Some(request.state.as_str()));
        assert!(!url.as_str().contains(&request.verifier));
    }

    #[test]
    fn the_redirect_uri_is_our_callback() {
        assert!(is_callback(&REDIRECT_URI.parse().unwrap()));
    }

    #[test]
    fn the_callback_scheme_is_the_app_identifier_and_has_a_dot() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        assert_eq!(config["identifier"], CALLBACK_SCHEME);
        assert!(CALLBACK_SCHEME.contains('.'), "lila requires a dot");
    }

    #[test]
    fn recognises_only_the_callback() {
        for other in [
            "https://lichess.org/oauth",
            "lichess-desktop://oauth/callback",
            "de.itshammer.lichessdesktop://oauth/other",
            "de.itshammer.lichessdesktop://evil/callback",
            "de.itshammer.lichessdesktop-evil://oauth/callback",
        ] {
            assert!(!is_callback(&other.parse().unwrap()), "{other}");
        }
    }

    #[test]
    fn reads_a_code_or_a_failure() {
        assert_eq!(
            callback("de.itshammer.lichessdesktop://oauth/callback?code=liu_a%2Bb&state=s"),
            Callback {
                state: "s".into(),
                code: Some("liu_a+b".into()),
            }
        );
        assert_eq!(
            callback(
                "de.itshammer.lichessdesktop://oauth/callback?error=access_denied&error_description=x&state=s"
            ),
            Callback {
                state: "s".into(),
                code: None,
            }
        );
        assert_eq!(
            callback("de.itshammer.lichessdesktop://oauth/callback?code=&state=s").code,
            None
        );
    }

    #[test]
    fn only_the_right_state_answers_a_request() {
        let request = Request::new().unwrap();
        let answer = |state: &str| Callback {
            state: state.into(),
            code: Some("code".into()),
        };
        assert!(request.answered_by(&answer(&request.state)));
        assert!(!request.answered_by(&answer("forged")));
        assert!(!request.answered_by(&answer("")));
    }
}
