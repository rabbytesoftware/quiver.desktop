use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tauri::AppHandle;

use crate::connection::transport::http::HttpTransport;
use crate::connection::transport::Transport;
use crate::connection::types::{ConnectionConfig, CoreStatus, Emitter, QuiverConnection};

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

		let req = match health_request() {
			Ok(r) => r,
			Err(e) => {
				log::error!("[remote] health check failed: {e}");
				app.emit_core_status(CoreStatus::Disconnected);
				return;
			}
		};

		match self.transport.request(req).await {
			Ok(resp) if resp.status().is_success() => {
				log::info!("[remote] reachable — emitting core://status: ready");
				app.emit_core_status(CoreStatus::Ready);
			}
			Ok(resp) => {
				log::error!(
					"[remote] health check returned status {}",
					resp.status()
				);
				app.emit_core_status(CoreStatus::Disconnected);
			}
			Err(e) => {
				log::error!("[remote] health check failed: {e}");
				app.emit_core_status(CoreStatus::Disconnected);
			}
		}
	}

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
}
