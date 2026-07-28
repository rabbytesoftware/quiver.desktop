//! The `quiver://` URI scheme: every HTTP call the webview makes.
//!
//! Requests arrive as `quiver://localhost/v0/...` and are forwarded to whatever
//! connection is active — resolved PER REQUEST, so switching connections takes
//! effect on the next call with nothing to re-register.
//!
//! Registered with `.register_asynchronous_uri_scheme_protocol("quiver", ...)`.

use std::time::Duration;

use tauri::http::{Request, Response};

use crate::connection::transport::Transport;

/// Marks a response this proxy generated rather than relayed. `apiFetch`
/// retries idempotent reads on a marked 502/504 and never on an unmarked one:
/// a daemon's own 404 is meaningful and a 500 is a real server error, so
/// replaying either would be wrong.
pub const PROXY_ERROR_HEADER: &str = "x-quiver-proxy";

/// Upper bound on one proxied request.
///
/// Each request costs this process a socket. A daemon that accepts a connection
/// but never answers would otherwise pin that descriptor — and the tasks driving
/// it — for the life of the app: nothing caps requests in flight, and the
/// frontend's `fetch` has no AbortController, so it never gives up either.
/// Dropping the timed-out future drops the request sender, which is what tells
/// hyper's connection task to close and hand the descriptor back.
#[cfg(not(test))]
const PROXY_TIMEOUT: Duration = Duration::from_secs(300);

/// Same constant, shortened under test. `tokio`'s `test-util` feature (needed
/// to pause the clock and jump straight to a timer) isn't enabled in this
/// crate and Task 5 does not get to add it — Cargo.toml is off-limits here, a
/// later task deletes `hyperlocal` from it instead. A real but tiny wait is
/// the remaining option that neither touches Cargo.toml nor costs the suite
/// an actual five minutes.
#[cfg(test)]
const PROXY_TIMEOUT: Duration = Duration::from_millis(20);

fn error_response(status: u16, msg: &str) -> Response<Vec<u8>> {
	Response::builder()
		.status(status)
		.header(tauri::http::header::CONTENT_TYPE, "text/plain")
		.header(PROXY_ERROR_HEADER, "error")
		.body(msg.as_bytes().to_vec())
		.expect("a static error response always builds")
}

/// The whole proxy decision, free of Tauri's responder so it can be driven
/// against a real socket. Everything worth asserting on lives here.
pub async fn proxy_once(transport: &dyn Transport, req: Request<Vec<u8>>) -> Response<Vec<u8>> {
	match tokio::time::timeout(PROXY_TIMEOUT, transport.request(req)).await {
		Ok(Ok(resp)) => resp,
		Ok(Err(e)) => error_response(502, &format!("quiver proxy: {e}")),
		Err(_) => error_response(504, "quiver.core did not answer in time"),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::transport::http::HttpTransport;
	use crate::connection::transport::{TransportError, WsStream};
	use tokio::io::AsyncWriteExt;
	use tokio::net::TcpListener;

	fn get(path: &str) -> Request<Vec<u8>> {
		Request::builder()
			.method("GET")
			.uri(format!("quiver://localhost{path}"))
			.body(Vec::new())
			.unwrap()
	}

	async fn dead() -> HttpTransport {
		let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = l.local_addr().unwrap();
		drop(l);
		HttpTransport::new(format!("http://{addr}"), None)
	}

	async fn serving(raw: &'static str) -> HttpTransport {
		let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = l.local_addr().unwrap();
		tokio::spawn(async move {
			while let Ok((mut s, _)) = l.accept().await {
				let _ = s.write_all(raw.as_bytes()).await;
				let _ = s.shutdown().await;
			}
		});
		HttpTransport::new(format!("http://{addr}"), None)
	}

	#[tokio::test]
	async fn relays_a_successful_response_untouched() {
		let _serialised = crate::FD_TESTS.lock().await;

		let t = serving("HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}")
			.await;
		let resp = proxy_once(&t, get("/v0/health")).await;
		assert_eq!(resp.status(), 200);
		assert_eq!(resp.body(), br#"{"status":"ok"}"#);
	}

	/// A status the DAEMON produced must arrive unmarked. The marker is what
	/// tells apiFetch a failure is ours and therefore retryable; marking a real
	/// daemon 404 would make the frontend retry something that will never change.
	#[tokio::test]
	async fn a_daemon_status_is_relayed_without_the_proxy_marker() {
		let _serialised = crate::FD_TESTS.lock().await;

		let t = serving("HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\n\r\n{}").await;
		let resp = proxy_once(&t, get("/v0/arrow/nope")).await;
		assert_eq!(resp.status(), 404);
		assert!(
			resp.headers().get(PROXY_ERROR_HEADER).is_none(),
			"a daemon's own status must never carry the proxy marker"
		);
	}

	/// A dead socket becomes a 502 that IS marked, because the request never
	/// reached the daemon and retrying is the right response.
	#[tokio::test]
	async fn a_connect_failure_becomes_a_marked_502() {
		let _serialised = crate::FD_TESTS.lock().await;

		let resp = proxy_once(&dead().await, get("/v0/health")).await;
		assert_eq!(resp.status(), 502);
		assert_eq!(
			resp.headers().get(PROXY_ERROR_HEADER).map(|v| v.as_bytes()),
			Some(&b"error"[..])
		);
	}

	#[tokio::test]
	async fn the_502_body_names_the_cause() {
		let _serialised = crate::FD_TESTS.lock().await;

		let resp = proxy_once(&dead().await, get("/v0/health")).await;
		let body = String::from_utf8(resp.body().clone()).unwrap();
		assert!(
			body.contains("connect failed"),
			"a 502 must say why; got {body:?}"
		);
	}

	/// A transport whose `request()` never resolves — a daemon that accepts a
	/// connection and then goes silent. The only way `proxy_once` can move on
	/// is `PROXY_TIMEOUT` firing.
	struct NeverResponds;

	#[async_trait::async_trait]
	impl Transport for NeverResponds {
		async fn request(
			&self,
			_req: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			std::future::pending().await
		}

		async fn open_ws(&self, _path: &str) -> Result<WsStream, TransportError> {
			std::future::pending().await
		}
	}

	/// A stalled daemon must not hang the request forever: `PROXY_TIMEOUT`
	/// turns it into a marked 504, distinct from the marked 502 a connect
	/// failure produces above. `apiFetch`'s retry needs both codes to tell
	/// "never connected" apart from "connected but never answered" — even
	/// though both carry the marker. `PROXY_TIMEOUT` is milliseconds under
	/// `cfg(test)` (see its definition), so this really waits it out rather
	/// than mocking the clock.
	#[tokio::test]
	async fn a_stalled_daemon_times_out_as_a_marked_504() {
		let resp = proxy_once(&NeverResponds, get("/v0/health")).await;
		assert_eq!(resp.status(), 504);
		assert_eq!(
			resp.headers().get(PROXY_ERROR_HEADER).map(|v| v.as_bytes()),
			Some(&b"error"[..])
		);
	}
}
