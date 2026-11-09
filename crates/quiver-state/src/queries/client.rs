use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::keys::QueryKey;
use crate::observable::Store;

#[derive(Default)]
struct Inner {
    data: HashMap<QueryKey, Value>,
    /// Every prefix ever invalidated, in order.
    invalidations: Vec<QueryKey>,
}

/// What the TanStack `QueryClient` was used for: invalidating by key prefix (so open screens
/// re-read) and holding data a mutation updates optimistically. A view re-reads a key when its
/// [`QueryClient::epoch`] changes; [`QueryClient::subscribe`] wakes it.
#[derive(Clone, Default)]
pub struct QueryClient {
    inner: Arc<Mutex<Inner>>,
    revision: Store<u64>,
}

impl QueryClient {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Marks every key that starts with `prefix` stale.
    pub fn invalidate(&self, prefix: &[String]) {
        self.lock().invalidations.push(prefix.to_vec());
        self.revision.update(|r| *r += 1);
    }

    /// How many invalidations have covered `key` so far; it changes exactly when a re-read is due.
    pub fn epoch(&self, key: &[String]) -> usize {
        self.lock()
            .invalidations
            .iter()
            .filter(|p| key.starts_with(p))
            .count()
    }

    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<u64> {
        self.revision.subscribe()
    }

    pub fn set_data<T: Serialize>(&self, key: &[String], value: &T) {
        if let Ok(value) = serde_json::to_value(value) {
            self.lock().data.insert(key.to_vec(), value);
            self.revision.update(|r| *r += 1);
        }
    }

    pub fn get_data<T: DeserializeOwned>(&self, key: &[String]) -> Option<T> {
        let value = self.lock().data.get(key).cloned()?;
        serde_json::from_value(value).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queries::keys::{arrow_detail_key, arrow_detail_prefix, collection_key};

    #[test]
    fn an_invalidation_covers_every_key_under_its_prefix_only() {
        let client = QueryClient::new();
        let detail = arrow_detail_key("a@1");
        let collection = collection_key("c");
        assert_eq!(client.epoch(&detail), 0);
        client.invalidate(&arrow_detail_prefix());
        assert_eq!((client.epoch(&detail), client.epoch(&collection)), (1, 0));
        client.invalidate(&detail);
        assert_eq!(client.epoch(&detail), 2);
        assert_eq!(client.epoch(&arrow_detail_key("b@1")), 1);
    }

    #[test]
    fn holds_typed_data_by_key() {
        let client = QueryClient::new();
        let key = collection_key("c");
        assert_eq!(client.get_data::<Vec<u8>>(&key), None);
        client.set_data(&key, &vec![1u8, 2]);
        assert_eq!(client.get_data::<Vec<u8>>(&key), Some(vec![1, 2]));
    }
}
