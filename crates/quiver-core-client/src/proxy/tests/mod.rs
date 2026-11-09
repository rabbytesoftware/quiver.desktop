use super::*;
use crate::transport::http::{HttpTransport, CONNECT_TIMEOUT};
use crate::transport::testing::drain_request;
use crate::transport::WsStream;
use crate::Error;
use std::future::ready;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

fn get(path: &str) -> Request<Vec<u8>> {
    req("GET", path)
}

/// A request the shell itself would send: cross-origin from its page, so
/// the browser always attaches the page's Origin.
fn req(method: &str, path: &str) -> Request<Vec<u8>> {
    shell_req(method, &format!("quiver://localhost{path}"))
}

fn shell_req(method: &str, uri: &str) -> Request<Vec<u8>> {
    let mut r = bare(method, uri);
    r.headers_mut().insert(
        header::ORIGIN,
        HeaderValue::from_static("tauri://localhost"),
    );
    r
}

/// A request with no Origin at all: a navigation or a subresource load.
fn bare(method: &str, uri: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Vec::new())
        .unwrap()
}

const HOSTS: [&str; 2] = ["quiver://localhost", "http://quiver.localhost"];
const METHODS: [&str; 4] = ["GET", "POST", "DELETE", "OPTIONS"];

/// Every refusal looks the same, whatever refused it: a marked 403 that
/// still carries the CORS and framing headers.
fn assert_refused(resp: &Response<Vec<u8>>, what: &str) {
    assert_eq!(resp.status(), 403, "{what}");
    assert_eq!(
        resp.headers().get(PROXY_ERROR_HEADER).map(|v| v.as_bytes()),
        Some(&b"error"[..]),
        "{what}"
    );
    assert_eq!(
        resp.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .map(|v| v.as_bytes()),
        Some(&b"*"[..]),
        "{what}"
    );
    assert_eq!(expose_header(resp), Some(PROXY_ERROR_HEADER), "{what}");
    assert_eq!(
        frame_guard(resp),
        (Some("frame-ancestors 'none'"), Some("DENY")),
        "{what}"
    );
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

    let t = serving("HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}").await;
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
/// error, so the failure is a `Error::Connect` inside
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
    async fn request(&self, _req: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Error> {
        std::future::pending().await
    }

    async fn open_ws(&self, _path: &str) -> Result<WsStream, Error> {
        std::future::pending().await
    }
}

/// A transport that fails the test if it is dialled at all. A preflight is
/// answered by the proxy; reaching a daemon with it is the defect.
struct NeverDialled;

#[async_trait::async_trait]
impl Transport for NeverDialled {
    async fn request(&self, _req: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Error> {
        panic!("a CORS preflight must be answered by the proxy, not forwarded")
    }

    async fn open_ws(&self, _path: &str) -> Result<WsStream, Error> {
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

mod cors;
mod paths;

fn frame_guard(resp: &Response<Vec<u8>>) -> (Option<&str>, Option<&str>) {
    let get = |name: header::HeaderName| resp.headers().get(name).and_then(|v| v.to_str().ok());
    (
        get(header::CONTENT_SECURITY_POLICY),
        get(header::X_FRAME_OPTIONS),
    )
}
