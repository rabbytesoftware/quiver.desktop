//! The TCP transport, serving two callers that differ only in credentials:
//!
//!   * a remote quiver.core over https with a bearer token, and
//!   * the LOCAL daemon on Windows, over loopback with no token at all —
//!     Rust cannot reach AF_UNIX on Windows (design doc §2.2), so the sidecar
//!     is spawned with `--host tcp://127.0.0.1:<port>` there.
//!
//! Note that quiver.core's TCP mode is unauthenticated: a loopback listener is
//! reachable by any local process. That is an accepted risk of the Windows
//! path, recorded in the design doc, not something this module can fix.

use async_trait::async_trait;
use tauri::http::{self, Request, Response};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::{AsyncReadWrite, Transport, TransportError, WsStream};

pub struct HttpTransport {
	client: reqwest::Client,
	base_url: String,
	token: Option<String>,
}

impl HttpTransport {
	pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
		// `tcp://` is quiver.core's own spelling for "plain HTTP here".
		let base_url = base_url.into().replace("tcp://", "http://");
		Self {
			client: reqwest::Client::new(),
			base_url,
			token,
		}
	}

	fn url(&self, path: &str) -> String {
		format!("{}{}", self.base_url, path)
	}

	fn ws_url(&self, path: &str) -> String {
		let base = self
			.base_url
			.replacen("https://", "wss://", 1)
			.replacen("http://", "ws://", 1);
		format!("{base}{path}")
	}

	fn auth_header(&self) -> Option<String> {
		self.token.as_ref().map(|t| format!("Bearer {t}"))
	}
}

#[async_trait]
impl Transport for HttpTransport {
	async fn request(
		&self,
		req: Request<Vec<u8>>,
	) -> Result<Response<Vec<u8>>, TransportError> {
		let path_and_query = req
			.uri()
			.path_and_query()
			.map(|pq| pq.as_str().to_string())
			.unwrap_or_else(|| req.uri().path().to_string());

		let (parts, body) = req.into_parts();
		let mut out = self
			.client
			.request(parts.method, self.url(&path_and_query))
			.body(body);

		for (name, value) in parts.headers.iter() {
			// The inbound Host (e.g. from the `quiver://localhost` scheme
			// callers dial through) names the *local* proxy, not the remote
			// peer this transport actually talks to — forwarding it verbatim
			// would address the request to the wrong virtual host on any
			// vhosted/proxied deployment, and reqwest will not override an
			// explicitly-set Host. Let reqwest derive the correct one from
			// the target URL instead.
			if *name == http::header::HOST {
				continue;
			}
			out = out.header(name, value);
		}
		if let Some(auth) = self.auth_header() {
			out = out.header(reqwest::header::AUTHORIZATION, auth);
		}

		let resp = out.send().await.map_err(|e| {
			// `send()` spans both the connect phase and everything after a
			// connection is established (e.g. a peer that accepts then hangs
			// up before responding). Only the former is actually a Connect
			// failure — collapsing both into Connect would make this
			// transport disagree with unix.rs, whose equivalent split
			// (`UnixStream::connect` vs. `send_request`) already keeps them
			// distinct.
			if e.is_connect() {
				TransportError::Connect(e.to_string())
			} else {
				TransportError::Protocol(e.to_string())
			}
		})?;
		let status = resp.status();
		let headers = resp.headers().clone();
		let bytes = resp
			.bytes()
			.await
			.map_err(|e| TransportError::Protocol(e.to_string()))?;

		let mut builder = Response::builder().status(status);
		if let Some(h) = builder.headers_mut() {
			for (name, value) in headers.iter() {
				h.insert(name, value.clone());
			}
		}
		builder.body(bytes.to_vec())
			.map_err(|e| TransportError::Protocol(e.to_string()))
	}

