use std::sync::Arc;

use tokio::task::JoinHandle;

use super::{Backend, ConnectionStore};

/// Dropping it stops listening.
pub struct ConnectionListeners {
    task: JoinHandle<()>,
}

impl Drop for ConnectionListeners {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Subscribes to connection changes, then seeds the store from the current list so it does not
/// wait for an event. A failed seed is logged and leaves the subscription in place.
pub async fn setup_connection_listeners(backend: Arc<dyn Backend>, store: ConnectionStore) -> ConnectionListeners {
    let mut changes = backend.on_connections_changed();
    let forward = store.clone();
    let task = tokio::spawn(async move {
        while let Some(snapshot) = changes.recv().await {
            forward.set_from_event(snapshot.connections, &snapshot.active_id);
        }
    });
    match backend.get_connections().await {
        Ok(snapshot) => store.set_from_event(snapshot.connections, &snapshot.active_id),
        Err(err) => log::error!("connection: failed to read the initial connection list: {err}"),
    }
    ConnectionListeners { task }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{ConnectionsSnapshot, FakeBackend};
    use crate::domain::{local_connection, ConnectionConfig, ConnectionKind};

    fn remote() -> ConnectionConfig {
        ConnectionConfig {
            id: "c1".into(),
            name: "Remote".into(),
            kind: ConnectionKind::Remote,
            url: Some("http://localhost:7070".into()),
            api_version: "v0".into(),
        }
    }

    #[tokio::test]
    async fn forwards_connection_changes_to_the_store() {
        let backend = Arc::new(FakeBackend::new());
        let store = ConnectionStore::default();
        let mut rx = store.store().subscribe();
        let _l = setup_connection_listeners(backend.clone(), store.clone()).await;
        rx.borrow_and_update();
        backend.emit_connections(ConnectionsSnapshot { connections: vec![remote()], active_id: "c1".into() });
        rx.changed().await.unwrap();
        let s = store.snapshot();
        assert_eq!((s.connections, s.active_id.as_str()), (vec![remote()], "c1"));
    }

    #[tokio::test]
    async fn seeds_the_store_from_the_current_list_without_waiting_for_an_event() {
        let backend = Arc::new(FakeBackend::new());
        backend.set_connections(ConnectionsSnapshot { connections: vec![local_connection()], active_id: "local".into() });
        let store = ConnectionStore::default();
        let _l = setup_connection_listeners(backend, store.clone()).await;
        assert_eq!(store.snapshot().connections, vec![local_connection()]);
    }

    #[tokio::test]
    async fn survives_a_failed_seed_with_the_subscription_still_registered() {
        let backend = Arc::new(FakeBackend::new());
        backend.fail_get_connections();
        let store = ConnectionStore::default();
        let mut rx = store.store().subscribe();
        let _l = setup_connection_listeners(backend.clone(), store.clone()).await;
        assert!(store.snapshot().connections.is_empty());
        backend.emit_connections(ConnectionsSnapshot { connections: vec![remote()], active_id: "c1".into() });
        rx.changed().await.unwrap();
        assert_eq!(store.snapshot().active_id, "c1");
    }
}
