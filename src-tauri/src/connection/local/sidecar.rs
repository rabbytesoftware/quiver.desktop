use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

use crate::connection::http::HttpClient;

const HEALTH_RETRY_MS: u64 = 200;
const HEALTH_MAX_ATTEMPTS: u32 = 25;

pub struct SidecarManager {
	socket_path: String,
}

impl SidecarManager {
	pub fn new(socket_path: impl Into<String>) -> Self {
		Self {
			socket_path: socket_path.into(),
		}
	}

	pub fn socket_path(&self) -> &str {
		&self.socket_path
	}

	pub async fn spawn(&self, app: &AppHandle) -> Result<(), String> {
		app.shell()
			.sidecar("binaries/quiver")
			.map_err(|e| e.to_string())?
			.args(["daemon", "--host", "unix://"])
			.spawn()
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	pub async fn wait_for_ready(&self, http: &HttpClient) -> Result<(), String> {
		for _ in 0..HEALTH_MAX_ATTEMPTS {
			if http.health().await.is_ok() {
				return Ok(());
			}
			tokio::time::sleep(Duration::from_millis(HEALTH_RETRY_MS)).await;
		}
		Err(format!(
			"quiver.core did not become ready after {}ms",
			HEALTH_RETRY_MS * HEALTH_MAX_ATTEMPTS as u64,
		))
	}
}
