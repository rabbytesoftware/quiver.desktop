//! Secrets (the bearer token of each paired remote connection) in the OS
//! keyring.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::{Error, Result};

/// Keyring service of every secret this app stores. The account is the
/// connection id. Same as the Tauri app, so existing tokens are found.
pub const KEYRING_SERVICE: &str = "quiver.desktop";

pub trait SecretStore: Send + Sync {
    fn set(&self, account: &str, secret: &str) -> Result<()>;
    /// `None` when there is no secret under `account`.
    fn get(&self, account: &str) -> Result<Option<String>>;
    /// Deleting what is not there is not an error.
    fn delete(&self, account: &str) -> Result<()>;
}

/// The OS keyring (Keychain, Credential Manager, Secret Service).
#[derive(Debug, Clone)]
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new() -> Self {
        Self::with_service(KEYRING_SERVICE)
    }

    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, account: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, account).map_err(|e| Error::Secret(e.to_string()))
    }
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeyringStore {
    fn set(&self, account: &str, secret: &str) -> Result<()> {
        self.entry(account)?
            .set_password(secret)
            .map_err(|e| Error::Secret(e.to_string()))
    }

    fn get(&self, account: &str) -> Result<Option<String>> {
        match self.entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Error::Secret(e.to_string())),
        }
    }

    fn delete(&self, account: &str) -> Result<()> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(Error::Secret(e.to_string())),
        }
    }
}

/// For tests, and for hosts with no keyring.
#[derive(Debug, Default)]
pub struct MemorySecretStore {
    secrets: Mutex<HashMap<String, String>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.secrets.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl SecretStore for MemorySecretStore {
    fn set(&self, account: &str, secret: &str) -> Result<()> {
        self.lock().insert(account.to_owned(), secret.to_owned());
        Ok(())
    }

    fn get(&self, account: &str) -> Result<Option<String>> {
        Ok(self.lock().get(account).cloned())
    }

    fn delete(&self, account: &str) -> Result<()> {
        self.lock().remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_service_name_is_the_one_existing_secrets_live_under() {
        assert_eq!(KEYRING_SERVICE, "quiver.desktop");
    }

    #[test]
    fn the_memory_store_sets_gets_and_deletes() {
        let store = MemorySecretStore::new();
        assert_eq!(store.get("c1").unwrap(), None);
        store.set("c1", "tok").unwrap();
        assert_eq!(store.get("c1").unwrap().as_deref(), Some("tok"));
        store.set("c1", "tok2").unwrap();
        assert_eq!(store.get("c1").unwrap().as_deref(), Some("tok2"));
        store.delete("c1").unwrap();
        store.delete("c1").unwrap();
        assert_eq!(store.get("c1").unwrap(), None);
    }

    #[test]
    fn the_store_is_usable_as_a_trait_object() {
        let store: Box<dyn SecretStore> = Box::new(MemorySecretStore::new());
        store.set("a", "b").unwrap();
        assert_eq!(store.get("a").unwrap().as_deref(), Some("b"));
    }
}
