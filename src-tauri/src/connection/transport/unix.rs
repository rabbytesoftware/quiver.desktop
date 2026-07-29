//! The local transport on macOS and Linux: quiver.core over its unix socket.
//!
//! Windows has no equivalent — Rust's async stack cannot reach AF_UNIX there
//! (see the design doc §2.2) — so this module is cfg-gated and Windows uses
//! `super::http` against a loopback port instead.

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request as HyperRequest;
use hyper_util::rt::TokioIo;
use tauri::http::{self, Request, Response};
use tokio::net::UnixStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::{AsyncReadWrite, Transport, TransportError, WsStream};

pub struct UnixTransport {
	socket_path: String,
}

impl UnixTransport {
	pub fn new(socket_path: impl Into<String>) -> Self {
		Self {
			socket_path: socket_path.into(),
		}
	}
}

#[async_trait]
impl Transport for UnixTransport {
	async fn request(
		&self,
		req: Request<Vec<u8>>,
	) -> Result<Response<Vec<u8>>, TransportError> {
		// hyper only needs the path-and-query for the request line; the
		// authority in `quiver://localhost/...` is meaningless over a socket.
		let path_and_query = req
			.uri()
			.path_and_query()
			.map(|pq| pq.as_str().to_string())
			.unwrap_or_else(|| req.uri().path().to_string());

		let stream = UnixStream::connect(&self.socket_path)
			.await
			.map_err(|e| TransportError::Connect(e.to_string()))?;
		let (mut sender, conn) =
			hyper::client::conn::http1::handshake(TokioIo::new(stream))
				.await
				.map_err(|e| TransportError::Protocol(e.to_string()))?;
		tokio::spawn(async move {
			let _ = conn.await;
		});

		let (parts, body) = req.into_parts();
		let mut builder = HyperRequest::builder()
			.method(parts.method)
			.uri(path_and_query);

		if let Some(headers) = builder.headers_mut() {
			for (name, value) in parts.headers.iter() {
				headers.insert(name, value.clone());
			}
			// HTTP/1.1 requires a Host; a unix socket has no meaningful one.
			if !headers.contains_key(http::header::HOST) {
				headers.insert(
					http::header::HOST,
					http::HeaderValue::from_static("localhost"),
				);
			}
		}

		let upstream = builder
			.body(Full::<Bytes>::new(body.into()))
			.map_err(|e| TransportError::Protocol(e.to_string()))?;

		let resp = sender
			.send_request(upstream)
			.await
			.map_err(|e| TransportError::Protocol(e.to_string()))?;
		let (rp, rb) = resp.into_parts();
		let collected = rb
			.collect()
			.await
			.map_err(|e| TransportError::Protocol(e.to_string()))?
			.to_bytes()
			.to_vec();

		let mut out = Response::builder().status(rp.status);
		if let Some(headers) = out.headers_mut() {
			for (name, value) in rp.headers.iter() {
				headers.insert(name, value.clone());
			}
		}
		out.body(collected)
			.map_err(|e| TransportError::Protocol(e.to_string()))
	}

