//! The `quiver://` URI scheme: every HTTP call the webview makes.
//!
//! Requests arrive as `quiver://localhost/v0/...` and are forwarded to whatever
//! connection is active — resolved PER REQUEST, so switching connections takes
//! effect on the next call with nothing to re-register.
//!
//! Registered with `.register_asynchronous_uri_scheme_protocol("quiver", ...)`.
//!
//! # Why this handler speaks CORS
//!
//! The page is served from the webview's own origin (`tauri://localhost`, or
//! `http://tauri.localhost` on Windows) and this scheme is a DIFFERENT origin,
//! so every `fetch()` the frontend makes here is cross-origin. Tauri adds
//! `Access-Control-Allow-Origin` for its own asset protocol and for nothing
//! else, so a handler that sets no CORS headers gets two things wrong:
//!
//!   1. `x-quiver-proxy` is not a CORS-safelisted response header, so without
//!      `Access-Control-Expose-Headers` the frontend reads `null` for it, and
//!      `isProxyFailure` in `src/lib/transport/api.ts` never returns true —
//!      the entire cold-start retry path is dead code.
//!   2. `DELETE`, and `POST` with a JSON content type, are not simple requests,
//!      so the browser sends an `OPTIONS` preflight first. Forwarded to gin,
//!      that answers whatever gin feels like; answered here, it answers what
//!      this proxy actually permits.
//!
//! Both are fixed below, on EVERY response — the marker is read on error
//! responses, so an error that omits the expose header is exactly as dead as
//! one that omits the marker.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tauri::http::{header, HeaderValue, Method, Request, Response};

use crate::connection::transport::Transport;

/// Marks a response this proxy generated rather than relayed. `apiFetch`
/// retries idempotent reads on a marked 502/504 and never on an unmarked one:
/// a daemon's own 404 is meaningful and a 500 is a real server error, so
/// replaying either would be wrong.
pub const PROXY_ERROR_HEADER: &str = "x-quiver-proxy";

/// Who may read a response from this scheme.
///
/// `*` rather than the window's origin, which is what Tauri's asset protocol
/// echoes back. Two reasons. The origin differs per platform and per dev/bundle
/// mode (`tauri://localhost`, `http://tauri.localhost`, `http://localhost:5173`)
/// and nothing hands this handler that string; and `*` costs nothing here
/// because no browser credential is ever attached — the webview sends no
/// cookies to this scheme and the bearer token is added by the transport, in
/// Rust, after the request has left the page. A wildcard would matter if a
/// hostile origin could reach this handler, and none can: the scheme is
/// registered on this app's own webview and is not routable from anywhere else.
const ALLOW_ORIGIN: HeaderValue = HeaderValue::from_static("*");

/// The methods `apiFetch` and the connection commands actually send. Listing
/// only these, rather than `*`, keeps the preflight answer an honest statement
/// of what the proxy serves.
const ALLOW_METHODS: HeaderValue = HeaderValue::from_static("GET, POST, DELETE, PATCH, OPTIONS");

/// The only header the frontend sets on a request that a preflight can reject.
/// (`Authorization` is not here on purpose: the page never sets it — the
/// transport adds the bearer server-side.)
const ALLOW_HEADERS: HeaderValue = HeaderValue::from_static("Content-Type");

/// Without this the frontend's `res.headers.get('x-quiver-proxy')` is `null` on
/// a cross-origin response, no matter what this proxy sets, because the header
/// is not on the CORS-safelist.
const EXPOSE_HEADERS: HeaderValue = HeaderValue::from_static(PROXY_ERROR_HEADER);

/// How long a preflight answer may be cached. Ten minutes: long enough that a
/// burst of DELETEs costs one preflight, short enough that changing the lists
/// above takes effect within one sitting.
const PREFLIGHT_MAX_AGE: HeaderValue = HeaderValue::from_static("600");

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

/// Same constant, shortened under test. `tokio`'s `test-util` feature — needed
/// to pause the clock and jump straight to a timer — is not enabled in this
/// crate, so a real but tiny wait is the remaining option that costs the suite
/// half a second instead of an actual five minutes.
///
/// It must stay ABOVE `http::CONNECT_TIMEOUT`, which is what ships (10s under
/// 300s) and what `the_proxy_ceiling_stays_above_the_transports_connect_ceiling`
/// pins. At the 20ms this used to be, that ordering was INVERTED under test —
/// the proxy's ceiling sat below the transport's own connect ceiling, so a
/// request to a dead port raced its connect failure against this timeout.
/// Loopback refuses in microseconds on Unix and the failure always won; the
/// Windows runner is slower than that, the timeout won, and two tests that
/// assert on the marked 502 a connect failure produces saw the marked 504 of a
/// proxy that gave up first. Above `CONNECT_TIMEOUT` there is no race left to
/// lose: reqwest bounds the connect itself and reports the expiry as a connect
/// error, so a dead port is a `TransportError::Connect` within 250ms on any
/// platform, however slowly the network stack answers.
#[cfg(test)]
const PROXY_TIMEOUT: Duration = Duration::from_millis(500);

