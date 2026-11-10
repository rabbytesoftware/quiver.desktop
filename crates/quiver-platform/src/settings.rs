//! A JSON key/value file, replacing `tauri-plugin-store`.
//!
//! The file format is the plugin's: one JSON object, key to value, so the
//! `connections.json` and `device.json` an existing install already has are
//! read as they are. Every write goes to a sibling temp file and is renamed
//! over the original, so a crash mid-write leaves the old file, never half of
//! a new one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::{Error, Result};

/// File names the Tauri app used, under [`crate::AppPaths::data_dir`].
pub const CONNECTIONS_FILE: &str = "connections.json";
pub const DEVICE_FILE: &str = "device.json";
/// Key of the saved remote connections inside [`CONNECTIONS_FILE`].
pub const CONNECTIONS_KEY: &str = "connections";
/// Key of this installation's pairing id inside [`DEVICE_FILE`].
pub const DEVICE_ID_KEY: &str = "device_id";

#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
    entries: Mutex<BTreeMap<String, Value>>,
}

impl SettingsStore {
    /// Opens `path`, which may not exist yet (an empty store). A file that is
    /// not a JSON object is an error rather than being silently replaced.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let entries = match std::fs::read_to_string(&path) {
            Ok(text) if text.trim().is_empty() => BTreeMap::new(),
            Ok(text) => serde_json::from_str(&text).map_err(|source| Error::Json {
                path: path.clone(),
                source,
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(source) => return Err(Error::Io { path, source }),
        };
        Ok(Self {
            path,
            entries: Mutex::new(entries),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Value>> {
        // Behind the lock is a plain map a panic cannot leave half-updated.
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The value at `key`; `None` when absent. A value of the wrong shape is
    /// an error.
    pub fn get<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>> {
        let Some(value) = self.lock().get(key).cloned() else {
            return Ok(None);
        };
        serde_json::from_value(value)
            .map(Some)
            .map_err(|source| Error::Value {
                key: key.to_owned(),
                source,
            })
    }

    /// Like [`Self::get`], but a missing or unreadable value is `None`, which
    /// is how the Tauri app treated its saved connections.
    pub fn get_or_none<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Option<T> {
        self.get(key).ok().flatten()
    }

    /// Sets `key` and writes the file.
    pub fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<()> {
        let value = serde_json::to_value(value).map_err(|source| Error::Value {
            key: key.to_owned(),
            source,
        })?;
        let mut entries = self.lock();
        entries.insert(key.to_owned(), value);
        self.write(&entries)
    }

    /// Removes `key` and writes the file. Returns whether it existed.
    pub fn remove(
        &self,
        key: &str,
    ) -> Result<bool> {
        let mut entries = self.lock();
        let existed = entries.remove(key).is_some();
        if existed {
            self.write(&entries)?;
        }
        Ok(existed)
    }

    fn write(
        &self,
        entries: &BTreeMap<String, Value>,
    ) -> Result<()> {
        let io = |source| Error::Io {
            path: self.path.clone(),
            source,
        };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let text = serde_json::to_string_pretty(entries).map_err(|source| Error::Json {
            path: self.path.clone(),
            source,
        })?;
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        std::fs::write(&tmp, text).map_err(io)?;
        std::fs::rename(&tmp, &self.path).map_err(io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_is_an_empty_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::open(dir.path().join("none.json")).unwrap();
        assert_eq!(store.get::<String>("k").unwrap(), None);
    }

    #[test]
    fn values_round_trip_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/s.json");
        let store = SettingsStore::open(&path).unwrap();
        store.set("device_id", &"abc").unwrap();
        store.set("list", &vec![1, 2, 3]).unwrap();

        let reopened = SettingsStore::open(&path).unwrap();
        assert_eq!(
            reopened.get::<String>("device_id").unwrap().as_deref(),
            Some("abc")
        );
        assert_eq!(
            reopened.get::<Vec<i32>>("list").unwrap(),
            Some(vec![1, 2, 3])
        );
        assert!(!dir.path().join("nested/s.json.tmp").exists());
    }

    /// The format an install made by the Tauri build already has on disk.
    #[test]
    fn a_file_written_by_tauri_plugin_store_is_read_as_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(DEVICE_FILE);
        std::fs::write(&path, r#"{"device_id":"7d1f"}"#).unwrap();
        let store = SettingsStore::open(&path).unwrap();
        assert_eq!(
            store.get::<String>(DEVICE_ID_KEY).unwrap().as_deref(),
            Some("7d1f")
        );
    }

    #[test]
    fn a_corrupt_file_is_an_error_not_a_silent_reset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.json");
        std::fs::write(&path, "{not json").unwrap();
        assert!(matches!(
            SettingsStore::open(&path),
            Err(Error::Json { .. })
        ));
    }

    #[test]
    fn a_value_of_the_wrong_shape_is_an_error_or_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::open(dir.path().join("s.json")).unwrap();
        store.set("k", &"text").unwrap();
        assert!(matches!(
            store.get::<Vec<i32>>("k"),
            Err(Error::Value { .. })
        ));
        assert_eq!(store.get_or_none::<Vec<i32>>("k"), None);
    }

    #[test]
    fn remove_deletes_the_key_from_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.json");
        let store = SettingsStore::open(&path).unwrap();
        store.set("a", &1).unwrap();
        assert!(store.remove("a").unwrap());
        assert!(!store.remove("a").unwrap());
        assert_eq!(
            SettingsStore::open(&path).unwrap().get::<i32>("a").unwrap(),
            None
        );
    }
}