	async fn open_ws(&self, path: &str) -> Result<WsStream, TransportError> {
		// TODO(wss): this dials a raw TcpStream and hands it to client_async,
		// which performs no TLS handshake, so wss:// does not actually work
		// yet. No current UI reaches a remote WebSocket. Fixing this needs a
		// tokio_tungstenite::connect_async path built on the native-tls
		// feature before remote WS ships — see Task 18's report.
		let url = self.ws_url(path);
		let mut request = url
			.into_client_request()
			.map_err(|e| TransportError::Protocol(e.to_string()))?;
		if let Some(auth) = self.auth_header() {
			request.headers_mut().insert(
				tokio_tungstenite::tungstenite::http::header::AUTHORIZATION,
				auth.parse().map_err(|_| {
					TransportError::Protocol("bad token".into())
				})?,
			);
		}

		// `into_client_request` above already rejects any URI lacking an
		// authority (tungstenite's `IntoClientRequest for Uri` returns
		// `Error::Url(UrlError::NoHostName)` in that case), so by the time
		// we get here `authority()` is always `Some`. The `ok_or_else` below
		// has no reachable failure path through this public API.
		let host = request
			.uri()
			.authority()
			.map(|a| a.to_string())
			.ok_or_else(|| {
				TransportError::Protocol("ws url has no authority".into())
			})?;
		let stream = TcpStream::connect(&host)
			.await
			.map_err(|e| TransportError::Connect(e.to_string()))?;
		let boxed: Box<dyn AsyncReadWrite> = Box::new(stream);
		let (ws, _) = tokio_tungstenite::client_async(request, boxed)
			.await
			.map_err(|e| TransportError::Protocol(e.to_string()))?;
		Ok(ws)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::io::{AsyncReadExt, AsyncWriteExt};
	use tokio::net::TcpListener;

	fn get(path: &str) -> Request<Vec<u8>> {
		Request::builder()
			.method("GET")
			.uri(format!("quiver://localhost{path}"))
			.body(Vec::new())
			.unwrap()
	}

	/// A transport pointed at a port nothing listens on, so every request fails
	/// at connect. Binding then dropping is what makes the port both free and
	/// known-unused. (Moved here from the deleted remote/transport.rs.)
	async fn unreachable() -> HttpTransport {
		let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
		let addr = listener.local_addr().expect("addr");
		drop(listener);
		HttpTransport::new(format!("tcp://{addr}"), Some("tok".into()))
	}

	/// Promises more body than it sends, then hangs up — the only way to reach
	/// the body-read error arm. (Moved here from the deleted remote/transport.rs.)
	async fn truncates_its_body(status: u16) -> HttpTransport {
		let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
		let addr = listener.local_addr().expect("addr");
		tokio::spawn(async move {
			while let Ok((mut sock, _)) = listener.accept().await {
				let _ = sock
					.write_all(
						format!("HTTP/1.1 {status} X\r\nContent-Length: 64\r\n\r\nshort").as_bytes(),
					)
					.await;
				let _ = sock.shutdown().await;
			}
		});
		HttpTransport::new(format!("tcp://{addr}"), None)
	}

	async fn serving(raw: &'static str) -> HttpTransport {
		let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
		let addr = listener.local_addr().expect("addr");
		tokio::spawn(async move {
			while let Ok((mut sock, _)) = listener.accept().await {
				let _ = sock.write_all(raw.as_bytes()).await;
				let _ = sock.shutdown().await;
			}
		});
		HttpTransport::new(format!("http://{addr}"), None)
	}

	#[test]
	fn tcp_scheme_is_normalised_to_http() {
		let t = HttpTransport::new("tcp://10.0.0.1:40257", None);
		assert_eq!(t.url("/v0/health"), "http://10.0.0.1:40257/v0/health");
	}

	#[test]
	fn https_base_is_left_alone() {
		let t = HttpTransport::new("https://core.example.com", None);
		assert_eq!(t.url("/v0/health"), "https://core.example.com/v0/health");
	}

	#[test]
	fn ws_url_derives_wss_from_https() {
		let t = HttpTransport::new("https://core.example.com", None);
		assert_eq!(t.ws_url("/v0/arrow"), "wss://core.example.com/v0/arrow");
	}

	#[test]
	fn ws_url_derives_ws_from_loopback_http() {
		let t = HttpTransport::new("tcp://127.0.0.1:5555", None);
		assert_eq!(t.ws_url("/v0/arrow"), "ws://127.0.0.1:5555/v0/arrow");
	}

	/// Local Windows has no token. A transport built without one must not send
	/// an Authorization header at all — an empty Bearer would be worse than none.
	#[test]
	fn a_transport_without_a_token_sends_no_auth_header() {
		let t = HttpTransport::new("http://127.0.0.1:1", None);
		assert!(t.auth_header().is_none());
	}

	#[test]
	fn a_transport_with_a_token_sends_a_bearer() {
		let t = HttpTransport::new("http://127.0.0.1:1", Some("abc".into()));
		assert_eq!(t.auth_header().as_deref(), Some("Bearer abc"));
	}

	#[tokio::test]
	async fn forwards_a_request_and_returns_the_whole_response() {
		let t = serving("HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}")
			.await;
		let resp = t.request(get("/v0/health")).await.expect("must succeed");
		assert_eq!(resp.status(), 200);
		assert_eq!(resp.body(), br#"{"status":"ok"}"#);
	}

	#[tokio::test]
	async fn a_daemon_error_status_is_a_response_not_an_error() {
		let t = serving("HTTP/1.1 500 Oops\r\nContent-Length: 2\r\n\r\n{}").await;
		assert_eq!(
			t.request(get("/v0/arrow")).await.expect("not Err").status(),
			500
		);
	}

	#[tokio::test]
	async fn an_unreachable_host_is_a_connect_error() {
		let err = unreachable()
			.await
			.request(get("/v0/health"))
			.await
			.unwrap_err();
		assert!(matches!(err, TransportError::Connect(_)), "got {err:?}");
	}

	#[tokio::test]
	async fn a_truncated_body_is_a_protocol_error() {
		let err = truncates_its_body(200)
			.await
			.request(get("/v0/arrow"))
			.await
			.unwrap_err();
		assert!(matches!(err, TransportError::Protocol(_)), "got {err:?}");
	}

	/// `send()` spans both the connect phase and everything after a
	/// connection is established. A peer that accepts the TCP connection and
	/// then closes before sending a byte back must surface as `Protocol`, not
	/// `Connect` — `reqwest::Error::is_connect()` is `false` for this case,
	/// matching unix.rs's identical distinction between `UnixStream::connect`
	/// (Connect) and `send_request` (Protocol), and its equivalent test
	/// (`a_peer_that_closes_before_reading_is_a_protocol_error`).
	#[tokio::test]
	async fn a_peer_that_accepts_then_closes_before_responding_is_a_protocol_error() {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		let t = HttpTransport::new(format!("http://{addr}"), None);

		let server = async {
			let (s, _) = listener.accept().await.unwrap();
			drop(s);
		};
		let (resp, _) = tokio::join!(t.request(get("/v0/health")), server);
		let err = resp.unwrap_err();
		assert!(matches!(err, TransportError::Protocol(_)), "got {err:?}");
	}

	#[tokio::test]
	async fn open_ws_against_a_dead_port_is_a_connect_error() {
		// `.unwrap_err()` needs `Ok`'s type to be `Debug`, and `WsStream`
		// (`WebSocketStream<Box<dyn AsyncReadWrite>>`) isn't. `.err().unwrap()`
		// drops the `Ok` value instead of formatting it, sidestepping that
		// bound — same fix as unix.rs's equivalent test.
		let err = unreachable()
			.await
			.open_ws("/v0/arrow")
			.await
			.err()
			.unwrap();
		assert!(matches!(err, TransportError::Connect(_)), "got {err:?}");
	}

	#[tokio::test]
	async fn open_ws_completes_the_upgrade_against_a_real_peer() {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		let t = HttpTransport::new(format!("http://{addr}"), None);
		let server = async {
			let (s, _) = listener.accept().await.unwrap();
			let _ = tokio_tungstenite::accept_async(s).await;
		};
		let (ws, _) = tokio::join!(t.open_ws("/v0/arrow"), server);
		assert!(ws.is_ok(), "upgrade must succeed");
	}

	// --- Coverage for branches the 12 tests above (the brief's own set)
	// don't reach. Each of the following asserts something none of the tests
	// above already prove. Two branches are deliberately left untested
	// because the public API has no way to reach them: the `ok_or_else`
	// "ws url has no authority" arm in `open_ws` (see the comment at its call
	// site — `into_client_request` already rejects a missing authority before
	// we get there), and the response-builder's `headers_mut`/`body` failure
	// arms in `request()` (the status always comes from an already-valid
	// `reqwest::StatusCode`, so the builder never enters an error state).
	// These mirror the unreachable arms unix.rs documents rather than tests.

	/// An authority-form URI (no `path_and_query`) drives `request()` down the
	/// `unwrap_or_else` fallback to `req.uri().path()`, which for that URI form
	/// is always `""` — so the request targets the bare base URL. Unlike
	/// unix.rs's hyper-based transport (where an empty request target fails to
	/// build), reqwest happily sends that as a request for `/`. Capturing the
	/// actual request line is what proves the fallback ran, rather than merely
	/// that some request succeeded.
	#[tokio::test]
	async fn a_uri_without_a_path_and_query_falls_back_to_the_bare_base_url() {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		let t = HttpTransport::new(format!("http://{addr}"), None);

		let req = Request::builder()
			.method("GET")
			.uri("localhost:1234") // authority-form: no path_and_query
			.body(Vec::new())
			.unwrap();

		let capture = async {
			let (mut sock, _) = listener.accept().await.expect("accept");
			let mut buf = [0u8; 256];
			let n = sock.read(&mut buf).await.unwrap_or(0);
			let raw = String::from_utf8_lossy(&buf[..n]).to_string();
			let _ = sock
				.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
				.await;
			let _ = sock.shutdown().await;
			raw
		};

		let (resp, request_line) = tokio::join!(t.request(req), capture);
		assert!(
			resp.is_ok(),
			"fallback target must still be a valid request"
		);
		assert!(
			request_line.starts_with("GET / HTTP"),
			"expected the bare base URL (root path), got {request_line:?}"
		);
	}

	/// `request()` must forward both an arbitrary caller-set header and the
	/// Authorization bearer onto the wire — two lines (the header-copy loop,
	/// and the `auth_header` branch inside `request()`) that none of the
	/// tests above exercise, since `get()` sets no extra headers and every
	/// prior `.request()` call used a tokenless transport.
	#[tokio::test]
	async fn a_request_forwards_custom_headers_and_the_bearer_token() {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		let t = HttpTransport::new(format!("http://{addr}"), Some("s3cr3t".into()));

		let req = Request::builder()
			.method("GET")
			.uri("quiver://localhost/v0/health")
			.header("x-quiver-test", "sentinel")
			.body(Vec::new())
			.unwrap();

		let capture = async {
			let (mut sock, _) = listener.accept().await.expect("accept");
			let mut buf = [0u8; 512];
			let n = sock.read(&mut buf).await.unwrap_or(0);
			let raw = String::from_utf8_lossy(&buf[..n]).to_string();
			let _ = sock
				.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
				.await;
			let _ = sock.shutdown().await;
			raw
		};

		let (resp, raw) = tokio::join!(t.request(req), capture);
		assert!(resp.is_ok());
		let lower = raw.to_lowercase();
		assert!(lower.contains("x-quiver-test: sentinel"), "got {raw:?}");
		assert!(
			lower.contains("authorization: bearer s3cr3t"),
			"got {raw:?}"
		);
	}

	/// The inbound `Host` header (as set by callers dialing through the
	/// `quiver://localhost` URI scheme) names the local proxy, not the remote
	/// peer this transport actually talks to. Forwarding it verbatim would
	/// silently address the request to the wrong virtual host on any
	/// vhosted/proxied deployment — and reqwest will not override an
	/// explicitly-set Host — so `request()` must drop it and let reqwest
	/// derive the correct one from the target URL instead.
	#[tokio::test]
	async fn an_inbound_host_header_is_not_forwarded_to_the_remote_peer() {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		let t = HttpTransport::new(format!("http://{addr}"), None);

		let req = Request::builder()
			.method("GET")
			.uri("quiver://localhost/v0/health")
			.header("host", "localhost")
			.body(Vec::new())
			.unwrap();

		let capture = async {
			let (mut sock, _) = listener.accept().await.expect("accept");
			let mut buf = [0u8; 512];
			let n = sock.read(&mut buf).await.unwrap_or(0);
			let raw = String::from_utf8_lossy(&buf[..n]).to_string();
			let _ = sock
				.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
				.await;
			let _ = sock.shutdown().await;
			raw
		};

		let (resp, raw) = tokio::join!(t.request(req), capture);
		assert!(resp.is_ok());
		let lower = raw.to_lowercase();
		assert!(
			lower.contains(format!("host: {addr}").to_lowercase().as_str()),
			"expected Host to be the peer's own authority, got {raw:?}"
		);
		assert!(
			!lower.contains("host: localhost"),
			"inbound Host header must not be forwarded verbatim, got {raw:?}"
		);
	}

	/// A path with a raw space is not a valid URI. `into_client_request`'s own
	/// parse fails, which must surface as `Protocol`, not panic or hang.
	#[tokio::test]
	async fn an_invalid_ws_path_is_a_protocol_error() {
		let t = HttpTransport::new("http://127.0.0.1:1", None);
		let err = t.open_ws("/v0/ar row").await.err().unwrap();
		assert!(matches!(err, TransportError::Protocol(_)), "got {err:?}");
	}

	/// A token containing a byte that is invalid in a header value (here, a
	/// newline) fails `auth.parse()` inside `open_ws` — a distinct guard, on a
	/// distinct input, from the path-parse failure above.
	#[tokio::test]
	async fn open_ws_with_an_invalid_token_is_a_protocol_error() {
		let t = HttpTransport::new("http://127.0.0.1:1", Some("bad\ntoken".into()));
		let err = t.open_ws("/v0/arrow").await.err().unwrap();
		assert!(matches!(err, TransportError::Protocol(_)), "got {err:?}");
	}

	/// `open_ws` inserts its own Authorization header — a separate code path
	/// from `request()`'s — so a passing `request()` token test does not prove
	/// this one works. The peer captures the raw handshake bytes and then
	/// hangs up before answering, which both proves the header was sent and
	/// exercises the upgrade failing against a peer that never completes it.
	#[tokio::test]
	async fn open_ws_sends_a_bearer_header_before_the_peer_hangs_up() {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		let t = HttpTransport::new(format!("http://{addr}"), Some("tok123".into()));

		let capture = async {
			let (mut sock, _) = listener.accept().await.unwrap();
			let mut buf = [0u8; 512];
			let n = sock.read(&mut buf).await.unwrap_or(0);
			String::from_utf8_lossy(&buf[..n]).to_string()
		};

		let (ws, raw) = tokio::join!(t.open_ws("/v0/arrow"), capture);
		assert!(
			raw.to_lowercase().contains("authorization: bearer tok123"),
			"expected bearer header in handshake, got {raw:?}"
		);
		let err = ws.err().unwrap();
		assert!(matches!(err, TransportError::Protocol(_)), "got {err:?}");
	}
}
