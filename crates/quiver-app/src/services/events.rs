//! The connection layer's [`EventSink`] fanned out to any number of subscribers over unbounded
//! channels. The Tauri app emitted `core://status` and `connection://changed`; `quiver-state`'s
//! `Backend::on_core_status` / `on_connections_changed` hand out the receivers.

use std::sync::{Mutex, PoisonError};

use quiver_core_client::{ConnectionsState, CoreStatus, EventSink};
use quiver_state::connection::ConnectionsSnapshot;
use quiver_state::domain::{ConnectionConfig, ConnectionKind, ConnectionStatus};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

struct Subscribers<T>(Mutex<Vec<UnboundedSender<T>>>);

impl<T: Clone> Subscribers<T> {
    fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    fn subscribe(&self) -> UnboundedReceiver<T> {
        let (tx, rx) = unbounded_channel();
        self.lock().push(tx);
        rx
    }

    /// Delivers to every live subscriber; a dropped receiver unsubscribes.
    fn publish(&self, value: &T) {
        self.lock().retain(|tx| tx.send(value.clone()).is_ok());
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<UnboundedSender<T>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub struct ChannelEvents {
    status: Subscribers<ConnectionStatus>,
    connections: Subscribers<ConnectionsSnapshot>,
}

impl Default for ChannelEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelEvents {
    pub fn new() -> Self {
        Self {
            status: Subscribers::new(),
            connections: Subscribers::new(),
        }
    }

    /// Core status events from now on.
    pub fn subscribe_status(&self) -> UnboundedReceiver<ConnectionStatus> {
        self.status.subscribe()
    }

    /// Connection-changed events from now on.
    pub fn subscribe_connections(&self) -> UnboundedReceiver<ConnectionsSnapshot> {
        self.connections.subscribe()
    }
}

pub(super) fn status_of(status: CoreStatus) -> ConnectionStatus {
    match status {
        CoreStatus::Starting => ConnectionStatus::Starting,
        CoreStatus::Ready => ConnectionStatus::Ready,
        CoreStatus::Disconnected => ConnectionStatus::Disconnected,
    }
}

pub(super) fn config_of(config: quiver_core_client::ConnectionConfig) -> ConnectionConfig {
    ConnectionConfig {
        id: config.id,
        name: config.name,
        kind: match config.kind {
            quiver_core_client::ConnectionKind::Local => ConnectionKind::Local,
            quiver_core_client::ConnectionKind::Remote => ConnectionKind::Remote,
        },
        url: config.url,
        api_version: config.api_version,
    }
}

pub(super) fn snapshot_of(state: &ConnectionsState) -> ConnectionsSnapshot {
    ConnectionsSnapshot {
        connections: state.connections.iter().cloned().map(config_of).collect(),
        active_id: state.active_id.clone(),
    }
}

impl EventSink for ChannelEvents {
    fn emit_core_status(&self, status: CoreStatus) {
        self.status.publish(&status_of(status));
    }

    fn emit_connection_changed(&self, state: &ConnectionsState) {
        self.connections.publish(&snapshot_of(state));
    }
}

#[cfg(test)]
mod tests {
    use quiver_core_client::ConnectionConfig as CoreConfig;
    use quiver_core_client::ConnectionKind as CoreKind;

    use super::*;

    #[test]
    fn every_subscriber_gets_every_status_in_order() {
        let events = ChannelEvents::new();
        let mut a = events.subscribe_status();
        let mut b = events.subscribe_status();
        events.emit_core_status(CoreStatus::Starting);
        events.emit_core_status(CoreStatus::Ready);
        for rx in [&mut a, &mut b] {
            assert_eq!(rx.try_recv().unwrap(), ConnectionStatus::Starting);
            assert_eq!(rx.try_recv().unwrap(), ConnectionStatus::Ready);
            assert!(rx.try_recv().is_err());
        }
    }

    #[test]
    fn a_dropped_receiver_unsubscribes_without_disturbing_the_others() {
        let events = ChannelEvents::new();
        let gone = events.subscribe_status();
        let mut kept = events.subscribe_status();
        drop(gone);
        events.emit_core_status(CoreStatus::Disconnected);
        assert_eq!(kept.try_recv().unwrap(), ConnectionStatus::Disconnected);
        assert_eq!(events.status.lock().len(), 1);
    }

    #[test]
    fn connection_changes_carry_the_list_and_the_active_id() {
        let events = ChannelEvents::new();
        let mut rx = events.subscribe_connections();
        events.emit_connection_changed(&ConnectionsState {
            connections: vec![CoreConfig {
                id: "c1".into(),
                name: "Remote".into(),
                kind: CoreKind::Remote,
                url: Some("http://h:7070".into()),
                api_version: "v0".into(),
            }],
            active_id: "c1".into(),
        });
        let snap = rx.try_recv().unwrap();
        assert_eq!(snap.active_id, "c1");
        assert_eq!(snap.connections[0].kind, ConnectionKind::Remote);
        assert_eq!(snap.connections[0].url.as_deref(), Some("http://h:7070"));
    }
}
