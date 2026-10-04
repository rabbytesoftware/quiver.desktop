//! The one abstraction every connection kind implements.
//!
//! Deliberately request-shaped rather than verb-shaped. The trait it replaces
//! had `get_bytes`/`post_json`/`delete`, which cannot express an arbitrary
//! request — and `post_json` returned `Result<(), _>`, throwing the daemon's
//! response body away. The `quiver://` proxy forwards whatever the webview
//! sends, so anything narrower than a whole request/response pair loses
//! information the caller asked for.

use std::time::Duration;

use async_trait::async_trait;
use tauri::http::{Request, Response};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
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
	pub chunks: mpsc::Receiver<Result<Vec<u8>, TransportError>>,
}

#[async_trait]
// async_trait expands to a must_use return type, which clippy flags against any #[must_use] on the trait.
#[allow(clippy::double_must_use)]
pub trait Transport: Send + Sync {
	/// Forward one request and return the daemon's whole response.
	async fn request(&self, req: Request<Vec<u8>>)
		-> Result<Response<Vec<u8>>, TransportError>;

	/// Dial `path` (a full `/v0/...` route) and complete the WebSocket upgrade.
	async fn open_ws(&self, path: &str) -> Result<WsStream, TransportError>;

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
	) -> Result<StreamResponse, TransportError> {
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

/// Fixture parts shared by the transports' own tests and by the `quiver://`
/// proxy's, which drives a real `HttpTransport` at a real socket rather than a
/// stand-in. Lives here rather than in either module so the two canned-response
/// listeners cannot drift apart.
#[cfg(test)]
pub(crate) mod testing {
	use tokio::io::AsyncReadExt;
	use tokio::net::TcpStream;

	/// Read the whole request the client sent, before answering it.
	///
	/// A fixture that answers WITHOUT reading leaves the request sitting unread
	/// in its receive queue, and closing a socket that still holds unread
	/// inbound data is an ABORTIVE close: the stack sends RST where it would
	/// otherwise send FIN. Windows applies that strictly — the client's next
	/// read fails with WSAECONNRESET (os error 10054) and its receive buffer is
	/// discarded, response bytes already delivered included — so the client
	/// loses the very answer the fixture just wrote. Linux and macOS leave data
	/// the client has already buffered alone, which is the only reason a
	/// write-and-close fixture ever passed anywhere. Draining first is also
	/// exactly what a real HTTP server does, so the fixture is more honest for
	/// it.
	///
	/// Bounded by the REQUEST, never by a clock: the head ends at the first
	/// blank line, and the body is exactly the `Content-Length` the client
	/// announced — none, for a GET. A peer that hangs up mid-request reads 0 and
	/// ends the drain, so this can never park waiting for bytes that will never
	/// come.
	pub(crate) async fn drain_request(sock: &mut TcpStream) {
		let mut req = Vec::new();
		let mut chunk = [0u8; 1024];
		loop {
			if let Some(head) = req.windows(4).position(|w| w == b"\r\n\r\n") {
				let head = head + 4;
				if req.len() - head >= announced_body_len(&req[..head]) {
					return;
				}
			}
			match sock.read(&mut chunk).await {
				// EOF, or a peer that reset: there is nothing further to
				// read, and waiting for more would be waiting forever.
				Ok(0) | Err(_) => return,
				Ok(n) => req.extend_from_slice(&chunk[..n]),
			}
		}
	}

	/// How many body bytes the request's head says follow it — zero when it says
	/// nothing, which is what every GET says.
	fn announced_body_len(head: &[u8]) -> usize {
		String::from_utf8_lossy(head)
			.lines()
			.filter_map(|line| line.split_once(':'))
			.find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
			.and_then(|(_, value)| value.trim().parse().ok())
			.unwrap_or(0)
	}
}

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

#[cfg(test)]
mod default_stream_tests {
	use super::*;

	/// A transport that can only answer whole: it implements `request` and `open_ws`
	/// and relies on the trait's default `request_stream`.
	struct Whole(Result<(u16, Vec<u8>), &'static str>);

	#[async_trait]
	impl Transport for Whole {
		async fn request(
			&self,
			_req: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			match &self.0 {
				Ok((status, body)) => Ok(Response::builder()
					.status(*status)
					.body(body.clone())
					.unwrap()),
				Err(why) => Err(TransportError::Connect((*why).into())),
			}
		}

		async fn open_ws(&self, _path: &str) -> Result<WsStream, TransportError> {
			Err(TransportError::Connect("no ws".into()))
		}
	}

	fn get() -> Request<Vec<u8>> {
		Request::builder()
			.uri("quiver://localhost/v0/x")
			.body(Vec::new())
			.unwrap()
	}

	#[tokio::test]
	async fn the_default_delivers_the_whole_body_as_one_chunk_then_ends() {
		let t = Whole(Ok((200, b"hello".to_vec())));
		let mut resp = t
			.request_stream(get(), Duration::from_secs(1))
			.await
			.unwrap();
		assert_eq!(resp.status, 200);
		assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b"hello");
		assert!(resp.chunks.recv().await.is_none());
	}

	#[tokio::test]
	async fn the_default_keeps_the_status_of_an_error_answer() {
		let t = Whole(Ok((403, b"{}".to_vec())));
		let resp = t
			.request_stream(get(), Duration::from_secs(1))
			.await
			.unwrap();
		assert_eq!(resp.status, 403);
	}

	#[tokio::test]
	async fn the_default_delivers_nothing_for_an_empty_body() {
		let t = Whole(Ok((204, Vec::new())));
		let mut resp = t
			.request_stream(get(), Duration::from_secs(1))
			.await
			.unwrap();
		assert_eq!(resp.status, 204);
		assert!(resp.chunks.recv().await.is_none());
	}

	#[tokio::test]
	async fn the_default_passes_a_transport_failure_through() {
		let t = Whole(Err("no socket"));
		let err = t
			.request_stream(get(), Duration::from_secs(1))
			.await
			.err()
			.unwrap();
		assert!(matches!(err, TransportError::Connect(_)), "got {err:?}");
	}
}
