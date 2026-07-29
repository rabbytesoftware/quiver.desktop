use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use tauri::AppHandle;

use crate::connection::transport::http::HttpTransport;
use crate::connection::transport::Transport;
use crate::connection::types::{ConnectionConfig, CoreStatus, Emitter, QuiverConnection};

/// How long the reachability probe in [`RemoteConnection::start`] may take.
///
/// Its own deadline, and a short one, because it cannot share the transport's:
/// `http::REQUEST_TIMEOUT` has to sit above the proxy's 300s ceiling so that a
/// long proxied request is bounded by the proxy rather than silently truncated
/// here — and a probe that takes five and a half minutes to say "unreachable"
/// is not a probe. `tokio::time::timeout` at the call site rather than a second
/// `reqwest::Client` so that `unix::UnixTransport`, which has no reqwest client
/// at all, is covered by the same bound.
///
/// This is what a switch's `prepare` step blocks on (`ConnectionManager::switch_to`),
/// so it is also the longest a switch to a stalled peer can hold `switching`.
#[cfg(not(test))]
const HEALTH_TIMEOUT: Duration = Duration::from_secs(10);

/// Same constant, shortened under test, for the reason `proxy::PROXY_TIMEOUT`
/// documents: no `test-util` feature in this crate, so the wait is real.
#[cfg(test)]
const HEALTH_TIMEOUT: Duration = Duration::from_millis(20);

pub struct RemoteConnection {
	config: ConnectionConfig,
	transport: Arc<dyn Transport>,
}

impl RemoteConnection {
	pub async fn new(
		id: String,
		name: String,
		url: String,
		token: String,
	) -> Result<Self, String> {
		let transport: Arc<dyn Transport> =
			Arc::new(HttpTransport::new(url.clone(), Some(token)));
		let api_version = negotiate_version(transport.as_ref()).await;
		let config = ConnectionConfig {
			id,
			name,
			kind: "remote".into(),
			url: Some(url),
			api_version,
		};
		Ok(Self { config, transport })
	}
}

fn health_request() -> Result<tauri::http::Request<Vec<u8>>, String> {
	tauri::http::Request::builder()
		.method("GET")
		.uri("quiver://localhost/v0/health")
		.body(Vec::new())
		.map_err(|e| e.to_string())
}

/// Is this peer answering right now? The probe half of [`RemoteConnection::start`],
/// lifted out of it so it can be driven without an `AppHandle` — `start`'s other
/// job is emitting status, which needs one.
///
/// Bounded by [`HEALTH_TIMEOUT`]. The unbounded version of this call is the one
/// that wedged the app: a peer that completed a TCP handshake and then said
/// nothing left it pending forever, under `ConnectionManager`'s write lock.
/// The lock is gone (see `manager::ConnectionManager::switch_to`) and so is the
/// unboundedness — either alone would have been half a fix, because a switch
/// that never returns is still a switch the user cannot escape.
async fn probe_health(transport: &dyn Transport) -> Result<(), String> {
	let req = health_request()?;
	match tokio::time::timeout(HEALTH_TIMEOUT, transport.request(req)).await {
		Ok(Ok(resp)) if resp.status().is_success() => Ok(()),
		Ok(Ok(resp)) => Err(format!("health check returned status {}", resp.status())),
		Ok(Err(e)) => Err(format!("health check failed: {e}")),
		Err(_) => Err(format!(
			"health check timed out after {}ms",
			HEALTH_TIMEOUT.as_millis()
		)),
	}
}

async fn negotiate_version(transport: &dyn Transport) -> String {
	#[derive(Deserialize)]
	struct VersionApi {
		supported: Vec<String>,
	}
	#[derive(Deserialize)]
	struct VersionData {
		api: VersionApi,
	}
	#[derive(Deserialize)]
	struct VersionEnvelope {
		data: Option<VersionData>,
	}

	let req = match tauri::http::Request::builder()
		.method("GET")
		.uri("quiver://localhost/versions")
		.body(Vec::new())
	{
		Ok(r) => r,
		Err(_) => return "v0".into(),
	};

	let resp = match transport.request(req).await {
		Ok(r) => r,
		Err(_) => return "v0".into(),
	};

	if !resp.status().is_success() {
		return "v0".into();
	}

	let env = match serde_json::from_slice::<VersionEnvelope>(resp.body()) {
		Ok(e) => e,
		Err(_) => return "v0".into(),
	};

	let data = match env.data {
		Some(d) => d,
		None => return "v0".into(),
	};

	// Only one known version exists so far, and every fallback branch above
	// also returns "v0" — so no test can tell "negotiated v0" apart from
	// "negotiation never ran, defaulted to v0" by inspecting this return
	// value alone until a second entry lands here. See
	// tests/remote_connection.rs for how that's worked around today (asserting
	// the request was made, not what it returned).
	let known = ["v0"];
	for version in known.iter().rev() {
		if data.api.supported.iter().any(|s| s == version) {
			return version.to_string();
		}
	}
	"v0".into()
}

#[async_trait]
impl QuiverConnection for RemoteConnection {
	async fn start(&self, app: &AppHandle) {
		log::info!(
			"[remote] starting — url: {:?}, api: {}",
			self.config.url,
			self.config.api_version
		);
		app.emit_core_status(CoreStatus::Starting);

		match probe_health(self.transport.as_ref()).await {
			Ok(()) => {
				log::info!("[remote] reachable — emitting core://status: ready");
				app.emit_core_status(CoreStatus::Ready);
			}
			Err(e) => {
				log::error!("[remote] {e}");
				app.emit_core_status(CoreStatus::Disconnected);
			}
		}
	}

	/// Nothing to tear down, and that is a statement about this connection rather
	/// than an omission.
	///
	/// A remote connection owns no process — the daemon belongs to whoever runs
	/// the host — and no socket of its own: the WebSocket legs belong to
	/// `WsBridgeManager`, which `retire_streams_and_teardown` closes just before
	/// calling this, and the `reqwest` client inside `HttpTransport` releases its
	/// pool when the last `Arc` to it drops, which is when this connection is
	/// replaced. `LocalConnection::teardown` has a body because a spawned child
	/// process is the one thing here that outlives its handle.
	async fn teardown(&self) {}

	fn transport(&self) -> Arc<dyn Transport> {
		Arc::clone(&self.transport)
	}

	fn config(&self) -> &ConnectionConfig {
		&self.config
	}

	fn set_name(&mut self, name: String) {
		self.config.name = name;
	}
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::transport::{TransportError, WsStream};
	use tauri::http::{Request, Response};

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
		async fn request(
			&self,
			_req: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			match &self.mode {
				StubMode::Status(status, body) => Response::builder()
					.status(*status)
					.body(body.clone().into_bytes())
					.map_err(|e| TransportError::Protocol(e.to_string())),
				StubMode::Unreachable => {
					Err(TransportError::Connect("connection refused".into()))
				}
				StubMode::Stalls => std::future::pending().await,
			}
		}

		async fn open_ws(&self, _path: &str) -> Result<WsStream, TransportError> {
			Err(TransportError::Protocol(
				"not supported by StubTransport".into(),
			))
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
	/// transport never reaches the peer at all (`Err(TransportError::Connect)`),
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
		assert!(err.contains("404"), "got {err:?}");
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
			err.contains("timed out"),
			"the failure must say the peer never answered; got {err:?}"
		);
	}
}
