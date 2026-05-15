pub mod http;
pub mod sidecar;
pub mod types;
pub mod ws;

use crate::core_client::{
	http::HttpClient,
	sidecar::SidecarManager,
	types::{CoreStatus, Emitter},
	ws::WsManager,
};
use std::sync::Arc;
use tauri::AppHandle;

const DEFAULT_PORT: u16 = 6982;

pub struct CoreClient {
	pub http: Arc<HttpClient>,
	sidecar: SidecarManager,
}

impl CoreClient {
	pub fn new() -> Self {
		let sidecar = SidecarManager::new(DEFAULT_PORT);
		let http = Arc::new(HttpClient::new(sidecar.base_url()));
		Self { http, sidecar }
	}

	pub async fn start(&self, app: AppHandle) {
		app.emit_core_status(CoreStatus::Starting);

		if let Err(e) = self.sidecar.spawn(&app).await {
			log::error!("failed to spawn sidecar: {e}");
			app.emit_core_status(CoreStatus::Disconnected);
			return;
		}

		if let Err(e) = self.sidecar.wait_for_ready().await {
			log::error!("sidecar not ready: {e}");
			app.emit_core_status(CoreStatus::Disconnected);
			return;
		}

		match self.http.fetch_arrows().await {
			Ok(items) => {
				let list_items = http::to_arrow_list_items(items);
				for chunk in list_items.chunks(100) {
					app.emit_arrow_hydrate(chunk.to_vec());
				}
			}
			Err(e) => log::warn!("initial hydration failed: {e}"),
		}

		app.emit_core_status(CoreStatus::Ready);

		let ws_manager = WsManager::new(
			self.sidecar.ws_base_url(),
			Arc::clone(&self.http),
			Arc::new(app),
		);
		ws_manager.start();
	}
}

impl Default for CoreClient {
	fn default() -> Self {
		Self::new()
	}
}
