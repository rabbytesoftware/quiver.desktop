//! In-memory stores, for tests and for hosts with nothing better yet.

use std::collections::HashMap;
use std::sync::Mutex;

use super::{SecretStore, SettingsStore};
use crate::{Error, Result};

#[derive(Default)]
pub struct MemorySecretStore {
    tokens: Mutex<HashMap<String, String>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemorySecretStore {
    fn get(
        &self,
        connection_id: &str,
    ) -> Result<String> {
        self.tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(connection_id)
            .cloned()
            .ok_or_else(|| Error::Secret(format!("no token stored for {connection_id}")))
    }

    fn set(
        &self,
        connection_id: &str,
        token: &str,
    ) -> Result<()> {
        self.tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(connection_id.to_string(), token.to_string());
        Ok(())
    }

    fn delete(
        &self,
        connection_id: &str,
    ) -> Result<()> {
        self.tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(connection_id);
        Ok(())
    }
}

#[derive(Default)]
pub struct MemorySettingsStore {
    values: Mutex<HashMap<(String, String), serde_json::Value>>,
}

impl MemorySettingsStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SettingsStore for MemorySettingsStore {
    fn get(
        &self,
        file: &str,
        key: &str,
    ) -> Option<serde_json::Value> {
        self.values
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&(file.to_string(), key.to_string()))
            .cloned()
    }

    fn set(
        &self,
        file: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<()> {
        self.values
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((file.to_string(), key.to_string()), value);
        Ok(())
    }
}
