//! Connection configuration, the status vocabulary and the traits the host
//! application implements to observe the connection layer.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::transport::Transport;

/// Whether quiver.core is up, coming up, or gone. Serialised as snake_case,
/// which is what the `core://status` event carried.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoreStatus {
    Starting,
    Ready,
    Disconnected,
}

/// Which kind of connection this is. Serialised as the lowercase strings
/// (`"local"`, `"remote"`) the `kind` field always held, so saved connections
/// and the frontend wire format are unchanged.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionKind {
    Local,
    Remote,
}

/// Only the local sidecar connection may register quiver.desktop in the Core
/// catalog. A remote core is somebody else's machine: announcing there would
/// file this desktop's build in a catalog it does not own.
pub fn should_announce_self(kind: ConnectionKind) -> bool {
    kind == ConnectionKind::Local
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub kind: ConnectionKind,
    pub url: Option<String>,
    pub api_version: String,
}

/// The connection list and which entry is active: the payload of the
/// connection-changed event and the answer to "what connections exist?".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionsState {
    pub connections: Vec<ConnectionConfig>,
    pub active_id: String,
}

/// Receives the connection layer's events. The application implements this
/// (the Tauri build emitted `core://status` and `connection://changed`).
pub trait EventSink: Send + Sync + 'static {
    fn emit_core_status(&self, status: CoreStatus);
    fn emit_connection_changed(&self, state: &ConnectionsState);
}

/// One live connection to a quiver.core.
#[async_trait]
// async_trait expands to a must_use return type, which clippy flags against any #[must_use] on the trait.
#[allow(clippy::double_must_use)]
pub trait QuiverConnection: Send + Sync {
    /// Bring the connection up, emitting `Starting` then `Ready` or
    /// `Disconnected` on `events`, and return the status it ended on.
    async fn start(&self, events: &dyn EventSink) -> CoreStatus;
    async fn teardown(&self);
    fn transport(&self) -> Arc<dyn Transport>;
    fn config(&self) -> &ConnectionConfig;
    fn set_name(&mut self, name: String);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_status_serializes_to_snake_case() {
        let json = serde_json::to_string(&CoreStatus::Disconnected).unwrap();
        assert_eq!(json, r#""disconnected""#);
    }

    #[test]
    fn kind_keeps_its_wire_spelling() {
        assert_eq!(
            serde_json::to_string(&ConnectionKind::Local).unwrap(),
            r#""local""#
        );
        let cfg: ConnectionConfig = serde_json::from_str(
            r#"{"id":"a","name":"A","kind":"remote","url":"http://x","api_version":"v0"}"#,
        )
        .unwrap();
        assert_eq!(cfg.kind, ConnectionKind::Remote);
    }

    #[test]
    fn only_the_local_connection_announces_itself() {
        assert!(should_announce_self(ConnectionKind::Local));
        assert!(!should_announce_self(ConnectionKind::Remote));
    }
}
