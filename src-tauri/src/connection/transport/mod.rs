//! The one abstraction every connection kind implements.
//!
//! Deliberately request-shaped rather than verb-shaped. The trait it replaces
//! had `get_bytes`/`post_json`/`delete`, which cannot express an arbitrary
//! request — and `post_json` returned `Result<(), _>`, throwing the daemon's
//! response body away. The `quiver://` proxy forwards whatever the webview
//! sends, so anything narrower than a whole request/response pair loses
//! information the caller asked for.

use async_trait::async_trait;
use tauri::http::{Request, Response};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

#[derive(Debug, Error)]
pub enum TransportError {
	#[error("connect failed: {0}")]
	Connect(String),
	#[error("protocol error: {0}")]
	Protocol(String),
}

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

#[async_trait]
pub trait Transport: Send + Sync {
	/// Forward one request and return the daemon's whole response.
	async fn request(&self, req: Request<Vec<u8>>)
		-> Result<Response<Vec<u8>>, TransportError>;

	/// Dial `path` (a full `/v0/...` route) and complete the WebSocket upgrade.
	async fn open_ws(&self, path: &str) -> Result<WsStream, TransportError>;
}

pub mod http;
#[cfg(unix)]
pub mod unix;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn connect_errors_name_the_cause() {
		let e = TransportError::Connect("no such file".into());
		assert_eq!(e.to_string(), "connect failed: no such file");
	}

	#[test]
	fn protocol_errors_name_the_cause() {
		let e = TransportError::Protocol("bad status line".into());
		assert_eq!(e.to_string(), "protocol error: bad status line");
	}
}