/// Stamp the headers every response from this scheme must carry, whoever built
/// it. `insert` rather than `append`: a daemon that sets its own
/// `Access-Control-Allow-Origin` must not end up with two, which browsers
/// reject outright.
fn with_cors(mut resp: Response<Vec<u8>>) -> Response<Vec<u8>> {
	let headers = resp.headers_mut();
	headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN);
	headers.insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, EXPOSE_HEADERS);
	resp
}

fn error_response(status: u16, msg: &str) -> Response<Vec<u8>> {
	with_cors(
		Response::builder()
			.status(status)
			.header(header::CONTENT_TYPE, "text/plain")
			.header(PROXY_ERROR_HEADER, "error")
			.body(msg.as_bytes().to_vec())
			.expect("a static error response always builds"),
	)
}

/// The answer to a CORS preflight. Built here rather than forwarded: gin knows
/// nothing about the origins this app runs on, and a preflight that reaches the
/// daemon has already cost a socket and a round trip for a question only this
/// process can answer.
fn preflight_response() -> Response<Vec<u8>> {
	let mut resp = with_cors(
		Response::builder()
			.status(204)
			.body(Vec::new())
			.expect("a static preflight response always builds"),
	);
	let headers = resp.headers_mut();
	headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, ALLOW_METHODS);
	headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, ALLOW_HEADERS);
	headers.insert(header::ACCESS_CONTROL_MAX_AGE, PREFLIGHT_MAX_AGE);
	resp
}

