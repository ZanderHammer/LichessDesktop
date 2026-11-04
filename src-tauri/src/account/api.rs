//! The Lichess API calls of the account connection. Status codes checked
//! against lila and lichess.org (ADR 0024). Tokens and codes never reach an
//! error message: the errors only say which request failed and how.

use std::error::Error;
use std::io::BufReader;
use std::time::Duration;

use serde::Deserialize;

use super::oauth;
use crate::update::USER_AGENT;

const TOKEN_URL: &str = "https://lichess.org/api/token";
const ACCOUNT_URL: &str = "https://lichess.org/api/account";
const EVENTS_URL: &str = "https://lichess.org/api/stream/event";
const TIMEOUT: Duration = Duration::from_secs(10);

/// Three missed keep-alives, which lila sends every 7 seconds.
const EVENTS_READ_TIMEOUT: Duration = Duration::from_secs(21);

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

/// The answer to opening the event stream.
pub enum Opened {
    /// The lines of the stream. An empty one is a keep-alive.
    Events(BufReader<ureq::BodyReader<'static>>),
    /// Lichess no longer accepts the token: it expired or was revoked.
    Rejected,
    RateLimited,
    Failed(u16),
}

/// Opens the event stream. Opening a new one closes the token's previous one.
pub fn open_events(token: &str) -> Result<Opened, ureq::Error> {
    open_events_at(&stream_agent(EVENTS_READ_TIMEOUT), EVENTS_URL, token)
}

/// The body never ends, so unlike [`agent`] this has no overall limit. A
/// connection that went silent, such as after a suspend, fails its next read.
fn stream_agent(read_timeout: Duration) -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_resolve(Some(TIMEOUT))
            .timeout_connect(Some(TIMEOUT))
            .timeout_per_read(Some(read_timeout))
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .build(),
    )
}

fn open_events_at(agent: &ureq::Agent, url: &str, token: &str) -> Result<Opened, ureq::Error> {
    let response = agent
        .get(url)
        .header("Authorization", bearer(token))
        .header("Accept", "application/x-ndjson")
        .call()?;
    let status = response.status();
    Ok(if status.is_success() {
        Opened::Events(BufReader::new(response.into_body().into_reader()))
    } else {
        opened_from_status(status.as_u16())
    })
}

fn opened_from_status(status: u16) -> Opened {
    match status {
        401 => Opened::Rejected,
        429 => Opened::RateLimited,
        status => Opened::Failed(status),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, ErrorKind, Read, Write};
    use std::net::TcpListener;
    use std::thread;

    /// A local server that answers one request with `head` and `chunks`, then
    /// keeps the connection open without sending anything until the test ends.
    fn serve(head: &'static str, chunks: &'static [&'static str]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            write!(socket, "{head}\r\n\r\n").unwrap();
            for chunk in chunks {
                write!(socket, "{:x}\r\n{chunk}\r\n", chunk.len()).unwrap();
            }
            socket.flush().unwrap();
            thread::sleep(Duration::from_secs(30));
        });
        format!("http://{address}/api/stream/event")
    }

    #[test]
    fn events_arrive_as_they_are_sent_and_a_silent_connection_fails() {
        let url = serve(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked",
            &["{\"type\":\"gameStart\"}\n", "\n"],
        );
        let agent = stream_agent(Duration::from_millis(300));
        let Ok(Opened::Events(mut lines)) = open_events_at(&agent, &url, "t") else {
            panic!("the stream didn't open");
        };
        let mut line = String::new();
        lines.read_line(&mut line).unwrap();
        assert_eq!(line, "{\"type\":\"gameStart\"}\n");
        line.clear();
        lines.read_line(&mut line).unwrap();
        assert_eq!(line, "\n");

        let started = std::time::Instant::now();
        let silence = lines.read_line(&mut line).unwrap_err();
        assert!(
            matches!(silence.kind(), ErrorKind::TimedOut | ErrorKind::Other),
            "{silence}"
        );
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_401_on_the_stream_is_seen_as_a_rejection() {
        let url = serve("HTTP/1.1 401 Unauthorized\r\nContent-Length: 0", &[]);
        let agent = stream_agent(Duration::from_secs(5));
        assert!(matches!(
            open_events_at(&agent, &url, "t"),
            Ok(Opened::Rejected)
        ));
    }

    #[test]
    fn the_stream_tells_rejection_from_rate_limit_and_outage() {
        assert!(matches!(opened_from_status(401), Opened::Rejected));
        assert!(matches!(opened_from_status(429), Opened::RateLimited));
        for status in [403, 500, 503] {
            assert!(matches!(opened_from_status(status), Opened::Failed(s) if s == status));
        }
    }

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
