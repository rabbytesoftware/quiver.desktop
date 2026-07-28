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
	use std::sync::Mutex;
	use tauri::http::{Request, Response};

	/// A `Transport` double that answers every `request()` with a canned status
	/// and body, so `negotiate_version`'s fallback branches can be driven
	/// without a real peer. `open_ws` is never exercised by these tests, so it
	/// always errors.
	struct StubTransport {
		status: u16,
		body: Mutex<String>,
	}

	impl StubTransport {
		fn ok(body: &str) -> Self {
			Self {
				status: 200,
				body: Mutex::new(body.into()),
			}
		}
		fn not_found() -> Self {
			Self {
				status: 404,
				body: Mutex::new("".into()),
			}
		}
	}

	#[async_trait]
	impl Transport for StubTransport {
		async fn request(
			&self,
			_req: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			let body = self.body.lock().unwrap().clone();
			Response::builder()
				.status(self.status)
				.body(body.into_bytes())
				.map_err(|e| TransportError::Protocol(e.to_string()))
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
}
