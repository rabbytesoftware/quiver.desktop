//! The local transport on macOS and Linux: quiver.core over its unix socket.
//!
//! Windows reaches the same daemon over a named pipe instead (`super::pipe`);
//! both hand their connected stream to `super::stream`, so the HTTP and
//! WebSocket handling exists once.

use async_trait::async_trait;
use tauri::http::{Request, Response};
use tokio::net::UnixStream;

use std::time::Duration;

use super::stream::{open_ws_over, request_over, request_stream_over};
use super::{StreamResponse, Transport, TransportError, WsStream};

pub struct UnixTransport {
	socket_path: String,
}

impl UnixTransport {
	pub fn new(socket_path: impl Into<String>) -> Self {
		Self {
			socket_path: socket_path.into(),
		}
	}

	async fn connect(&self) -> Result<UnixStream, TransportError> {
		UnixStream::connect(&self.socket_path)
			.await
			.map_err(|e| TransportError::Connect(e.to_string()))
	}
}

#[async_trait]
impl Transport for UnixTransport {
	async fn request(
		&self,
		req: Request<Vec<u8>>,
	) -> Result<Response<Vec<u8>>, TransportError> {
		request_over(self.connect().await?, req).await
	}

	async fn open_ws(&self, path: &str) -> Result<WsStream, TransportError> {
		open_ws_over(self.connect().await?, path).await
	}

	async fn request_stream(
		&self,
		req: Request<Vec<u8>>,
		deadline: Duration,
	) -> Result<StreamResponse, TransportError> {
		request_stream_over(self.connect().await?, req, deadline).await
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

	fn post(path: &str, body: &str) -> Request<Vec<u8>> {
		Request::builder()
			.method("POST")
			.uri(format!("quiver://localhost{path}"))
			.header("content-type", "application/json")
			.body(body.as_bytes().to_vec())
			.unwrap()
	}

	/// The point of `request_stream`: a chunk the daemon has written reaches the
	/// caller BEFORE the daemon finishes the response. A transport that collected
	/// the body first would deadlock here, since the server only finishes once
	/// the client has acknowledged the first chunk.
	#[tokio::test]
	async fn request_stream_delivers_chunks_before_the_response_ends() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("stream");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());
		let (ack_tx, ack_rx) = tokio::sync::oneshot::channel::<()>();