	async fn open_ws(&self, path: &str) -> Result<WsStream, TransportError> {
		let stream = UnixStream::connect(&self.socket_path)
			.await
			.map_err(|e| TransportError::Connect(e.to_string()))?;
		let request = format!("ws://localhost{path}")
			.into_client_request()
			.map_err(|e| TransportError::Protocol(e.to_string()))?;
		let boxed: Box<dyn AsyncReadWrite> = Box::new(stream);
		// The same door `http.rs` uses, so both transports resolve to one
		// `WsStream` type. The request is always `ws://` here — a unix socket
		// carries no TLS — so `uri_mode` picks `Mode::Plain` and this is
		// `client_async` with an extra enum wrapper: no handshake, no
		// certificate work, nothing on the wire that was not there before.
		let (ws, _) =
			tokio_tungstenite::client_async_tls_with_config(request, boxed, None, None)
				.await
				.map_err(|e| TransportError::Protocol(e.to_string()))?;
		Ok(ws)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::io::{AsyncReadExt, AsyncWriteExt};
	use tokio::net::UnixListener;

	/// Short path: sun_path is capped at 104 bytes and macOS $TMPDIR is long.
	fn test_socket(tag: &str) -> std::path::PathBuf {
		let p = std::path::PathBuf::from(format!(
			"/tmp/qvx-{}-{tag}.sock",
			std::process::id()
		));
		let _ = std::fs::remove_file(&p);
		p
	}

	/// Binds the listener; `accept_and_reply` does the writing.
	fn serve(sock: &std::path::Path) -> UnixListener {
		UnixListener::bind(sock).expect("bind")
	}

	/// Answers every connection with one canned HTTP/1.1 response.
	async fn accept_and_reply(listener: &UnixListener, raw: &'static str) {
		if let Ok((mut s, _)) = listener.accept().await {
			let _ = s.write_all(raw.as_bytes()).await;
			let _ = s.shutdown().await;
		}
	}

	fn get(path: &str) -> Request<Vec<u8>> {
		Request::builder()
			.method("GET")
			.uri(format!("quiver://localhost{path}"))
			.body(Vec::new())
			.unwrap()
	}

	#[tokio::test]
	async fn forwards_a_request_and_returns_the_whole_response() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("ok");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let (resp, _) = tokio::join!(
			t.request(get("/v0/health")),
			accept_and_reply(
				&listener,
				"HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}"
			)
		);

		let resp = resp.expect("request must succeed");
		assert_eq!(resp.status(), 200);
		assert_eq!(resp.body(), br#"{"status":"ok"}"#);
		let _ = std::fs::remove_file(&sock);
	}

	/// A 4xx/5xx is the daemon ANSWERING. It must come back as a Response, not
	/// an Err — the proxy relays it verbatim so the frontend can tell a 404
	/// (meaningful) from a dead socket (retryable).
	#[tokio::test]
	async fn a_daemon_error_status_is_a_response_not_an_error() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("404");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let (resp, _) = tokio::join!(
			t.request(get("/v0/arrow/nope")),
			accept_and_reply(
				&listener,
				"HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\n\r\n{}"
			)
		);

		assert_eq!(resp.expect("must not be an Err").status(), 404);
		let _ = std::fs::remove_file(&sock);
	}

	#[tokio::test]
	async fn a_missing_socket_is_a_connect_error() {
		let _serialised = crate::FD_TESTS.lock().await;

		let t = UnixTransport::new("/tmp/qvx-definitely-not-here.sock");
		let err = t.request(get("/v0/health")).await.unwrap_err();
		assert!(matches!(err, TransportError::Connect(_)), "got {err:?}");
	}

	#[tokio::test]
	async fn open_ws_on_a_missing_socket_is_a_connect_error() {
		let _serialised = crate::FD_TESTS.lock().await;

		let t = UnixTransport::new("/tmp/qvx-definitely-not-here.sock");
		// `.unwrap_err()` needs the `Ok` type to be `Debug`, and `WsStream`
		// isn't. `.err().unwrap()` drops the `Ok` value instead of formatting
		// it, sidestepping that bound.
		let err = t.open_ws("/v0/arrow").await.err().unwrap();
		assert!(matches!(err, TransportError::Connect(_)), "got {err:?}");
	}

