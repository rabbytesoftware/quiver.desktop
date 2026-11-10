//! The one abstraction every connection kind implements.
//!
//! Deliberately request-shaped rather than verb-shaped. The trait it replaces
//! had `get_bytes`/`post_json`/`delete`, which cannot express an arbitrary
//! request — and `post_json` returned `Result<(), _>`, throwing the daemon's
//! response body away. The `quiver://` proxy forwards whatever the webview
//! sends, so anything narrower than a whole request/response pair loses
//! information the caller asked for.

use std::time::Duration;

use ::http::{Request, Response};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::Error;

/// The object-safe stream every transport's WebSocket half resolves to. Boxing
/// is what lets a unix socket and a TCP socket share one return type, so
/// the bridge never learns which connection kind it is driving.
pub trait AsyncReadWrite: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncReadWrite for T {}

/// The WebSocket every transport hands back.
///
/// The `MaybeTlsStream` layer is not decoration: it is what
/// `tokio_tungstenite::client_async_tls_with_config` returns, and that function
/// is the only public door in this crate's dependency set that performs a real
/// TLS handshake for `wss://` over a socket the CALLER opened — which both
/// transports need, because a unix socket is not a `TcpStream` and so
/// `connect_async` (which opens its own) cannot serve them both.
///
/// The `Plain` variant costs the unix and `ws://` paths nothing but a match
/// arm: `uri_mode` picks it from the scheme, and no TLS code runs.
pub type WsStream = WebSocketStream<MaybeTlsStream<Box<dyn AsyncReadWrite>>>;

/// How many body chunks may sit unread between a transport's pump task and its
/// consumer. Small on purpose: a slow consumer should push back on the daemon's
/// connection, not buffer without bound.
pub(crate) const STREAM_BUFFER: usize = 16;

/// A response whose body is still arriving.
///
/// Dropping `chunks` abandons the request: the pump task notices the closed
/// channel, stops reading and drops the connection, which the daemon sees as
/// a cancelled request context.
pub struct StreamResponse {
    pub status: u16,
    pub chunks: mpsc::Receiver<Result<Vec<u8>, Error>>,
}

#[async_trait]
// async_trait expands to a must_use return type, which clippy flags against any #[must_use] on the trait.
#[allow(clippy::double_must_use)]
pub trait Transport: Send + Sync {
    /// Forward one request and return the daemon's whole response.
    async fn request(
        &self,
        req: Request<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, Error>;

    /// Dial `path` (a full `/v0/...` route) and complete the WebSocket upgrade.
    async fn open_ws(
        &self,
        path: &str,
    ) -> Result<WsStream, Error>;

    /// Forward one request and hand back the response body as it arrives, for
    /// endpoints that stream (`POST /v0/console/exec`). `deadline` bounds the
    /// whole exchange.
    ///
    /// The default delivers the whole response as one chunk, which is correct
    /// for any transport that cannot stream and keeps stand-in transports
    /// working unchanged; the real transports override it.
    async fn request_stream(
        &self,
        req: Request<Vec<u8>>,
        _deadline: Duration,
    ) -> Result<StreamResponse, Error> {
        Ok(whole_as_stream(self.request(req).await?))
    }
}

/// A whole response as a stream: its body as one chunk (none for an empty body),
/// then the end. A free function, not part of the trait's default method, so it is
/// compiled once however many transports lean on that default.
fn whole_as_stream(resp: Response<Vec<u8>>) -> StreamResponse {
    let (parts, body) = resp.into_parts();
    let (tx, rx) = mpsc::channel(1);
    if !body.is_empty() {
        // The channel is empty and has room for one, so this cannot fail.
        let _ = tx.try_send(Ok(body));
    }
    StreamResponse {
        status: parts.status.as_u16(),
        chunks: rx,
    }
}

pub mod http;
#[cfg(windows)]
pub mod pipe;
mod stream;
#[cfg(unix)]
pub mod unix;

#[cfg(test)]
pub(crate) mod testing;

#[cfg(test)]
mod tests;
