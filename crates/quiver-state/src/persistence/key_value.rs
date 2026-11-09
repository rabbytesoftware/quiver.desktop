use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

/// A tiny string store (what `localStorage` was): the app backs it with a file, tests with memory.
/// Both directions may fail (private mode, full quota, unreadable file); callers treat a failure
/// as "nothing there" / "not saved" and carry on.
pub trait KeyValueStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
}

#[derive(Default)]
pub struct InMemoryKeyValueStore {
    map: Mutex<HashMap<String, String>>,
}

impl InMemoryKeyValueStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl KeyValueStore for InMemoryKeyValueStore {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.map.lock().unwrap_or_else(PoisonError::into_inner).get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.map.lock().unwrap_or_else(PoisonError::into_inner).insert(key.into(), value.into());
        Ok(())
    }
}
