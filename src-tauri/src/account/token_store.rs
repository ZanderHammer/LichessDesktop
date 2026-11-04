//! The access token lives only in the OS credential store, and there is no
//! fallback to a file (ADR 0024). Each call blocks and may show the system's
//! unlock prompt, so call these from a background thread.

use std::sync::Arc;

use keyring_core::{CredentialStore, Entry, Error, Result};

/// The credential's name in the store: the app identifier and what it holds.
const SERVICE: &str = "de.itshammer.lichessdesktop";
const USER: &str = "lichess-access-token";

#[cfg(target_os = "linux")]
fn native() -> Result<Arc<CredentialStore>> {
    Ok(zbus_secret_service_keyring_store::Store::new()?)
}

#[cfg(target_os = "macos")]
fn native() -> Result<Arc<CredentialStore>> {
    Ok(apple_native_keyring_store::keychain::Store::new()?)
}

#[cfg(windows)]
fn native() -> Result<Arc<CredentialStore>> {
    Ok(windows_native_keyring_store::Store::new()?)
}

/// The stored token, if any. An error means the store can't be used: there is
/// none, it stayed locked, or it failed.
pub fn read() -> Result<Option<String>> {
    read_from(&*native()?)
}

/// Saves the token, replacing an older one.
pub fn write(token: &str) -> Result<()> {
    write_to(&*native()?, token)
}

/// Removes the token. Having none is fine.
pub fn remove() -> Result<()> {
    remove_from(&*native()?)
}

fn entry(store: &CredentialStore) -> Result<Entry> {
    store.build(SERVICE, USER, None)
}

fn read_from(store: &CredentialStore) -> Result<Option<String>> {
    match entry(store)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(Error::NoEntry) => Ok(None),
        Err(err) => Err(err),
    }
}

fn write_to(store: &CredentialStore, token: &str) -> Result<()> {
    entry(store)?.set_password(token)
}

fn remove_from(store: &CredentialStore) -> Result<()> {
    match entry(store)?.delete_credential() {
        Err(Error::NoEntry) => Ok(()),
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyring_core::mock;

    #[test]
    fn writes_reads_and_removes_the_token() {
        let store = mock::Store::new().unwrap();
        assert_eq!(read_from(&*store).unwrap(), None);
        write_to(&*store, "lio_a").unwrap();
        assert_eq!(read_from(&*store).unwrap().as_deref(), Some("lio_a"));
        write_to(&*store, "lio_b").unwrap();
        assert_eq!(read_from(&*store).unwrap().as_deref(), Some("lio_b"));
        remove_from(&*store).unwrap();
        assert_eq!(read_from(&*store).unwrap(), None);
        remove_from(&*store).unwrap();
    }

    #[test]
    fn a_failing_store_is_an_error_and_not_a_missing_token() {
        let store = mock::Store::new().unwrap();
        let entry = entry(&*store).unwrap();
        let cred: &mock::Cred = entry.as_any().downcast_ref().unwrap();
        cred.set_error(Error::NoStorageAccess("locked".into()));
        assert!(read_from(&*store).is_err());
    }

    /// Writes one throwaway credential to the real store of this machine and
    /// removes it again: `cargo test -- --ignored real_store`.
    #[test]
    #[ignore = "needs an unlocked system keyring"]
    fn real_store_round_trip() {
        let store = native().unwrap();
        let entry = store
            .build("de.itshammer.lichessdesktop.test", USER, None)
            .unwrap();
        entry.set_password("test-value").unwrap();
        assert_eq!(entry.get_password().unwrap(), "test-value");
        entry.delete_credential().unwrap();
        assert!(matches!(entry.get_password(), Err(Error::NoEntry)));
    }
}