		let server = async {
			let (mut s, _) = listener.accept().await.unwrap();
			let mut buf = [0u8; 1024];
			let _ = s.read(&mut buf).await.unwrap();
			s.write_all(
				b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n",
			)
			.await
			.unwrap();
			ack_rx.await.unwrap();
			s.write_all(b"6\r\n world\r\n0\r\n\r\n").await.unwrap();
			let _ = s.shutdown().await;
		};
		let client = async {
			let mut resp = t
				.request_stream(
					post("/v0/console/exec", "{}"),
					Duration::from_secs(5),
				)
				.await
				.expect("stream must open");
			assert_eq!(resp.status, 200);
			assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b"hello");
			ack_tx.send(()).unwrap();
			assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b" world");
			assert!(resp.chunks.recv().await.is_none(), "stream must end");
		};
		tokio::join!(server, client);
		let _ = std::fs::remove_file(&sock);
	}

	/// Dropping the receiver abandons the request: the daemon must see the
	/// connection close, because that is what cancels the command it is running.
	#[tokio::test]
	async fn dropping_the_stream_closes_the_connection() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("stream-drop");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());
		let (first_tx, first_rx) = tokio::sync::oneshot::channel::<()>();

		let server = async {
			let (mut s, _) = listener.accept().await.unwrap();
			let mut buf = [0u8; 1024];
			let _ = s.read(&mut buf).await.unwrap();
			s.write_all(
				b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n",
			)
			.await
			.unwrap();
			first_tx.send(()).unwrap();
			// The client hanging up shows as EOF (or a reset) on read.
			let closed = tokio::time::timeout(Duration::from_secs(3), async {
				loop {
					match s.read(&mut buf).await {
						Ok(0) | Err(_) => return,
						Ok(_) => {}
					}
				}
			})
			.await;
			assert!(closed.is_ok(), "the daemon never saw the client hang up");
		};
		let client = async {
			let mut resp = t
				.request_stream(
					post("/v0/console/exec", "{}"),
					Duration::from_secs(5),
				)
				.await
				.expect("stream must open");
			assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b"hello");
			first_rx.await.unwrap();
			drop(resp);
		};
		tokio::join!(server, client);
		let _ = std::fs::remove_file(&sock);
	}

	async fn collect(mut resp: StreamResponse) -> Vec<Result<Vec<u8>, TransportError>> {
		let mut items = Vec::new();
		while let Some(item) = resp.chunks.recv().await {
			items.push(item);
		}
		items
	}

	/// The daemon hanging up mid-body is an error the consumer must hear about, after
	/// what did arrive -- not a quiet end that reads as a finished command.
	#[tokio::test]
	async fn a_body_cut_short_is_reported_after_what_arrived() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("stream-cut");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let server = async {
			let (mut s, _) = listener.accept().await.unwrap();
			let mut buf = [0u8; 1024];
			let _ = s.read(&mut buf).await.unwrap();
			s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nhello")
				.await
				.unwrap();
			let _ = s.shutdown().await;
		};
		let client = async {
			let resp = t
				.request_stream(
					post("/v0/console/exec", "{}"),
					Duration::from_secs(5),
				)
				.await
				.unwrap();
			collect(resp).await
		};
		let (_, items) = tokio::join!(server, client);
		assert_eq!(items.first().unwrap().as_ref().unwrap(), b"hello");
		assert!(
			matches!(items.last().unwrap(), Err(TransportError::Protocol(_))),
			"{items:?}"
		);
		let _ = std::fs::remove_file(&sock);
	}

	/// A command that goes quiet for longer than its deadline ends the stream with a
	/// timeout error rather than holding the connection for ever.
	#[tokio::test]
	async fn a_body_that_stalls_past_the_deadline_ends_in_a_timeout() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("stream-stall");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let server = async {
			let (mut s, _) = listener.accept().await.unwrap();
			let mut buf = [0u8; 1024];
			let _ = s.read(&mut buf).await.unwrap();
			s.write_all(
				b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n",
			)
			.await
			.unwrap();
			// Say nothing more until the client has given up.
			let _ = tokio::time::timeout(Duration::from_secs(3), s.read(&mut buf))
				.await;
		};
		let client = async {
			let resp = t
				.request_stream(
					post("/v0/console/exec", "{}"),
					Duration::from_millis(300),
				)
				.await
				.unwrap();
			collect(resp).await
		};
		let (_, items) = tokio::join!(server, client);
		assert_eq!(items[0].as_ref().unwrap(), b"hello");
		match items.last().unwrap() {
			Err(TransportError::Protocol(why)) => {
				assert!(why.contains("timed out"), "{why}")
			}
			other => panic!("expected a timeout, got {other:?}"),
		}
		let _ = std::fs::remove_file(&sock);
	}

	/// A daemon that accepts the request and never answers is cut off at the deadline too.
	#[tokio::test]
	async fn a_daemon_that_never_answers_is_a_timeout_before_the_status() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("stream-silent");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let server = async {
			let (mut s, _) = listener.accept().await.unwrap();
			let mut buf = [0u8; 1024];
			let _ = s.read(&mut buf).await.unwrap();
			let _ = tokio::time::timeout(Duration::from_secs(3), s.read(&mut buf))
				.await;
		};
		let client = async {
			t.request_stream(post("/v0/console/exec", "{}"), Duration::from_millis(200))
				.await
				.err()
				.unwrap()
		};
		let (_, err) = tokio::join!(server, client);
		match err {
			TransportError::Protocol(why) => {
				assert!(why.contains("timed out"), "{why}")
			}
			other => panic!("expected a timeout, got {other:?}"),
		}
		let _ = std::fs::remove_file(&sock);
	}

	/// Trailers carry nothing this app reads: they are skipped, and the body still ends cleanly.
	#[tokio::test]
	async fn trailers_are_skipped_and_the_body_still_ends_cleanly() {
		let _serialised = crate::FD_TESTS.lock().await;

		let sock = test_socket("stream-trailers");
		let listener = serve(&sock);
		let t = UnixTransport::new(sock.to_string_lossy().to_string());

		let server = async {
			let (mut s, _) = listener.accept().await.unwrap();
			let mut buf = [0u8; 1024];
			let _ = s.read(&mut buf).await.unwrap();
			s.write_all(
				b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: X-Done\r\n\r\n5\r\nhello\r\n0\r\nX-Done: yes\r\n\r\n",
			)
			.await
			.unwrap();
			let _ = s.shutdown().await;
		};
		let client = async {
			let resp = t
				.request_stream(
					post("/v0/console/exec", "{}"),
					Duration::from_secs(5),
				)
				.await
				.unwrap();
			collect(resp).await
		};
		let (_, items) = tokio::join!(server, client);
		let ok: Vec<_> = items.iter().filter_map(|i| i.as_ref().ok()).collect();
		assert_eq!(ok, vec![&b"hello".to_vec()]);
		assert!(items.iter().all(|i| i.is_ok()), "{items:?}");
		let _ = std::fs::remove_file(&sock);
	}

	#[tokio::test]
	async fn request_stream_on_a_missing_socket_is_a_connect_error() {
		let _serialised = crate::FD_TESTS.lock().await;

		let t = UnixTransport::new("/tmp/qvx-definitely-not-here.sock");
		let err = t
			.request_stream(get("/v0/health"), Duration::from_secs(1))
			.await
			.err()
			.unwrap();
		assert!(matches!(err, TransportError::Connect(_)), "got {err:?}");
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
