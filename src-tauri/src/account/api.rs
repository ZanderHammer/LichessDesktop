//! The Lichess API calls of the account connection. Status codes checked
//! against lila and lichess.org (ADR 0024). Tokens and codes never reach an
//! error message: the errors only say which request failed and how.

use std::error::Error;
use std::time::Duration;

use serde::Deserialize;

use super::oauth;
use crate::update::USER_AGENT;

const TOKEN_URL: &str = "https://lichess.org/api/token";
const ACCOUNT_URL: &str = "https://lichess.org/api/account";
const TIMEOUT: Duration = Duration::from_secs(10);

/// Handing status codes to the caller keeps an expired token apart from an
/// outage.
fn agent() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .build(),
    )
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Trades an authorization code for an access token.
pub fn exchange_code(code: &str, verifier: &str) -> Result<String, Box<dyn Error>> {
    let mut response = agent().post(TOKEN_URL).send_form([
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", verifier),
        ("redirect_uri", oauth::REDIRECT_URI),
        ("client_id", oauth::CLIENT_ID),
    ])?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("the token request got {status}").into());
    }
    let token: TokenResponse = response.body_mut().read_json()?;
    Ok(token.access_token)
}

/// What a token is worth right now.
#[derive(Debug, PartialEq, Eq)]
pub enum Account {
    Username(String),
    /// Lichess no longer accepts the token: it expired or was revoked.
    Rejected,
    /// No answer that says anything about the token.
    Unknown,
}

#[derive(Deserialize)]
struct AccountResponse {
    username: String,
}

/// Asks who the token belongs to. Lila asks clients not to poll this.
pub fn account(token: &str) -> Account {
    let response = agent()
        .get(ACCOUNT_URL)
        .header("Authorization", bearer(token))
        .call();
    let mut response = match response {
        Ok(response) => response,
        Err(err) => {
            eprintln!("the account request failed: {err}");
            return Account::Unknown;
        }
    };
    let status = response.status().as_u16();
    if !response.status().is_success() {
        return account_from_status(status);
    }
    match response.body_mut().read_json::<AccountResponse>() {
        Ok(account) => Account::Username(account.username),
        Err(err) => {
            eprintln!("the account response is unreadable: {err}");
            Account::Unknown
        }
    }
}

fn account_from_status(status: u16) -> Account {
    if status == 401 {
        Account::Rejected
    } else {
        eprintln!("the account request got {status}");
        Account::Unknown
    }
}

/// Revokes the token. A token lichess no longer accepts counts as revoked.
pub fn revoke(token: &str) -> Result<(), Box<dyn Error>> {
    let response = agent()
        .delete(TOKEN_URL)
        .header("Authorization", bearer(token))
        .call()?;
    let status = response.status();
    if status.is_success() || status.as_u16() == 401 {
        Ok(())
    } else {
        Err(format!("the revoke request got {status}").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_401_rejects_the_token() {
        assert_eq!(account_from_status(401), Account::Rejected);
        for status in [403, 429, 500, 503] {
            assert_eq!(account_from_status(status), Account::Unknown, "{status}");
        }
    }

    #[test]
    fn reads_the_fields_lichess_sends() {
        let token: TokenResponse = serde_json::from_str(
            r#"{"token_type":"Bearer","access_token":"lio_a","expires_in":31536000}"#,
        )
        .unwrap();
        assert_eq!(token.access_token, "lio_a");
        let account: AccountResponse =
            serde_json::from_str(r#"{"id":"someone","username":"SomeOne","perfs":{}}"#).unwrap();
        assert_eq!(account.username, "SomeOne");
    }
}
