//! A connection to a quiver.core somebody else runs, over https with a bearer token.

mod pairing;
mod probe;
mod version;

use std::sync::Arc;

use async_trait::async_trait;

use crate::transport::http::HttpTransport;
use crate::transport::Transport;
use crate::types::{ConnectionConfig, ConnectionKind, CoreStatus, EventSink, QuiverConnection};
use crate::Result;

pub use pairing::redeem_pairing_code;
pub use probe::{check_remote_health, probe_health};
use version::negotiate_version;

pub struct RemoteConnection {
    config: ConnectionConfig,
    transport: Arc<dyn Transport>,
}

impl RemoteConnection {
    pub async fn new(
        id: String,
        name: String,
        url: String,
        token: String,
    ) -> Result<Self> {
        let transport: Arc<dyn Transport> = Arc::new(HttpTransport::new(url.clone(), Some(token)));
        let api_version = negotiate_version(transport.as_ref()).await;
        let config = ConnectionConfig {
            id,
            name,
            kind: ConnectionKind::Remote,
            url: Some(url),
            api_version,
        };
        Ok(Self { config, transport })
    }
}

#[async_trait]
impl QuiverConnection for RemoteConnection {
    async fn start(
        &self,
        events: &dyn EventSink,
    ) -> CoreStatus {
        log::info!(
            "[remote] starting — url: {:?}, api: {}",
            self.config.url,
            self.config.api_version
        );
        events.emit_core_status(CoreStatus::Starting);

        match probe_health(self.transport.as_ref()).await {
            Ok(()) => {
                log::info!("[remote] reachable — emitting core://status: ready");
                events.emit_core_status(CoreStatus::Ready);
                CoreStatus::Ready
            }
            Err(e) => {
                log::error!("[remote] {e}");
                events.emit_core_status(CoreStatus::Disconnected);
                CoreStatus::Disconnected
            }
        }
    }

    /// Nothing to tear down, and that is a statement about this connection rather
    /// than an omission.
    ///
    /// A remote connection owns no process — the daemon belongs to whoever runs
    /// the host — and no socket of its own: the WebSocket legs belong to
    /// `WsBridgeManager`, which `retire_streams_and_teardown` closes just before
    /// calling this, and the `reqwest` client inside `HttpTransport` releases its
    /// pool when the last `Arc` to it drops, which is when this connection is
    /// replaced. `LocalConnection::teardown` has a body because a spawned child
    /// process is the one thing here that outlives its handle.
    async fn teardown(&self) {}

    fn transport(&self) -> Arc<dyn Transport> {
        Arc::clone(&self.transport)
    }

    fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    fn set_name(
        &mut self,
        name: String,
    ) {
        self.config.name = name;
    }
}

#[cfg(test)]
mod tests;
