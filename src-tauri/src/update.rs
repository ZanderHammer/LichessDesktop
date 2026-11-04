//! Update notice (#17, ADR 0017): no auto-updater, only a hint. At startup and
//! when "Check for updates" is turned on (ADR 0019), a background thread asks
//! GitHub for the latest release, at most once per 24 hours. If it is newer
//! than this build, one native notification is shown per new version, and the
//! tray and the About tab offer its release page.
//!
//! What the last check found is kept in its own store file, apart from the
//! settings, so a restart within 24 hours sends no request.

use std::sync::RwLock;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use semver::Version;
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime, Url};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::{Store, StoreExt};

use crate::i18n::{self, Key};
use crate::settings::SettingsStore;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/ZanderHammer/LichessDesktop/releases/latest";
/// GitHub rejects API requests without one. Names the app, nothing else.
const USER_AGENT: &str = concat!("LichessDesktop/", env!("CARGO_PKG_VERSION"));
const TIMEOUT: Duration = Duration::from_secs(10);
/// Minimum time between two requests.
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

const STORE_FILE: &str = "update.json";
/// Unix time in seconds of the last successful check.
const CHECKED_AT: &str = "checked_at";
/// The latest release the last check found, e.g. `0.3.0`.
const LATEST: &str = "latest";
/// The newest version a notification was shown for.
const NOTIFIED: &str = "notified";

/// Tells an open settings window that [`status`] changed.
pub const CHANGED: &str = "update-changed";

/// Tells our notification apart from later ones in `on_action`.
const NOTIFICATION_ID: i32 = 17_000;

/// The current [`Status`]. In managed state.
#[derive(Default)]
struct State(RwLock<Status>);

/// What is known about updates right now.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    /// "Check for updates" is off.
    #[default]
    Off,
    Checking,
    UpToDate,
    /// No answer: offline, rate limited or a bad response.
    Failed,
    Available(Update),
}

/// A release newer than the running build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    version: Version,
}

impl Update {
    /// The version as shown to the user: `v0.3.0`, like the tag.
    pub fn label(&self) -> String {
        format!("v{}", self.version)
    }

    /// The release page on GitHub. Built from the repository and the parsed
    /// version, never from the response, so it can only point there.
    pub fn url(&self) -> Url {
        format!("{REPOSITORY}/releases/tag/{}", self.label())
            .parse()
            .expect("the release URL is valid")
    }

    /// "Update available: v0.3.0" in the UI language.
    pub fn text(&self, strings: &i18n::Strings) -> String {
        strings.format(Key::UpdateAvailable, &self.label())
    }
}

pub fn status<R: Runtime>(app: &AppHandle<R>) -> Status {
    app.try_state::<State>().map_or(Status::Off, |state| {
        state
            .0
            .read()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    })
}

/// The newer release found by the last check, if any.
pub fn available<R: Runtime>(app: &AppHandle<R>) -> Option<Update> {
    match status(app) {
        Status::Available(update) => Some(update),
        _ => None,
    }
}

/// Opens the release page of [`available`] in the browser.
pub fn open_release_page<R: Runtime>(app: &AppHandle<R>) {
    if let Some(update) = available(app) {
        crate::open_in_browser(app, &update.url());
    }
}

/// Shows what the last check found and, when it is due, checks again in the
/// background. Does nothing while "Check for updates" is off. Needs the
/// store and notification plugins and [`crate::settings::init`].
pub fn init<R: Runtime>(app: &AppHandle<R>) {
    app.manage(State::default());
    let handle = app.clone();
    let _ = app.notification().on_action(move |action| {
        if action.action_id() == "tap"
            && action.notification().map(|n| n.id()) == Some(NOTIFICATION_ID)
        {
            open_release_page(&handle);
        }
    });
    if app.state::<SettingsStore<R>>().get().check_updates {
        start_check(app);
    }
}

/// Applies a new value of "Check for updates" at once: on starts a check, off
/// removes the update entry.
pub fn setting_changed<R: Runtime>(app: &AppHandle<R>, on: bool) {
    if on {
        start_check(app);
    } else {
        set_status(app, Status::Off);
    }
}

