pub mod transport;

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tauri::AppHandle;

use crate::connection::http::{HttpClient, HttpTransport};
use crate::connection::types::{ConnectionConfig, CoreStatus, Emitter, QuiverConnection, WsTarget};
use crate::connection::ws::WsManager;

use self::transport::RemoteTransport;

pub struct RemoteConnection {
	config: ConnectionConfig,
	http: Arc<HttpClient>,
	transport: Arc<RemoteTransport>,
}

impl RemoteConnection {
	pub async fn new(
		id: String,
		name: String,
		url: String,
		token: String,
	) -> Result<Self, String> {
		let transport = Arc::new(RemoteTransport::new(&url, &token));
		let http = Arc::new(HttpClient::new(
			Arc::clone(&transport) as Arc<dyn HttpTransport>
		));
		let api_version = negotiate_version(&http).await;
		let config = ConnectionConfig {
			id,
			name,
			kind: "remote".into(),
			url: Some(url),
			api_version,
		};
		Ok(Self {
			config,
			http,
			transport,
		})
	}
}

async fn negotiate_version(http: &HttpClient) -> String {
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

	let bytes = match http.get_raw_bytes("/versions").await {
		Ok(b) => b,
		Err(_) => return "v0".into(),
	};

	let env = match serde_json::from_slice::<VersionEnvelope>(&bytes) {
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
		app.emit_core_status(CoreStatus::Starting);

		if let Err(e) = self.http.health().await {
			log::error!("remote not reachable: {e}");
			app.emit_core_status(CoreStatus::Disconnected);
			return;
		}

		app.emit_core_status(CoreStatus::Ready);

		WsManager::new(
			WsTarget::Tcp(self.transport.base_ws_url()),
			Arc::new(app.clone()),
		)
		.start();
	}

	async fn teardown(&self) {}

	fn http(&self) -> Arc<HttpClient> {
		Arc::clone(&self.http)
	}

	fn config(&self) -> &ConnectionConfig {
		&self.config
	}

	fn set_name(&mut self, name: String) {
		self.config.name = name;
	}
}

// ── Tests (expose negotiate_version for integration tests) ───────────────────

#[cfg(test)]
pub async fn negotiate_version_pub(http: &HttpClient) -> String {
	negotiate_version(http).await
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::http::{HttpError, HttpTransport};
	use async_trait::async_trait;
	use bytes::Bytes;
	use std::sync::Mutex;

	struct StubTransport {
		body: Mutex<String>,
	}

	impl StubTransport {
		fn ok(body: &str) -> Arc<Self> {
			Arc::new(Self {
				body: Mutex::new(body.into()),
			})
		}
		fn not_found() -> Arc<Self> {
			Arc::new(Self {
				body: Mutex::new("".into()),
			})
		}
	}

	#[async_trait]
	impl HttpTransport for StubTransport {
		async fn get_bytes(&self, _: &str) -> Result<Bytes, HttpError> {
			let body = self.body.lock().unwrap().clone();
			if body.is_empty() {
				Err(HttpError::Request("404".into()))
			} else {
				Ok(Bytes::from(body))
			}
		}
		async fn post_json(&self, _: &str, _: serde_json::Value) -> Result<(), HttpError> {
			Ok(())
		}
		async fn delete(&self, _: &str) -> Result<(), HttpError> {
			Ok(())
		}
	}

	#[tokio::test]
	async fn negotiate_version_picks_supported() {
		let json = r#"{"success":true,"data":{"version":"1.0","build_id":"1","api":{"supported":["v0"],"latest":"v0","min_client_version":"1.0.0"}}}"#;
		let http = HttpClient::new(StubTransport::ok(json));
		let v = negotiate_version(&http).await;
		assert_eq!(v, "v0");
	}

	#[tokio::test]
	async fn negotiate_version_falls_back_on_404() {
		let http = HttpClient::new(StubTransport::not_found());
		let v = negotiate_version(&http).await;
		assert_eq!(v, "v0");
	}
}