/// The whole proxy decision, free of Tauri's responder so it can be driven
/// against a real socket. Everything worth asserting on lives here.
///
/// `transport` is a FUTURE, not a reference, and it is awaited INSIDE the
/// timeout. Resolving the active connection means taking `ConnectionManager`'s
/// read lock, and tokio's `RwLock` is fair: a reader that arrives while a
/// writer waits queues behind that writer. Awaited before the timeout, as this
/// used to be, a request that never even reaches a transport waits forever —
/// past `PROXY_TIMEOUT`, past anything. Inside it, the worst case is the same
/// marked 504 a stalled daemon produces, which is a thing the frontend already
/// knows how to retry.
pub async fn proxy_once<F>(transport: F, req: Request<Vec<u8>>) -> Response<Vec<u8>>
where
	F: Future<Output = Arc<dyn Transport>>,
{
	// Answered before the transport is even resolved: a preflight asks what
	// THIS proxy permits, and no daemon can answer that.
	if req.method() == Method::OPTIONS {
		return preflight_response();
	}

	let proxied = async move {
		let transport = transport.await;
		transport.request(req).await
	};

	match tokio::time::timeout(PROXY_TIMEOUT, proxied).await {
		Ok(Ok(resp)) => with_cors(resp),
		Ok(Err(e)) => error_response(502, &format!("quiver proxy: {e}")),
		Err(_) => error_response(504, "quiver.core did not answer in time"),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::transport::http::{HttpTransport, CONNECT_TIMEOUT};
	use crate::connection::transport::testing::drain_request;
	use crate::connection::transport::{TransportError, WsStream};
	use std::future::ready;
	use tokio::io::AsyncWriteExt;
	use tokio::net::TcpListener;

	fn get(path: &str) -> Request<Vec<u8>> {
		req("GET", path)
	}

	fn req(method: &str, path: &str) -> Request<Vec<u8>> {
		Request::builder()
			.method(method)
			.uri(format!("quiver://localhost{path}"))
			.body(Vec::new())
			.unwrap()
	}

	/// The shape `handle_request` passes in: an already-resolved transport,
	/// wrapped in the future `proxy_once` awaits inside its timeout.
	fn resolved(t: impl Transport + 'static) -> impl Future<Output = Arc<dyn Transport>> {
		ready(Arc::new(t) as Arc<dyn Transport>)
	}

	async fn dead() -> HttpTransport {
		let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = l.local_addr().unwrap();
		drop(l);
		HttpTransport::new(format!("http://{addr}"), None)
	}

	/// A daemon that answers with the same canned bytes every time. It reads the
	/// request before answering it, because a server that closes with the
	/// request still unread closes ABORTIVELY — RST, not FIN — and on Windows
	/// that reset discards the response it had already sent; see
	/// `transport::testing::drain_request`.
	async fn serving(raw: &'static str) -> HttpTransport {
		let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = l.local_addr().unwrap();
		tokio::spawn(async move {
			while let Ok((mut s, _)) = l.accept().await {
				drain_request(&mut s).await;
				let _ = s.write_all(raw.as_bytes()).await;
				let _ = s.shutdown().await;
			}
		});
		HttpTransport::new(format!("http://{addr}"), None)
	}

	fn expose_header(resp: &Response<Vec<u8>>) -> Option<&str> {
		resp.headers()
			.get(header::ACCESS_CONTROL_EXPOSE_HEADERS)
			.and_then(|v| v.to_str().ok())
	}

	#[tokio::test]
	async fn relays_a_successful_response_untouched() {
		let _serialised = crate::FD_TESTS.lock().await;

		let t = serving("HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}")
			.await;
		let resp = proxy_once(resolved(t), get("/v0/health")).await;
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
		let resp = proxy_once(resolved(t), get("/v0/arrow/nope")).await;
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

		let resp = proxy_once(resolved(dead().await), get("/v0/health")).await;
		assert_eq!(resp.status(), 502);
		assert_eq!(
			resp.headers().get(PROXY_ERROR_HEADER).map(|v| v.as_bytes()),
			Some(&b"error"[..])
		);
	}

	#[tokio::test]
	async fn the_502_body_names_the_cause() {
		let _serialised = crate::FD_TESTS.lock().await;

		let resp = proxy_once(resolved(dead().await), get("/v0/health")).await;
		let body = String::from_utf8(resp.body().clone()).unwrap();
		assert!(
			body.contains("connect failed"),
			"a 502 must say why; got {body:?}"
		);
	}

	/// The two tests above dial a real dead port, so each of them is a race
	/// between the transport failing to connect and this proxy giving up — and
	/// they only mean anything when the transport wins it. What ships orders the
	/// two so it always does (connect 10s, proxy 300s); the cfg(test) pair used
	/// to invert that ordering (connect 250ms, proxy 20ms), and inverted, the
	/// winner is decided by how fast the platform refuses a loopback connect.
	/// Unix refuses in microseconds and the tests passed; the Windows runner does
	/// not, the proxy's timeout fired first, and both saw a marked 504 instead of
	/// the marked 502 they assert on. Ordered, the proxy cannot get there first:
	/// reqwest bounds the connect itself and reports the expiry as a connect
	/// error, so the failure is a `TransportError::Connect` inside
	/// `CONNECT_TIMEOUT` no matter what the stack does.
	#[test]
	fn the_proxy_ceiling_stays_above_the_transports_connect_ceiling() {
		assert!(
			PROXY_TIMEOUT > CONNECT_TIMEOUT,
			"a proxy ceiling at or below the transport's connect ceiling ({PROXY_TIMEOUT:?} \
			 vs {CONNECT_TIMEOUT:?}) turns every connect failure into a race the slower \
			 platform loses: the proxy answers 504 for a daemon that was never reached"
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

	/// A transport that fails the test if it is dialled at all. A preflight is
	/// answered by the proxy; reaching a daemon with it is the defect.
	struct NeverDialled;

	#[async_trait::async_trait]
	impl Transport for NeverDialled {
		async fn request(
			&self,
			_req: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			panic!("a CORS preflight must be answered by the proxy, not forwarded")
		}

		async fn open_ws(&self, _path: &str) -> Result<WsStream, TransportError> {
			panic!("open_ws is not part of the proxy path")
		}
	}

	/// A stalled daemon must not hang the request forever: `PROXY_TIMEOUT`
	/// turns it into a marked 504 rather than an open-ended wait. The code
	/// differs from the marked 502 a connect failure produces above only to
	/// name the cause for a human reading a log — `apiFetch` treats 502 and
	/// 504 identically, retrying an idempotent read on either as long as the
	/// marker is present (`src/lib/transport/api.ts`, `isProxyFailure`). What
	/// this test guards is the MARKER and the timeout firing at all; the exact
	/// code is diagnostic. `PROXY_TIMEOUT` is milliseconds under `cfg(test)`
	/// (see its definition), so this really waits it out rather than mocking
	/// the clock.
	#[tokio::test]
	async fn a_stalled_daemon_times_out_as_a_marked_504() {
		let resp = proxy_once(resolved(NeverResponds), get("/v0/health")).await;
		assert_eq!(resp.status(), 504);
		assert_eq!(
			resp.headers().get(PROXY_ERROR_HEADER).map(|v| v.as_bytes()),
			Some(&b"error"[..])
		);
	}

	/// The other half of finding B: resolving the active connection takes
	/// `ConnectionManager`'s read lock, which a switch can hold. Awaited
	/// OUTSIDE the timeout, that wait is unbounded and every `quiver://`
	/// request in the app hangs forever. Inside it, it is a marked 504 the
	/// frontend already retries. A transport that never even resolves stands in
	/// for the wedged lock.
	#[tokio::test]
	async fn a_transport_that_never_resolves_times_out_as_a_marked_504() {
		let never = async {
			std::future::pending::<()>().await;
			unreachable!("pending never resolves")
		};
		let resp = proxy_once(never, get("/v0/health")).await;
		assert_eq!(
			resp.status(),
			504,
			"the transport lookup must be inside PROXY_TIMEOUT, not before it"
		);
		assert_eq!(
			resp.headers().get(PROXY_ERROR_HEADER).map(|v| v.as_bytes()),
			Some(&b"error"[..])
		);
	}

	// ── CORS (finding C) ─────────────────────────────────────────────────────

	/// `DELETE`, and `POST` with a JSON body, are not simple requests, so the
	/// browser preflights them. The proxy answers; the daemon never hears about
	/// it. `NeverDialled` panics if it does.
	#[tokio::test]
	async fn a_preflight_is_answered_locally_without_dialling_the_transport() {
		let resp = proxy_once(resolved(NeverDialled), req("OPTIONS", "/v0/arrow/x")).await;
		assert_eq!(resp.status(), 204);

		let allow = |name: header::HeaderName| {
			resp.headers()
				.get(name)
				.and_then(|v| v.to_str().ok())
				.unwrap_or_default()
				.to_string()
		};
		let methods = allow(header::ACCESS_CONTROL_ALLOW_METHODS);
		for m in ["GET", "POST", "DELETE", "PATCH"] {
			assert!(
				methods.contains(m),
				"the frontend sends {m}; the preflight must allow it. Got {methods:?}"
			);
		}
		assert!(
			allow(header::ACCESS_CONTROL_ALLOW_HEADERS)
				.to_lowercase()
				.contains("content-type"),
			"a JSON POST sets Content-Type, which is not a safelisted request header"
		);
		assert_eq!(
			resp.headers()
				.get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
				.map(|v| v.as_bytes()),
			Some(&b"*"[..]),
			"a preflight without an allow-origin fails the preflight itself"
		);
	}

	/// Without `Access-Control-Expose-Headers`, `x-quiver-proxy` is invisible to
	/// the page — `res.headers.get(...)` reads `null` — so `isProxyFailure`
	/// never fires and the whole cold-start retry path in
	/// `src/lib/transport/api.ts` is dead. It is read on ERROR responses, so an
	/// error that omits the expose header is exactly as dead as one that omits
	/// the marker: every response has to carry it.
	#[tokio::test]
	async fn every_response_exposes_the_proxy_marker_to_the_page() {
		let _serialised = crate::FD_TESTS.lock().await;

		let ok = serving("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}").await;
		let daemon_404 =
			serving("HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\n\r\n{}").await;

		let cases: Vec<(&str, Response<Vec<u8>>)> = vec![
			(
				"a relayed 200",
				proxy_once(resolved(ok), get("/v0/health")).await,
			),
			(
				"a relayed daemon 404",
				proxy_once(resolved(daemon_404), get("/v0/arrow/nope")).await,
			),
			(
				"a marked 502",
				proxy_once(resolved(dead().await), get("/v0/health")).await,
			),
			(
				"a marked 504",
				proxy_once(resolved(NeverResponds), get("/v0/health")).await,
			),
			(
				"a preflight",
				proxy_once(resolved(NeverDialled), req("OPTIONS", "/v0/arrow"))
					.await,
			),
		];

		for (what, resp) in cases {
			assert_eq!(
				resp.headers()
					.get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
					.map(|v| v.as_bytes()),
				Some(&b"*"[..]),
				"{what} must be readable cross-origin at all"
			);
			assert_eq!(
				expose_header(&resp),
				Some(PROXY_ERROR_HEADER),
				"{what} must expose {PROXY_ERROR_HEADER}, or the page reads null for it"
			);
		}
	}

	/// A daemon that sets its own `Access-Control-Allow-Origin` must not leave
	/// the response with two of them — a browser rejects a duplicated
	/// allow-origin outright, which would break exactly the responses that were
	/// working before.
	#[tokio::test]
	async fn a_daemons_own_allow_origin_is_replaced_not_appended() {
		let _serialised = crate::FD_TESTS.lock().await;

		let t = serving(
			"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: https://elsewhere.example\r\nContent-Length: 2\r\n\r\n{}",
		)
		.await;
		let resp = proxy_once(resolved(t), get("/v0/health")).await;
		let origins: Vec<_> = resp
			.headers()
			.get_all(header::ACCESS_CONTROL_ALLOW_ORIGIN)
			.iter()
			.map(|v| v.to_str().unwrap_or_default().to_string())
			.collect();
		assert_eq!(origins, vec!["*".to_string()], "got {origins:?}");
	}
}