fn start_check<R: Runtime>(app: &AppHandle<R>) {
    set_status(app, Status::Checking);
    let app = app.clone();
    thread::spawn(move || check(&app));
}

fn check<R: Runtime>(app: &AppHandle<R>) {
    let latest = latest_release(app);
    // Turned off while the request was running.
    if !app.state::<SettingsStore<R>>().get().check_updates {
        return;
    }
    let latest = match latest {
        Ok(latest) => latest,
        Err(err) => {
            eprintln!("update check failed: {err}");
            set_status(app, Status::Failed);
            return;
        }
    };
    if !is_newer(&latest, &current_version()) {
        set_status(app, Status::UpToDate);
        return;
    }
    let update = Update { version: latest };
    set_status(app, Status::Available(update.clone()));
    if let Err(err) = notify(app, &update) {
        eprintln!("failed to show the update notification: {err}");
    }
}

/// The latest release: the stored result of the last check, or from GitHub
/// when a check is due.
fn latest_release<R: Runtime>(app: &AppHandle<R>) -> Result<Version, Box<dyn std::error::Error>> {
    let store = app.store(STORE_FILE)?;
    let now = unix_now();
    let checked_at = store.get(CHECKED_AT).and_then(|value| value.as_u64());
    if !is_due(checked_at, now)
        && let Some(latest) = stored_version(&store, LATEST)
    {
        return Ok(latest);
    }
    let latest = fetch_latest()?;
    store.set(CHECKED_AT, now);
    store.set(LATEST, latest.to_string());
    store.save()?;
    Ok(latest)
}

/// Shows the notification, unless one was shown for this version already.
fn notify<R: Runtime>(
    app: &AppHandle<R>,
    update: &Update,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = app.store(STORE_FILE)?;
    if should_notify(&update.version, stored_version(&store, NOTIFIED).as_ref()) {
        app.notification()
            .builder()
            .id(NOTIFICATION_ID)
            .body(update.text(&app.state::<i18n::Current>().get()))
            .show()?;
        store.set(NOTIFIED, update.version.to_string());
        store.save()?;
    }
    Ok(())
}

fn stored_version<R: Runtime>(store: &Store<R>, key: &str) -> Option<Version> {
    store
        .get(key)
        .as_ref()
        .and_then(Value::as_str)
        .and_then(parse_tag)
}

