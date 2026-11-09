use std::sync::Arc;

use async_trait::async_trait;

use crate::connection::{Backend, ConnectionsSnapshot};
use crate::domain::ConnectionKind;

/// Announces quiver.desktop to the connected daemon (register the row, adopt the running build,
/// forget what earlier announces left). Implemented by `quiver-core-client`; fire-and-forget with
/// logging inside -- a daemon too old for any of this, an unreachable manifest host, or a transient
/// error must never fail or block the connection this rides on.
#[async_trait]
pub trait SelfAnnounceHook: Send + Sync {
    async fn announce_self(&self);
}

/// Only the machine's own daemon is told about this app. A remote daemon belongs to someone else's
/// host: registering this desktop's build in its catalog would be wrong, and the build tag here
/// says nothing about what runs there. An unknown connection announces nothing.
pub fn should_announce(kind: Option<ConnectionKind>) -> bool {
    kind == Some(ConnectionKind::Local)
}

pub fn active_connection_kind(snapshot: &ConnectionsSnapshot) -> Option<ConnectionKind> {
    snapshot.connections.iter().find(|c| c.id == snapshot.active_id).map(|c| c.kind)
}

/// The announce a `ready` (or an adopted core) triggers, gated on the active connection being local.
pub(super) async fn announce_if_local(backend: Arc<dyn Backend>, hook: Arc<dyn SelfAnnounceHook>) {
    let kind = match backend.get_connections().await {
        Ok(snapshot) => active_connection_kind(&snapshot),
        Err(err) => {
            log::error!("core-store: could not name the connection; not announcing: {err}");
            None
        }
    };
    if should_announce(kind) {
        hook.announce_self().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{local_connection, ConnectionConfig};

    fn remote(id: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: id.into(),
            name: "R".into(),
            kind: ConnectionKind::Remote,
            url: Some("http://h:7070".into()),
            api_version: "v0".into(),
        }
    }

    #[test]
    fn a_local_connection_announces_and_a_remote_one_does_not() {
        assert!(should_announce(Some(ConnectionKind::Local)));
        assert!(!should_announce(Some(ConnectionKind::Remote)));
        assert!(!should_announce(None));
    }

    #[test]
    fn the_kind_comes_from_the_active_connection_not_the_first_one() {
        let snapshot = ConnectionsSnapshot { connections: vec![local_connection(), remote("r1")], active_id: "r1".into() };
        assert_eq!(active_connection_kind(&snapshot), Some(ConnectionKind::Remote));
        let local = ConnectionsSnapshot { active_id: "local".into(), ..snapshot.clone() };
        assert_eq!(active_connection_kind(&local), Some(ConnectionKind::Local));
        let unknown = ConnectionsSnapshot { active_id: "gone".into(), ..snapshot };
        assert_eq!(active_connection_kind(&unknown), None);
    }
}
