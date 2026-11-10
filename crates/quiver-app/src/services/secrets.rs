//! `quiver-core-client`'s secret store over `quiver-platform`'s (the OS keyring in the app).

use std::sync::Arc;

use quiver_core_client::stores::SecretStore;
use quiver_core_client::Error;

/// Adapts any `quiver_platform::SecretStore`; `KeyringStore` in the app, `MemorySecretStore` in tests.
pub struct KeyringSecrets {
    inner: Arc<dyn quiver_platform::SecretStore>,
}

impl KeyringSecrets {
    pub fn new(inner: Arc<dyn quiver_platform::SecretStore>) -> Self {
        Self { inner }
    }

    /// The OS keyring under the service name the Tauri app used.
    pub fn keyring() -> Self {
        Self::new(Arc::new(quiver_platform::KeyringStore::new()))
    }
}

impl SecretStore for KeyringSecrets {
    fn get(
        &self,
        connection_id: &str,
    ) -> quiver_core_client::Result<String> {
        match self.inner.get(connection_id) {
            Ok(Some(token)) => Ok(token),
            Ok(None) => Err(Error::Secret(format!(
                "no saved token for connection {connection_id}"
            ))),
            Err(e) => Err(Error::Secret(e.to_string())),
        }
    }

    fn set(
        &self,
        connection_id: &str,
        token: &str,
    ) -> quiver_core_client::Result<()> {
        self.inner
            .set(connection_id, token)
            .map_err(|e| Error::Secret(e.to_string()))
    }

    fn delete(
        &self,
        connection_id: &str,
    ) -> quiver_core_client::Result<()> {
        self.inner
            .delete(connection_id)
            .map_err(|e| Error::Secret(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secrets() -> KeyringSecrets {
        KeyringSecrets::new(Arc::new(quiver_platform::MemorySecretStore::new()))
    }

    #[test]
    fn a_stored_token_is_returned_and_a_missing_one_is_an_error() {
        let s = secrets();
        assert!(s.get("c1").is_err());
        s.set("c1", "tok").unwrap();
        assert_eq!(s.get("c1").unwrap(), "tok");
    }

    #[test]
    fn deleting_forgets_the_token_and_deleting_nothing_is_fine() {
        let s = secrets();
        s.set("c1", "tok").unwrap();
        s.delete("c1").unwrap();
        s.delete("c1").unwrap();
        assert!(s.get("c1").is_err());
    }
}
