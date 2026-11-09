//! `quiver-core-client`'s settings store (named JSON files) over `quiver-platform`'s one-file store,
//! plus the file-backed key-value store `quiver-state` uses where the web app used `localStorage`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use quiver_core_client::stores::SettingsStore;
use quiver_state::persistence::KeyValueStore;

/// One `quiver_platform::SettingsStore` per file name, opened lazily under the app data dir, so
/// `connections.json` and `device.json` keep the names and locations the Tauri app used.
pub struct FileSettings {
    dir: PathBuf,
    open: Mutex<HashMap<String, std::sync::Arc<quiver_platform::SettingsStore>>>,
}

impl FileSettings {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            open: Mutex::default(),
        }
    }

    fn file(
        &self,
        name: &str,
    ) -> quiver_platform::Result<std::sync::Arc<quiver_platform::SettingsStore>> {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(store) = open.get(name) {
            return Ok(store.clone());
        }
        let store = std::sync::Arc::new(quiver_platform::SettingsStore::open(self.dir.join(name))?);
        open.insert(name.to_string(), store.clone());
        Ok(store)
    }
}

impl SettingsStore for FileSettings {
    fn get(&self, file: &str, key: &str) -> Option<serde_json::Value> {
        match self.file(file) {
            Ok(store) => store.get_or_none(key),
            Err(e) => {
                log::error!("settings: {e}");
                None
            }
        }
    }

    fn set(
        &self,
        file: &str,
        key: &str,
        value: serde_json::Value,
    ) -> quiver_core_client::Result<()> {
        self.file(file)
            .and_then(|store| store.set(key, &value))
            .map_err(|e| quiver_core_client::Error::Settings(e.to_string()))
    }
}

/// `quiver-state`'s string store in a settings file (the state's cache version marker, the
/// self-announced rows, the cached catalog). A failure is returned and the caller carries on.
pub struct FileKeyValue {
    store: quiver_platform::Result<quiver_platform::SettingsStore>,
}

impl FileKeyValue {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self {
            store: quiver_platform::SettingsStore::open(path),
        }
    }
}

impl KeyValueStore for FileKeyValue {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        let store = self.store.as_ref().map_err(ToString::to_string)?;
        store.get::<String>(key).map_err(|e| e.to_string())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let store = self.store.as_ref().map_err(ToString::to_string)?;
        store.set(key, &value).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_land_in_the_named_file_and_survive_a_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let settings = FileSettings::new(dir.path());
        settings
            .set("connections.json", "connections", serde_json::json!([1, 2]))
            .unwrap();
        assert!(dir.path().join("connections.json").exists());

        let again = FileSettings::new(dir.path());
        assert_eq!(
            again.get("connections.json", "connections"),
            Some(serde_json::json!([1, 2]))
        );
        assert_eq!(again.get("device.json", "device_id"), None);
    }

    #[test]
    fn a_corrupt_file_reads_as_absent_and_refuses_writes_instead_of_replacing_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("device.json"), "[not an object").unwrap();
        let settings = FileSettings::new(dir.path());
        assert_eq!(settings.get("device.json", "device_id"), None);
        assert!(settings
            .set("device.json", "device_id", serde_json::json!("x"))
            .is_err());
    }

    #[test]
    fn the_key_value_store_round_trips_strings_and_reports_a_bad_file() {
        let dir = tempfile::tempdir().unwrap();
        let kv = FileKeyValue::open(dir.path().join("state.json"));
        assert_eq!(kv.get("k").unwrap(), None);
        kv.set("k", "v").unwrap();
        assert_eq!(kv.get("k").unwrap().as_deref(), Some("v"));

        std::fs::write(dir.path().join("bad.json"), "nope").unwrap();
        let bad = FileKeyValue::open(dir.path().join("bad.json"));
        assert!(bad.get("k").is_err());
        assert!(bad.set("k", "v").is_err());
    }
}
