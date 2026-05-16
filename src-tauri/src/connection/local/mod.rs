pub mod sidecar;
pub mod transport;

use std::sync::Arc;

use async_trait::async_trait;
use tauri::AppHandle;

use crate::connection::http::HttpClient;
use crate::connection::types::{ConnectionConfig, CoreStatus, Emitter, QuiverConnection, WsTarget};
use crate::connection::ws::WsManager;

use self::sidecar::SidecarManager;
use self::transport::LocalTransport;

pub struct LocalConnection {
	config: ConnectionConfig,
	http: Arc<HttpClient>,
	sidecar: SidecarManager,
	socket_path: String,
}

impl Default for LocalConnection {
	fn default() -> Self {
		Self::new()
	}
}

impl LocalConnection {
	pub fn new() -> Self {
		let socket_path = default_socket_path();
		let transport = Arc::new(LocalTransport::new(&socket_path));
		let http = Arc::new(HttpClient::new(transport));
		let config = ConnectionConfig {
			id: "local".into(),
			name: "Local".into(),
			kind: "local".into(),
			url: None,
			api_version: "v0".into(),
		};
		Self {
			config,
			http,
			sidecar: SidecarManager::new(&socket_path),
			socket_path,
		}
	}
}

fn default_socket_path() -> String {
	let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
	format!("{}/.quiver/quiver.sock", home)
}

#[async_trait]
impl QuiverConnection for LocalConnection {
	async fn start(&self, app: &AppHandle) {
		app.emit_core_status(CoreStatus::Starting);

		if let Err(e) = self.sidecar.spawn(app).await {
			log::error!("failed to spawn sidecar: {e}");
			app.emit_core_status(CoreStatus::Disconnected);
			return;
		}

		if let Err(e) = self.sidecar.wait_for_ready(&self.http).await {
			log::error!("sidecar not ready: {e}");
			app.emit_core_status(CoreStatus::Disconnected);
			return;
		}

		app.emit_core_status(CoreStatus::Ready);

		WsManager::new(
			WsTarget::Unix(self.socket_path.clone()),
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
