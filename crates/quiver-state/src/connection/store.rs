use crate::domain::{ConnectionConfig, LOCAL_CONNECTION_ID};
use crate::observable::Store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionState {
    pub connections: Vec<ConnectionConfig>,
    pub active_id: String,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self { connections: vec![], active_id: LOCAL_CONNECTION_ID.into() }
    }
}

#[derive(Clone, Default)]
pub struct ConnectionStore {
    store: Store<ConnectionState>,
}

impl ConnectionStore {
    pub fn store(&self) -> &Store<ConnectionState> {
        &self.store
    }

    pub fn snapshot(&self) -> ConnectionState {
        self.store.snapshot()
    }

    pub fn set_from_event(&self, connections: Vec<ConnectionConfig>, active_id: &str) {
        self.store.set(ConnectionState { connections, active_id: active_id.to_string() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{local_connection, ConnectionKind};

    fn remote() -> ConnectionConfig {
        ConnectionConfig {
            id: "conn-1".into(),
            name: "Remote".into(),
            kind: ConnectionKind::Remote,
            url: Some("http://localhost:7070".into()),
            api_version: "v0".into(),
        }
    }

    #[test]
    fn has_the_right_initial_state() {
        let s = ConnectionStore::default().snapshot();
        assert!(s.connections.is_empty());
        assert_eq!(s.active_id, "local");
    }

    #[test]
    fn set_from_event_updates_connections_and_active_id() {
        let store = ConnectionStore::default();
        store.set_from_event(vec![remote()], "conn-1");
        let s = store.snapshot();
        assert_eq!((s.connections, s.active_id.as_str()), (vec![remote()], "conn-1"));
    }

    #[test]
    fn set_from_event_with_no_connections_clears_the_list() {
        let store = ConnectionStore::default();
        store.set_from_event(vec![local_connection()], "old");
        store.set_from_event(vec![], "local");
        let s = store.snapshot();
        assert!(s.connections.is_empty());
        assert_eq!(s.active_id, "local");
    }
}
