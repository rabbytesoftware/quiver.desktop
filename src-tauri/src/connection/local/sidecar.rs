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
		// `sidecar(name)` resolves to `dirname(current_exe)/name`, so the
		// argument must be the bare binary name, NOT the `binaries/quiver`
		// path from tauri.conf.json's externalBin. The bundler places the
		// sidecar beside the main executable (Contents/MacOS/quiver on macOS),
		// so "binaries/quiver" resolves to Contents/MacOS/binaries/quiver and
		// fails with ENOENT in every bundled build. `tauri dev` runs
		// unbundled and never catches it — use `make dev-bundle` for that.
		app.shell()
			.sidecar("quiver")
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
