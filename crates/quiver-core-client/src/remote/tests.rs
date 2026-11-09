use super::*;
use crate::transport::WsStream;
use crate::Error;
use ::http::{Request, Response};
use std::result::Result;
use std::time::Duration;

/// A `Transport` double that answers every `request()` the same way, so
/// `negotiate_version`'s fallback branches can be driven without a real
/// peer. `open_ws` is never exercised by these tests, so it always errors.
enum StubMode {
    /// A response the peer actually sent — status + body.
    Status(u16, String),
    /// The peer was never reached at all (dead host, dropped connection).
    Unreachable,
    /// The peer accepted the connection and then said nothing, ever. This
    /// is finding B's actual trigger, and the only mode whose request
    /// cannot end on its own.
    Stalls,
}

struct StubTransport {
    mode: StubMode,
}

impl StubTransport {
    fn ok(body: &str) -> Self {
        Self {
            mode: StubMode::Status(200, body.into()),
        }
    }
    fn not_found() -> Self {
        Self {
            mode: StubMode::Status(404, "".into()),
        }
    }
    /// Mirrors what a real `Transport` returns when the peer can't be
    /// reached at all — as opposed to `not_found()`, which is the peer
    /// answering with a 404. Both must fall back to "v0", but only one of
    /// them goes through `negotiate_version`'s `Err(_) => ...` arm.
    fn unreachable() -> Self {
        Self {
            mode: StubMode::Unreachable,
        }
    }
    /// A peer that completes the TCP handshake and then goes silent. Not a
    /// hypothetical: `reqwest::Client::new()` set no timeouts at all, so
    /// this is what a half-dead remote core looked like from in here.
    fn stalls() -> Self {
        Self {
            mode: StubMode::Stalls,
        }
    }
}