	#[tokio::test]
	async fn open_ws_completes_the_upgrade_against_a_real_peer() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("ws");
		let listener = UnixListener::bind(&sock).unwrap();
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let server = async {
			let (s, _) = listener.accept().await.unwrap();
			let _ = tokio_tungstenite::accept_async(s).await;
		};
		let (ws, _) = tokio::join!(t.open_ws("/v0/arrow"), server);
		assert!(ws.is_ok(), "upgrade must succeed");
		let _ = std::fs::remove_file(&sock);
	}

	// --- Coverage for branches the 5 tests above the brief specifies don't
	// reach: the request-target fallback (which, in turn, surfaces the
	// request-builder's own `map_err`), and every other reachable failure
	// point downstream of a successful connect. `handshake`'s `map_err` and
	// the response-builder's `map_err` are not exercised below because they
	// have no black-box trigger — HTTP/1 handshake does no I/O, and the
	// response builder only ever receives an already-valid `StatusCode`, so
	// those two arms are unreachable short of injecting a broken stream.

	/// A request target with no `path_and_query` (e.g. a CONNECT-style
	/// authority-form URI) exercises the `unwrap_or_else` fallback to
	/// `req.uri().path()` — which, for that URI form, is always `""`. That
	/// value never reaches the peer: building the outgoing request with an
	/// empty target fails immediately, before `send_request` is even called.
	/// Standing up a real listener and observing zero bytes read (rather
	/// than trusting `Err` alone) is what ties this assertion to the
	/// fallback's actual output — an unrelated failure (e.g. a bad
	/// `socket_path`) wouldn't get this far to write anything either, but it
	/// also wouldn't leave a listener sitting there having accepted a
	/// connection that then received nothing.
	#[tokio::test]
	async fn a_uri_without_a_path_and_query_fails_to_build_a_request() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("fallback");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let req = Request::builder()
			.method("GET")
			.uri("localhost:1234") // authority-form: no path_and_query
			.body(Vec::new())
			.unwrap();

		let capture = async {
			let (mut s, _) = listener.accept().await.expect("accept");
			let mut buf = [0u8; 16];
			s.read(&mut buf).await.unwrap_or(0)
		};

		let (resp, bytes_received) = tokio::join!(t.request(req), capture);

		assert_eq!(
			bytes_received, 0,
			"the empty fallback target must never reach the wire"
		);
		let err = resp.unwrap_err();
		assert!(
			matches!(&err, TransportError::Protocol(msg) if msg.contains("empty")),
			"got {err:?}"
		);
		let _ = std::fs::remove_file(&sock);
	}

	/// A response cut short of its promised `Content-Length` is a protocol
	/// error, not a panic or a silently truncated body.
	#[tokio::test]
	async fn a_truncated_body_is_a_protocol_error() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("trunc");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let (resp, _) = tokio::join!(
			t.request(get("/v0/health")),
			accept_and_reply(
				&listener,
				"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\nshort"
			)
		);

		let err = resp.unwrap_err();
		assert!(matches!(err, TransportError::Protocol(_)), "got {err:?}");
		let _ = std::fs::remove_file(&sock);
	}

	/// A path with a character that is invalid in a URI (a raw space) fails
	/// to build a WebSocket client request rather than being silently
	/// mangled into a different route.
	#[tokio::test]
	async fn an_invalid_ws_path_is_a_protocol_error() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("wsbadpath");
		let _listener = UnixListener::bind(&sock).unwrap();
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let err = t.open_ws("/v0/ar row").await.err().unwrap();
		assert!(matches!(err, TransportError::Protocol(_)), "got {err:?}");
		let _ = std::fs::remove_file(&sock);
	}

	/// A peer that accepts and then closes the connection before completing
	/// the WebSocket handshake fails the upgrade with a protocol error.
	#[tokio::test]
	async fn a_peer_that_closes_before_the_upgrade_is_a_protocol_error() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("wsclose");
		let listener = UnixListener::bind(&sock).unwrap();
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let server = async {
			let (s, _) = listener.accept().await.unwrap();
			drop(s);
		};
		let (ws, _) = tokio::join!(t.open_ws("/v0/arrow"), server);
		let err = ws.err().unwrap();
		assert!(matches!(err, TransportError::Protocol(_)), "got {err:?}");
		let _ = std::fs::remove_file(&sock);
	}

	/// A peer that accepts and then closes the connection before reading the
	/// request fails to send it with a protocol error, rather than hanging.
	#[tokio::test]
	async fn a_peer_that_closes_before_reading_is_a_protocol_error() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("closefast");
		let listener = UnixListener::bind(&sock).unwrap();
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let server = async {
			let (s, _) = listener.accept().await.unwrap();
			drop(s);
		};
		let (resp, _) = tokio::join!(t.request(get("/v0/health")), server);
		let err = resp.unwrap_err();
		assert!(matches!(err, TransportError::Protocol(_)), "got {err:?}");
		let _ = std::fs::remove_file(&sock);
	}
}
