//! Keeps `/api/stream/event` open on a thread of its own (#25, ADR 0025) and
//! reconnects when it drops. Dropping the sender that goes with `stop` ends the
//! thread: at the next line or keep-alive if it is reading, at once if waiting.

use std::io::BufRead;
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, Runtime};

use super::api::{self, Account, Opened};
use super::events::{Alert, Seen};
use super::token_store;
use super::{Notice, Status};
use crate::i18n;
use crate::notifications;

const FIRST_RETRY: Duration = Duration::from_secs(2);
const LAST_RETRY: Duration = Duration::from_secs(5 * 60);

/// A connection that lasted this long starts the backoff over. Lila allows 30
/// connections per 10 minutes, and a 429 ends a connection right away.
const STABLE_AFTER: Duration = Duration::from_secs(60);

/// Lichess's docs: after a 429, a minute is enough in most cases. The backoff
/// keeps growing in case it isn't.
const AFTER_RATE_LIMIT: Duration = Duration::from_secs(60);

/// Doubling waits between attempts.
#[derive(Debug)]
struct Backoff(Duration);

impl Backoff {
    fn new() -> Self {
        Self(FIRST_RETRY)
    }

    fn next(&mut self) -> Duration {
        let wait = self.0;
        self.0 = (wait * 2).min(LAST_RETRY);
        wait
    }

    fn reset(&mut self) {
        self.0 = FIRST_RETRY;
    }
}

/// Whether the sender is gone. It never sends anything.
fn is_stopped(stop: &Receiver<()>) -> bool {
    stop.try_recv() == Err(TryRecvError::Disconnected)
}

/// Waits for `duration`. Returns `false` if told to stop in the meantime.
fn wait(stop: &Receiver<()>, duration: Duration) -> bool {
    stop.recv_timeout(duration) == Err(RecvTimeoutError::Timeout)
}

/// Runs until `stop` says so, or until lichess rejects the token. `username`
/// is `None` if lichess.org couldn't say whose token it is yet.
pub fn run<R: Runtime>(app: &AppHandle<R>, stop: &Receiver<()>, username: Option<String>) {
    // Read once, so a reconnect can't bring up the store's unlock prompt.
    let token = match token_store::read() {
        Ok(Some(token)) => token,
        Ok(None) => {
            if !is_stopped(stop) {
                super::token_rejected(app);
            }
            return;
        }
        Err(err) => {
            eprintln!("failed to read the token for the event stream: {err}");
            // Otherwise the account looks connected with no stream behind it.
            if !is_stopped(stop) {
                super::set_status(
                    app,
                    Status::Disconnected {
                        notice: Some(Notice::NoKeyring),
                    },
                );
            }
            return;
        }
    };
    let mut backoff = Backoff::new();
    let Some(username) = username.or_else(|| find_username(app, stop, &token, &mut backoff)) else {
        return;
    };
    let mut seen = Seen::new(&username);
    backoff.reset();
    loop {
        if is_stopped(stop) {
            return;
        }
        let started = Instant::now();
        let mut least_wait = Duration::ZERO;
        match api::open_events(&token) {
            Ok(Opened::Events(lines)) => {
                if let Err(err) = read_events(app, stop, lines, &mut seen) {
                    eprintln!("the event stream broke: {err}");
                }
            }
            Ok(Opened::Rejected) => {
                if !is_stopped(stop) {
                    super::token_rejected(app);
                }
                return;
            }
            Ok(Opened::RateLimited) => {
                eprintln!("the event stream is rate limited");
                least_wait = AFTER_RATE_LIMIT;
            }
            Ok(Opened::Failed(status)) => eprintln!("the event stream got {status}"),
            Err(err) => eprintln!("failed to open the event stream: {err}"),
        }
        if started.elapsed() >= STABLE_AFTER {
            backoff.reset();
        }
        if !wait(stop, backoff.next().max(least_wait)) {
            return;
        }
    }
}

/// Asks lichess.org until it answers, which also settles the account's status.
/// Outgoing challenges can't be told apart without the username.
fn find_username<R: Runtime>(
    app: &AppHandle<R>,
    stop: &Receiver<()>,
    token: &str,
    backoff: &mut Backoff,
) -> Option<String> {
    loop {
        match api::account(token) {
            Account::Username(username) => {
                super::token_verified(app, &username);
                return Some(username);
            }
            Account::Rejected => {
                if !is_stopped(stop) {
                    super::token_rejected(app);
                }
                return None;
            }
            Account::Unknown => {
                if !wait(stop, backoff.next()) {
                    return None;
                }
            }
        }
    }
}

/// Reads until the connection ends or `stop` says so.
fn read_events<R: Runtime>(
    app: &AppHandle<R>,
    stop: &Receiver<()>,
    lines: impl BufRead,
    seen: &mut Seen,
) -> std::io::Result<()> {
    for line in lines.lines() {
        let line = line?;
        if is_stopped(stop) {
            return Ok(());
        }
        if let Some(alert) = seen.line(&line) {
            notify(app, &alert);
        }
    }
    Ok(())
}

fn notify<R: Runtime>(app: &AppHandle<R>, alert: &Alert) {
    let body = alert.body(&app.state::<i18n::Current>().get());
    match alert.url() {
        Some(url) => notifications::account_event(app, body, &url),
        None => eprintln!("skipped an account event without a usable address"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn the_wait_doubles_up_to_a_limit() {
        let mut backoff = Backoff::new();
        let waits: Vec<u64> = (0..9).map(|_| backoff.next().as_secs()).collect();
        assert_eq!(waits, [2, 4, 8, 16, 32, 64, 128, 256, 300]);
        assert_eq!(backoff.next(), LAST_RETRY);
        backoff.reset();
        assert_eq!(backoff.next(), FIRST_RETRY);
    }

    #[test]
    fn the_backoff_stays_within_lilas_limit() {
        // Lila allows 30 connections per 10 minutes.
        let mut backoff = Backoff::new();
        let (mut attempts, mut elapsed) = (1, Duration::ZERO);
        while elapsed < Duration::from_secs(10 * 60) {
            elapsed += backoff.next();
            attempts += 1;
        }
        assert!(attempts <= 30, "{attempts}");
    }

    #[test]
    fn dropping_the_sender_stops_a_wait_at_once() {
        let (sender, stop) = mpsc::channel();
        assert!(!is_stopped(&stop));
        assert!(wait(&stop, Duration::from_millis(1)), "kept going");
        drop(sender);
        assert!(is_stopped(&stop));
        let started = Instant::now();
        assert!(!wait(&stop, Duration::from_secs(60)));
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
