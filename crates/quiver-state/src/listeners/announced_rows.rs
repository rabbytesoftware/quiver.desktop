use std::sync::{Arc, Mutex, PoisonError};

use crate::persistence::KeyValueStore;

const KEY_PREFIX: &str = "quiver:self-announced:";

/// The quiver.desktop identities this app's own announces registered on one daemon connection --
/// the only rows an announce may later forget. A row the user added (a pin, another channel) is
/// never on it, and core cannot tell the two apart: both are user-installed, and the
/// `preinstalled:` probe marks either ready on a machine that has the app.
pub struct AnnouncedRows {
    key: String,
    store: Arc<dyn KeyValueStore>,
    identities: Mutex<Vec<String>>,
}

impl AnnouncedRows {
    /// The record for the connection `active_id`. (`None` from the caller when the connection
    /// cannot be named, so nothing is recorded or forgotten.)
    pub fn open(store: Arc<dyn KeyValueStore>, active_id: &str) -> Self {
        let key = format!("{KEY_PREFIX}{active_id}");
        let identities = read(store.as_ref(), &key);
        Self {
            key,
            store,
            identities: Mutex::new(identities),
        }
    }

    pub fn list(&self) -> Vec<String> {
        self.identities
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn add(&self, identity: &str) {
        let mut ids = self
            .identities
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if !ids.iter().any(|known| known == identity) {
            ids.push(identity.to_string());
            self.write(&ids);
        }
    }

    pub fn drop_identity(&self, identity: &str) {
        let mut ids = self
            .identities
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        ids.retain(|known| known != identity);
        self.write(&ids);
    }

    /// A private mode or a full quota: the announce itself still stands.
    fn write(&self, ids: &[String]) {
        if let Ok(raw) = serde_json::to_string(ids) {
            if let Err(err) = self.store.set(&self.key, &raw) {
                log::debug!("core-store: announced rows not saved: {err}");
            }
        }
    }
}

fn read(store: &dyn KeyValueStore, key: &str) -> Vec<String> {
    let Ok(Some(raw)) = store.get(key) else {
        return vec![];
    };
    match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(serde_json::Value::Array(items)) => items
            .into_iter()
            .filter_map(|i| i.as_str().map(str::to_string))
            .collect(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::InMemoryKeyValueStore;

    #[test]
    fn records_per_connection_without_duplicates_and_forgets() {
        let kv = Arc::new(InMemoryKeyValueStore::new());
        let rows = AnnouncedRows::open(kv.clone(), "local");
        rows.add("ns@stable");
        rows.add("ns@stable");
        rows.add("ns@beta");
        assert_eq!(rows.list(), vec!["ns@stable", "ns@beta"]);
        rows.drop_identity("ns@stable");
        assert_eq!(
            AnnouncedRows::open(kv.clone(), "local").list(),
            vec!["ns@beta"]
        );
        assert!(AnnouncedRows::open(kv, "remote-1").list().is_empty());
    }

    #[test]
    fn a_corrupt_record_reads_as_empty_and_non_strings_are_dropped() {
        let kv = Arc::new(InMemoryKeyValueStore::new());
        kv.set("quiver:self-announced:local", "{nope").unwrap();
        assert!(AnnouncedRows::open(kv.clone(), "local").list().is_empty());
        kv.set("quiver:self-announced:local", r#"["a", 3, null, "b"]"#)
            .unwrap();
        assert_eq!(
            AnnouncedRows::open(kv.clone(), "local").list(),
            vec!["a", "b"]
        );
        kv.set("quiver:self-announced:local", r#"{"a":1}"#).unwrap();
        assert!(AnnouncedRows::open(kv, "local").list().is_empty());
    }
}
