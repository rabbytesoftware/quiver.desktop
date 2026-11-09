use crate::domain::ConnectionStatus;
use crate::observable::Store;

/// The daemon connection status; starts as `Starting`.
#[derive(Clone)]
pub struct StatusStore {
    store: Store<ConnectionStatus>,
}

impl Default for StatusStore {
    fn default() -> Self {
        Self {
            store: Store::new(ConnectionStatus::Starting),
        }
    }
}

impl StatusStore {
    pub fn status(&self) -> ConnectionStatus {
        self.store.snapshot()
    }

    pub fn set_status(&self, status: ConnectionStatus) {
        self.store.set(status);
    }

    pub fn store(&self) -> &Store<ConnectionStatus> {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_in_starting_and_follows_set_status() {
        let s = StatusStore::default();
        assert_eq!(s.status(), ConnectionStatus::Starting);
        s.set_status(ConnectionStatus::Ready);
        assert_eq!(s.status(), ConnectionStatus::Ready);
        s.set_status(ConnectionStatus::Disconnected);
        assert_eq!(s.status(), ConnectionStatus::Disconnected);
    }
}
