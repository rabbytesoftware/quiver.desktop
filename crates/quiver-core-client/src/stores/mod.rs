//! What the host application provides so this crate needs no OS keyring and
//! no settings file of its own.

mod memory;

pub use memory::{MemorySecretStore, MemorySettingsStore};

use crate::Result;

/// Where bearer tokens of saved remote connections live (the OS keyring in
/// the app), keyed by connection id.
pub trait SecretStore: Send + Sync + 'static {
    fn get(
        &self,
        connection_id: &str,
    ) -> Result<String>;
    fn set(
        &self,
        connection_id: &str,
        token: &str,
    ) -> Result<()>;
    fn delete(
        &self,
        connection_id: &str,
    ) -> Result<()>;
}

/// A small JSON key-value settings file set (the Tauri store plugin in the
/// old app): `file` is a store name such as `connections.json`.
pub trait SettingsStore: Send + Sync + 'static {
    fn get(
        &self,
        file: &str,
        key: &str,
    ) -> Option<serde_json::Value>;
    /// Set the value and persist it.
    fn set(
        &self,
        file: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<()>;
}
