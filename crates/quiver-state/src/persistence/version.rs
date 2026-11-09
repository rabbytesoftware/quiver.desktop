use super::entity_cache::EntityCache;
use super::key_value::KeyValueStore;

/// Bump to throw away every cached row on the next start.
pub const QUIVER_CACHE_VERSION: &str = "1";
pub const CACHE_VERSION_KEY: &str = "quiver:cache-version";

/// Wipes the entity cache when the stored cache version differs from this build's. An unreadable
/// store counts as a mismatch; a store that cannot be written still gets the wipe.
pub fn maybe_wipe_on_version_change(cache: &EntityCache, store: &dyn KeyValueStore) {
    let stored = store.get(CACHE_VERSION_KEY).unwrap_or(None);
    if stored.as_deref() == Some(QUIVER_CACHE_VERSION) {
        return;
    }
    cache.wipe();
    if let Err(err) = store.set(CACHE_VERSION_KEY, QUIVER_CACHE_VERSION) {
        log::debug!("entity-cache: cache version not recorded (the wipe still ran): {err}");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::persistence::entity_cache::tests::rec;
    use crate::persistence::InMemoryKeyValueStore;

    struct Unreadable;

    impl KeyValueStore for Unreadable {
        fn get(&self, _: &str) -> Result<Option<String>, String> {
            Err("SecurityError".into())
        }

        fn set(&self, _: &str, _: &str) -> Result<(), String> {
            Err("QuotaExceededError".into())
        }
    }

    fn seeded() -> EntityCache {
        let cache = EntityCache::in_memory();
        cache.upsert(rec("local", "a@1"));
        cache
    }

    #[test]
    fn wipes_on_a_version_mismatch_and_records_the_new_version() {
        let kv = Arc::new(InMemoryKeyValueStore::new());
        kv.set(CACHE_VERSION_KEY, "ancient").unwrap();
        let cache = seeded();
        maybe_wipe_on_version_change(&cache, kv.as_ref());
        assert!(cache.arrows_for("local").is_empty());
        assert_eq!(
            kv.get(CACHE_VERSION_KEY).unwrap().as_deref(),
            Some(QUIVER_CACHE_VERSION)
        );
    }

    #[test]
    fn wipes_a_store_that_never_recorded_a_version() {
        let cache = seeded();
        maybe_wipe_on_version_change(&cache, &InMemoryKeyValueStore::new());
        assert!(cache.arrows_for("local").is_empty());
    }

    #[test]
    fn is_a_noop_when_the_version_already_matches() {
        let kv = InMemoryKeyValueStore::new();
        kv.set(CACHE_VERSION_KEY, QUIVER_CACHE_VERSION).unwrap();
        let cache = seeded();
        maybe_wipe_on_version_change(&cache, &kv);
        assert_eq!(cache.arrows_for("local").len(), 1);
    }

    #[test]
    fn an_unreadable_and_unwritable_store_still_wipes() {
        let cache = seeded();
        maybe_wipe_on_version_change(&cache, &Unreadable);
        assert!(cache.arrows_for("local").is_empty());
    }
}
