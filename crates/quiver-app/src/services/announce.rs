//! The two halves of the desktop's self-announce, wired so that exactly ONE of them announces.
//!
//! * `quiver-core-client`'s `LocalSelfAnnounce` is the announcer. It is handed to the connection
//!   manager (`ManagerDeps::self_announce`), which calls it for the LOCAL connection only.
//! * `quiver-state`'s listeners also have an announce hook (the TypeScript `announceSelf` call
//!   sites). It is a no-op here: a second announcer would be a second way for a remote daemon to
//!   get this desktop registered in its catalog, which is the bug being fixed.

use std::sync::Arc;

use async_trait::async_trait;
use quiver_core_client::AnnouncedRows;
use quiver_state::listeners::SelfAnnounceHook;
use quiver_state::persistence::KeyValueStore;

/// `quiver-state`'s per-connection record of announced rows, as the announce flow wants it.
pub(super) struct StoredRows(quiver_state::listeners::AnnouncedRows);

impl StoredRows {
    /// The record for the local connection, kept in `store`.
    pub(super) fn local(store: Arc<dyn KeyValueStore>) -> Self {
        Self(quiver_state::listeners::AnnouncedRows::open(
            store,
            quiver_state::domain::LOCAL_CONNECTION_ID,
        ))
    }
}

impl AnnouncedRows for StoredRows {
    fn list(&self) -> Vec<String> {
        self.0.list()
    }

    fn add(&self, identity: &str) {
        self.0.add(identity);
    }

    fn drop_identity(&self, identity: &str) {
        self.0.drop_identity(identity);
    }
}

pub(super) struct NoAnnounce;

#[async_trait]
impl SelfAnnounceHook for NoAnnounce {
    async fn announce_self(&self) {}
}

#[cfg(test)]
mod tests {
    use quiver_state::persistence::InMemoryKeyValueStore;

    use super::*;

    #[test]
    fn rows_are_kept_in_the_store_under_the_local_connection() {
        let kv: Arc<dyn KeyValueStore> = Arc::new(InMemoryKeyValueStore::new());
        let rows = StoredRows::local(kv.clone());
        rows.add("ns@stable");
        rows.add("ns@beta");
        rows.drop_identity("ns@beta");
        assert_eq!(StoredRows::local(kv).list(), vec!["ns@stable"]);
    }

    #[tokio::test]
    async fn the_state_hook_announces_nothing() {
        NoAnnounce.announce_self().await;
    }
}
