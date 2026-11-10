use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::domain::{ConnectionConfig, ConnectionStatus};
use crate::store::search_pass::PlatformSource;
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionsSnapshot {
    pub connections: Vec<ConnectionConfig>,
    pub active_id: String,
}

/// What state needs from the native shell: the connection list and its events, the core status
/// events, this machine's platform key, and the connection commands. The app implements it over
/// `quiver-core-client` and `quiver-platform`.
#[async_trait]
pub trait Backend: Send + Sync {
    async fn get_connections(&self) -> Result<ConnectionsSnapshot>;

    /// Core status events (`starting` / `ready` / `disconnected`), from now on. Dropping the
    /// receiver unsubscribes.
    fn on_core_status(&self) -> UnboundedReceiver<ConnectionStatus>;

    /// `connection://changed`, from now on.
    fn on_connections_changed(&self) -> UnboundedReceiver<ConnectionsSnapshot>;

    /// This machine's real `"os/arch"` platform key -- e.g. `"darwin/arm64"` -- the way quiver.core's
    /// manifest `targets` are keyed. Native-side because a UA guess can misdetect an architecture.
    async fn get_platform(&self) -> Result<String>;

    async fn check_remote_health(
        &self,
        url: &str,
    ) -> Result<()>;
    async fn add_connection(
        &self,
        name: &str,
        url: &str,
        code: &str,
    ) -> Result<ConnectionConfig>;
    async fn remove_connection(
        &self,
        id: &str,
    ) -> Result<()>;
    async fn switch_connection(
        &self,
        id: &str,
    ) -> Result<()>;
    async fn rename_connection(
        &self,
        id: &str,
        name: &str,
    ) -> Result<()>;
}

/// The search controller's platform source, answered by the backend.
pub struct BackendPlatform(pub std::sync::Arc<dyn Backend>);

#[async_trait]
impl PlatformSource for BackendPlatform {
    async fn platform(&self) -> String {
        match self.0.get_platform().await {
            Ok(platform) => platform,
            Err(err) => {
                log::error!("could not read the platform: {err}");
                String::new()
            }
        }
    }
}
