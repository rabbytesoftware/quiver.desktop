//! The connection to the quiver.core this app spawns and supervises itself.

mod config;
mod host;
mod sidecar;

use std::sync::Arc;

use async_trait::async_trait;

use crate::transport::Transport;
use crate::types::{ConnectionConfig, ConnectionKind, CoreStatus, EventSink, QuiverConnection};

pub use config::LocalConfig;
use host::transport_for;
pub use host::LocalHost;
use sidecar::SidecarManager;

/// The local connection's config as it is before anyone renames it. The
/// connection list shows this when another connection is active.
pub(crate) fn default_config() -> ConnectionConfig {
    ConnectionConfig {
        id: "local".into(),
        name: "Local".into(),
        kind: ConnectionKind::Local,
        url: None,
        api_version: "v0".into(),
    }
}

pub struct LocalConnection {
    config: ConnectionConfig,
    transport: Arc<dyn Transport>,
    sidecar: SidecarManager,
    host: LocalHost,
}

impl LocalConnection {
    pub fn new(config: LocalConfig) -> Self {
        let host = config.host.clone();
        Self {
            config: default_config(),
            transport: transport_for(&host),
            sidecar: SidecarManager::new(config),
            host,
        }
    }
}

#[async_trait]
impl QuiverConnection for LocalConnection {
    async fn start(
        &self,
        events: &dyn EventSink,
    ) -> CoreStatus {
        log::info!("[local] starting — host: {:?}", self.host);
        events.emit_core_status(CoreStatus::Starting);

        // `ensure_running`, not `spawn` + `wait_for_ready`: `new()` runs again
        // on every switch back to local, and an unconditional spawn is what
        // left Windows hosting a daemon per switch. See `LOCAL_PIPE_NAME`.
        if let Err(e) = self.sidecar.ensure_running(self.transport.as_ref()).await {
            log::error!("[local] sidecar did not become ready: {e}");
            events.emit_core_status(CoreStatus::Disconnected);
            return CoreStatus::Disconnected;
        }

        log::info!("[local] ready");
        events.emit_core_status(CoreStatus::Ready);
        CoreStatus::Ready
    }

    /// Stop the daemon this connection spawned, and only that one. See
    /// [`SidecarManager::reap`].
    async fn teardown(&self) {
        self.sidecar.reap(self.transport.as_ref()).await;
    }

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
