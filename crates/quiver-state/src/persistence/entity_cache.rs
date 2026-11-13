use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use super::key_value::KeyValueStore;
use super::schemas::ArrowCatalogRecord;

/// Where the snapshot lives in the key-value store.
pub const SNAPSHOT_KEY: &str = "quiver:entity-cache:arrows";

type Key = (String, String);

/// The catalog cache: rows per connection, keyed by `(connection_id, namespace)`. Held in memory;
/// after every write a snapshot goes to the [`KeyValueStore`] on a best-effort basis (a failed
/// write costs persistence, never correctness), and the snapshot seeds the cache on creation.
pub struct EntityCache {
    rows: Mutex<BTreeMap<Key, ArrowCatalogRecord>>,
    persistence: Option<Arc<dyn KeyValueStore>>,
}

impl EntityCache {
    pub fn in_memory() -> Self {
        Self {
            rows: Mutex::default(),
            persistence: None,
        }
    }

    /// Loads whatever snapshot is readable; an unreadable or corrupt one reads as empty.
    pub fn with_persistence(store: Arc<dyn KeyValueStore>) -> Self {
        let rows = store
            .get(SNAPSHOT_KEY)
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<Vec<ArrowCatalogRecord>>(&raw).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|r| ((r.connection_id.clone(), r.namespace.clone()), r))
            .collect();
        Self {
            rows: Mutex::new(rows),
            persistence: Some(store),
        }
    }

    fn mutate(
        &self,
        f: impl FnOnce(&mut BTreeMap<Key, ArrowCatalogRecord>),
    ) {
        let mut rows = self.rows.lock().unwrap_or_else(PoisonError::into_inner);
        f(&mut rows);
        if let Some(store) = &self.persistence {
            let snapshot: Vec<&ArrowCatalogRecord> = rows.values().collect();
            if let Ok(raw) = serde_json::to_string(&snapshot) {
                if let Err(err) = store.set(SNAPSHOT_KEY, &raw) {
                    log::debug!("entity-cache: snapshot not saved: {err}");
                }
            }
        }
    }

    pub fn upsert(
        &self,
        record: ArrowCatalogRecord,
    ) {
        self.mutate(|rows| {
            rows.insert(
                (record.connection_id.clone(), record.namespace.clone()),
                record,
            );
        });
    }

    pub fn arrows_for(
        &self,
        connection_id: &str,
    ) -> Vec<ArrowCatalogRecord> {
        let rows = self.rows.lock().unwrap_or_else(PoisonError::into_inner);
        rows.values()
            .filter(|r| r.connection_id == connection_id)
            .cloned()
            .collect()
    }

    pub fn arrow(
        &self,
        connection_id: &str,
        namespace: &str,
    ) -> Option<ArrowCatalogRecord> {
        let rows = self.rows.lock().unwrap_or_else(PoisonError::into_inner);
        rows.get(&(connection_id.to_string(), namespace.to_string()))
            .cloned()
    }

    pub fn remove(
        &self,
        connection_id: &str,
        namespace: &str,
    ) {
        self.mutate(|rows| {
            rows.remove(&(connection_id.to_string(), namespace.to_string()));
        });
    }

    /// Drops every row of every connection.
    pub fn wipe(&self) {
        self.mutate(BTreeMap::clear);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::persistence::InMemoryKeyValueStore;

    pub(crate) fn rec(
        connection_id: &str,
        namespace: &str,
    ) -> ArrowCatalogRecord {
        ArrowCatalogRecord {
            connection_id: connection_id.into(),
            namespace: namespace.into(),
            name: namespace.into(),
            description: String::new(),
            tags: vec![],
            icon: None,
            banner: None,
            version: "1".into(),
            last_used_at: None,
            origin: None,
            confidence: None,
        }
    }

    struct Broken;

    impl KeyValueStore for Broken {
        fn get(
            &self,
            _: &str,
        ) -> Result<Option<String>, String> {
            Err("blocked".into())
        }

        fn set(
            &self,
            _: &str,
            _: &str,
        ) -> Result<(), String> {
            Err("QuotaExceeded".into())
        }
    }

    #[test]
    fn round_trips_a_record() {
        let cache = EntityCache::in_memory();
        cache.upsert(rec("local", "a@1"));
        assert_eq!(cache.arrows_for("local").len(), 1);
    }

    #[test]
    fn keeps_connections_isolated() {
        let cache = EntityCache::in_memory();
        cache.upsert(rec("local", "a@1"));
        cache.upsert(rec("remote-1", "a@1"));
        assert_eq!(cache.arrows_for("local")[0].namespace, "a@1");
        assert_eq!(cache.arrows_for("remote-1").len(), 1);
    }

    #[test]
    fn removes_only_the_addressed_row() {
        let cache = EntityCache::in_memory();
        cache.upsert(rec("local", "a@1"));
        cache.upsert(rec("remote-1", "a@1"));
        cache.remove("local", "a@1");
        assert!(cache.arrows_for("local").is_empty());
        assert_eq!(cache.arrows_for("remote-1").len(), 1);
    }

    #[test]
    fn reads_one_record_by_connection_and_identity() {
        let cache = EntityCache::in_memory();
        cache.upsert(rec("local", "a@stable"));
        assert_eq!(
            cache.arrow("local", "a@stable").map(|r| r.version),
            Some("1".into())
        );
        assert!(cache.arrow("remote-1", "a@stable").is_none());
    }

    #[test]
    fn a_failing_snapshot_store_degrades_to_memory_only() {
        let cache = EntityCache::with_persistence(Arc::new(Broken));
        assert!(cache.arrows_for("local").is_empty());
        cache.upsert(rec("local", "a@1"));
        assert_eq!(cache.arrows_for("local").len(), 1);
        cache.remove("local", "a@1");
        cache.wipe();
    }

    #[test]
    fn the_snapshot_survives_a_restart() {
        let kv = Arc::new(InMemoryKeyValueStore::new());
        EntityCache::with_persistence(kv.clone()).upsert(rec("local", "a@1"));
        assert_eq!(
            EntityCache::with_persistence(kv.clone())
                .arrows_for("local")
                .len(),
            1
        );
        kv.set(SNAPSHOT_KEY, "{corrupt").unwrap();
        assert!(EntityCache::with_persistence(kv)
            .arrows_for("local")
            .is_empty());
    }
}
