//! `quiver-state`'s [`Backend`] (what the old `invoke()` commands did) over the connection manager.

use std::sync::Arc;

use async_trait::async_trait;
use quiver_core_client::ConnectionManager;
use quiver_state::connection::{Backend, ConnectionsSnapshot};
use quiver_state::domain::{ConnectionConfig, ConnectionStatus};
use quiver_state::{Error, Result};
use tokio::sync::mpsc::UnboundedReceiver;

use super::events::{config_of, snapshot_of, ChannelEvents};

pub struct NativeBackend {
    manager: Arc<ConnectionManager>,
    events: Arc<ChannelEvents>,
    platform: String,
}

impl NativeBackend {
    /// `platform` is this machine's `"os/arch"` key (`quiver_platform::platform_key()`), taken as a
    /// parameter so a test does not depend on the machine it runs on.
    pub fn new(
        manager: Arc<ConnectionManager>,
        events: Arc<ChannelEvents>,
        platform: String,
    ) -> Self {
        Self {
            manager,
            events,
            platform,
        }
    }
}

fn backend_error(err: quiver_core_client::Error) -> Error {
    Error::Backend(err.to_string())
}

#[async_trait]
impl Backend for NativeBackend {
    async fn get_connections(&self) -> Result<ConnectionsSnapshot> {
        Ok(snapshot_of(&self.manager.get_connections().await))
    }

    fn on_core_status(&self) -> UnboundedReceiver<ConnectionStatus> {
        self.events.subscribe_status()
    }

    fn on_connections_changed(&self) -> UnboundedReceiver<ConnectionsSnapshot> {
        self.events.subscribe_connections()
    }

    async fn get_platform(&self) -> Result<String> {
        Ok(self.platform.clone())
    }

    async fn check_remote_health(
        &self,
        url: &str,
    ) -> Result<()> {
        quiver_core_client::check_remote_health(url)
            .await
            .map_err(backend_error)
    }

    async fn add_connection(
        &self,
        name: &str,
        url: &str,
        code: &str,
    ) -> Result<ConnectionConfig> {
        self.manager
            .add_connection(name.into(), url.into(), code.into())
            .await
            .map(config_of)
            .map_err(backend_error)
    }

    async fn remove_connection(
        &self,
        id: &str,
    ) -> Result<()> {
        self.manager
            .remove_connection(id)
            .await
            .map_err(backend_error)
    }

    async fn switch_connection(
        &self,
        id: &str,
    ) -> Result<()> {
        self.manager
            .switch_connection(id)
            .await
            .map_err(backend_error)
    }

    async fn rename_connection(
        &self,
        id: &str,
        name: &str,
    ) -> Result<()> {
        self.manager
            .rename_connection(id, name.into())
            .await
            .map_err(backend_error)
    }
}