/// Sets the status and tells an open settings window. The tray menu is rebuilt
/// only if its update entry comes or goes.
fn set_status<R: Runtime>(app: &AppHandle<R>, status: Status) {
    let Some(state) = app.try_state::<State>() else {
        return;
    };
    let old = std::mem::replace(
        &mut *state.0.write().unwrap_or_else(|err| err.into_inner()),
        status.clone(),
    );
    if matches!(old, Status::Available(_)) != matches!(status, Status::Available(_))
        && let Err(err) = crate::tray::refresh_menu(app)
    {
        eprintln!("failed to rebuild the tray menu: {err}");
    }
    if app
        .get_webview_window(crate::settings_window::LABEL)
        .is_some()
    {
        let _ = app.emit_to(crate::settings_window::LABEL, CHANGED, ());
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

/// The version of the latest release on GitHub. Drafts and prereleases are
/// not returned by this endpoint.
fn fetch_latest() -> Result<Version, Box<dyn std::error::Error>> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(USER_AGENT)
        .build();
    let release: Release = ureq::Agent::new_with_config(config)
        .get(LATEST_RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()?
        .body_mut()
        .read_json()?;
    parse_tag(&release.tag_name)
        .ok_or_else(|| format!("release tag {:?} is not a version", release.tag_name).into())
}

fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("the crate version is SemVer")
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// A release tag (`v0.3.0`) or a stored version (`0.3.0`) as a version.
fn parse_tag(tag: &str) -> Option<Version> {
    Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()
}

/// SemVer order: a prerelease is older than its release, and build metadata
/// doesn't count.
fn is_newer(latest: &Version, current: &Version) -> bool {
    latest.cmp_precedence(current).is_gt()
}

/// Whether the last successful check is [`INTERVAL`] old or more. A check
/// "in the future" (the clock was turned back) counts as due.
fn is_due(checked_at: Option<u64>, now: u64) -> bool {
    checked_at.is_none_or(|checked_at| checked_at > now || now - checked_at >= INTERVAL.as_secs())
}

/// At most one notification per new version: only if no notification was
/// shown yet for this version or a newer one.
fn should_notify(latest: &Version, notified: Option<&Version>) -> bool {
    notified.is_none_or(|notified| is_newer(latest, notified))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(version: &str) -> Version {
        Version::parse(version).unwrap()
    }

    #[test]
    fn parses_tags_and_stored_versions() {
        assert_eq!(parse_tag("v0.3.0"), Some(v("0.3.0")));
        assert_eq!(parse_tag("0.3.0"), Some(v("0.3.0")));
        assert_eq!(parse_tag("v1.0.0-rc.1"), Some(v("1.0.0-rc.1")));
        for bad in ["", "v", "latest", "v1.0", "vv1.0.0", "1.0.0 "] {
            assert_eq!(parse_tag(bad), None, "parsed {bad:?}");
        }
    }

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer(&v("0.3.0"), &v("0.2.0")));
        assert!(is_newer(&v("0.10.0"), &v("0.9.0")));
        assert!(is_newer(&v("0.2.1"), &v("0.2.0")));
        assert!(is_newer(&v("1.0.0"), &v("0.99.99")));
        assert!(!is_newer(&v("0.2.0"), &v("0.2.0")));
        assert!(!is_newer(&v("0.1.9"), &v("0.2.0")));
    }

    #[test]
    fn a_prerelease_is_older_than_its_release() {
        assert!(is_newer(&v("1.0.0"), &v("1.0.0-rc.1")));
        assert!(!is_newer(&v("1.0.0-rc.1"), &v("1.0.0")));
        assert!(is_newer(&v("1.0.0-rc.2"), &v("1.0.0-rc.1")));
    }

    #[test]
    fn build_metadata_does_not_count() {
        assert!(!is_newer(&v("0.2.0+build.2"), &v("0.2.0+build.1")));
    }

    #[test]
    fn checks_at_most_once_per_interval() {
        let day = INTERVAL.as_secs();
        assert!(is_due(None, 1_000));
        assert!(!is_due(Some(1_000), 1_000));
        assert!(!is_due(Some(1_000), 1_000 + day - 1));
        assert!(is_due(Some(1_000), 1_000 + day));
        assert!(is_due(Some(5_000), 1_000), "clock turned back");
    }

    #[test]
    fn notifies_once_per_new_version() {
        assert!(should_notify(&v("0.3.0"), None));
        assert!(!should_notify(&v("0.3.0"), Some(&v("0.3.0"))));
        assert!(should_notify(&v("0.4.0"), Some(&v("0.3.0"))));
        assert!(!should_notify(&v("0.3.0"), Some(&v("0.4.0"))));
    }

    #[test]
    fn turning_the_setting_off_removes_the_update_entry() {
        let app = tauri::test::mock_app();
        app.manage(State::default());
        let app = app.handle();
        let update = Update {
            version: v("0.3.0"),
        };
        set_status(app, Status::Available(update.clone()));
        assert_eq!(available(app), Some(update));

        setting_changed(app, false);
        assert_eq!(status(app), Status::Off);
        assert_eq!(available(app), None);
    }

    #[test]
    fn current_version_parses() {
        assert_eq!(current_version().to_string(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn api_and_release_page_belong_to_the_repository() {
        let repo = REPOSITORY.strip_prefix("https://github.com/").unwrap();
        assert_eq!(
            LATEST_RELEASE_API,
            format!("https://api.github.com/repos/{repo}/releases/latest")
        );
        let update = Update {
            version: v("0.3.0"),
        };
        assert_eq!(update.label(), "v0.3.0");
        assert_eq!(
            update.url().as_str(),
            format!("{REPOSITORY}/releases/tag/v0.3.0")
        );
    }

    #[test]
    fn text_names_the_version() {
        let strings = i18n::Strings::load(i18n::BASE);
        let update = Update {
            version: v("0.3.0"),
        };
        assert_eq!(update.text(&strings), "Update available: v0.3.0");
    }
}