#[async_trait]
impl Transport for StubTransport {
    async fn request(&self, _req: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Error> {
        match &self.mode {
            StubMode::Status(status, body) => Response::builder()
                .status(*status)
                .body(body.clone().into_bytes())
                .map_err(|e| Error::Protocol(e.to_string())),
            StubMode::Unreachable => Err(Error::Connect("connection refused".into())),
            StubMode::Stalls => std::future::pending().await,
        }
    }

    async fn open_ws(&self, _path: &str) -> Result<WsStream, Error> {
        Err(Error::Protocol("not supported by StubTransport".into()))
    }
}

#[tokio::test]
async fn negotiate_version_picks_supported() {
    let json = r#"{"success":true,"data":{"version":"1.0","build_id":"1","api":{"supported":["v0"],"latest":"v0","min_client_version":"1.0.0"}}}"#;
    let transport = StubTransport::ok(json);
    let v = negotiate_version(&transport).await;
    assert_eq!(v, "v0");
}

#[tokio::test]
async fn negotiate_version_falls_back_on_404() {
    let transport = StubTransport::not_found();
    let v = negotiate_version(&transport).await;
    assert_eq!(v, "v0");
}

/// The case that fires when a user adds a connection to a dead host: the
/// transport never reaches the peer at all (`Err(Error::Connect)`),
/// which is a distinct code path from `not_found()`'s `Ok` with a 404
/// status — both must land on the same "v0" fallback, but only this one
/// exercises `negotiate_version`'s `Err(_) => return "v0"` arm.
#[tokio::test]
async fn negotiate_version_falls_back_when_the_peer_is_unreachable() {
    let transport = StubTransport::unreachable();
    let v = negotiate_version(&transport).await;
    assert_eq!(v, "v0");
}

// ── The reachability probe (finding B) ───────────────────────────────────

/// A peer that answers 200 is reachable, and the probe must say so — the
/// bound added below must not cost the working case anything.
#[tokio::test]
async fn a_peer_that_answers_is_reachable() {
    assert!(probe_health(&StubTransport::ok("{}")).await.is_ok());
}

/// A 404 on `/v0/health` is a peer that is up but is not serving this API.
/// Reported as an error, and one that names the status, because "the URL is
/// wrong" and "the host is down" need different fixes.
#[tokio::test]
async fn a_peer_that_answers_with_an_error_status_names_it() {
    let err = probe_health(&StubTransport::not_found()).await.unwrap_err();
    assert!(err.to_string().contains("404"), "got {err:?}");
}

#[tokio::test]
async fn an_unreachable_peer_is_an_error() {
    assert!(probe_health(&StubTransport::unreachable()).await.is_err());
}

/// Finding B, at its source. A peer that accepts a connection and then
/// stalls used to leave this call pending forever — and it ran under
/// `ConnectionManager`'s write lock, so it took every `quiver://` request
/// in the app with it. `HEALTH_TIMEOUT` is what ends it. Without the
/// bound this test does not fail, it HANGS, so it is wrapped: a suite that
/// never finishes is a worse signal than one that goes red.
#[tokio::test]
async fn a_stalled_peer_ends_at_the_health_deadline_rather_than_never() {
    let err = tokio::time::timeout(
        Duration::from_secs(5),
        probe_health(&StubTransport::stalls()),
    )
    .await
    .expect("the health probe must be bounded — an unbounded one wedges the app")
    .unwrap_err();
    assert!(
        err.to_string().contains("timed out"),
        "the failure must say the peer never answered; got {err:?}"
    );
}

// ── Pairing-code redemption ───────────────────────────────────────────

#[tokio::test]
async fn redeem_pairing_code_returns_the_token_on_success() {
    let json = r#"{"success":true,"data":{"token":"tok_abc123"}}"#;
    let token = redeem_pairing_code(
        &StubTransport::ok(json),
        "482913",
        "device-1",
        "Quiver Desktop",
    )
    .await
    .expect("redeem must succeed");
    assert_eq!(token, "tok_abc123");
}

/// A rejected code (expired, already claimed, or simply wrong) comes back
/// as a non-success status -- named in the error, the same way
/// `probe_health` names a bad status rather than swallowing it.
#[tokio::test]
async fn redeem_pairing_code_names_the_status_on_failure() {
    let err = redeem_pairing_code(
        &StubTransport::not_found(),
        "482913",
        "device-1",
        "Quiver Desktop",
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("404"), "got {err:?}");
}

#[tokio::test]
async fn redeem_pairing_code_fails_when_the_peer_is_unreachable() {
    let err = redeem_pairing_code(
        &StubTransport::unreachable(),
        "482913",
        "device-1",
        "Quiver Desktop",
    )
    .await
    .unwrap_err();
    assert!(!err.to_string().is_empty());
}

/// A malformed or token-less success response must not be mistaken for a
/// working pairing -- an empty string token would silently authenticate
/// nothing on every request that follows.
#[tokio::test]
async fn redeem_pairing_code_fails_when_the_response_carries_no_token() {
    let json = r#"{"success":true,"data":null}"#;
    let err = redeem_pairing_code(
        &StubTransport::ok(json),
        "482913",
        "device-1",
        "Quiver Desktop",
    )
    .await
    .unwrap_err();
    assert!(!err.to_string().is_empty());
}

/// A `Transport` double that records the one request it receives, so a
/// test can assert on exactly what `redeem_pairing_code` sent.
/// `StubTransport`'s modes above only ever answer -- none of them keep a
/// copy of what arrived.
struct CapturingTransport {
    captured: std::sync::Mutex<Option<Request<Vec<u8>>>>,
}

impl CapturingTransport {
    fn new() -> Self {
        Self {
            captured: std::sync::Mutex::new(None),
        }
    }
}

#[async_trait]
impl Transport for CapturingTransport {
    async fn request(&self, req: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Error> {
        *self.captured.lock().unwrap() = Some(req);
        Response::builder()
            .status(201)
            .body(br#"{"success":true,"data":{"token":"tok"}}"#.to_vec())
            .map_err(|e| Error::Protocol(e.to_string()))
    }

    async fn open_ws(&self, _path: &str) -> Result<WsStream, Error> {
        Err(Error::Protocol(
            "not supported by CapturingTransport".into(),
        ))
    }
}

#[tokio::test]
async fn redeem_pairing_code_sends_the_code_device_id_and_label() {
    let transport = CapturingTransport::new();
    redeem_pairing_code(&transport, "482913", "device-xyz", "Quiver Desktop")
        .await
        .expect("redeem must succeed");

    let captured = transport
        .captured
        .lock()
        .unwrap()
        .take()
        .expect("a request must have been captured");
    assert_eq!(captured.method(), "POST");
    assert_eq!(captured.uri().path(), "/v0/auth/pairing/redeem");
    let body = String::from_utf8(captured.body().clone()).unwrap();
    assert!(body.contains("\"code\":\"482913\""), "got {body}");
    assert!(body.contains("\"device_id\":\"device-xyz\""), "got {body}");
    assert!(body.contains("\"label\":\"Quiver Desktop\""), "got {body}");
}
