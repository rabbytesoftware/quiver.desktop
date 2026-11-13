//! What the arrow host needs from the daemon connection. The app implements
//! this on top of `quiver-core-client`'s transport (bearer token added there).

use async_trait::async_trait;

use crate::message::{ArrowRequest, ArrowResponse};
use crate::Result;

#[async_trait]
pub trait ArrowUpstream: Send + Sync {
    /// Forwards a request whose `uri` is already the daemon path
    /// (`/v0/ui/<ns>/...`) and returns the daemon's whole response.
    async fn request(
        &self,
        req: ArrowRequest,
    ) -> Result<ArrowResponse>;

    /// Opens a text WebSocket to a daemon path built by [`crate::ws_target`].
    async fn open_ws(
        &self,
        path: &str,
    ) -> Result<Box<dyn WsRelay>>;
}

/// One open socket. Only text frames cross (see docs/arrow-apps.md).
#[async_trait]
pub trait WsRelay: Send {
    async fn send(
        &mut self,
        text: String,
    ) -> Result<()>;
    /// `None` once the socket is closed.
    async fn recv(&mut self) -> Option<String>;
    async fn close(&mut self);
}
