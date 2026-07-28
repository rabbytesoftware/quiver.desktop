use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

use crate::connection::transport::Transport;

use super::LocalHost;

const HEALTH_RETRY_MS: u64 = 200;
const HEALTH_MAX_ATTEMPTS: u32 = 25;

pub struct SidecarManager {
	host: LocalHost,
}

impl SidecarManager {
	pub fn new(host: LocalHost) -> Self {
		Self { host }
	}

	pub async fn spawn(&self, app: &AppHandle) -> Result<(), String> {
		// `sidecar(name)` resolves to `dirname(current_exe)/name`, so the
		// argument must be the bare binary name, NOT the `binaries/quiver`
		// path from tauri.conf.json's externalBin. `tauri dev` runs unbundled
		// and never catches a mistake here — use `make dev-bundle`.
		app.shell()
			.sidecar("quiver")
			.map_err(|e| e.to_string())?
			.args(["daemon", "--host", &self.host.host_arg()])
			.spawn()
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	pub async fn wait_for_ready(&self, transport: &dyn Transport) -> Result<(), String> {
		for _ in 0..HEALTH_MAX_ATTEMPTS {
			let req = tauri::http::Request::builder()
				.method("GET")
				.uri("quiver://localhost/v0/health")
				.body(Vec::new())
				.map_err(|e| e.to_string())?;
			if let Ok(resp) = transport.request(req).await {
				if resp.status().is_success() {
					return Ok(());
				}
			}
			tokio::time::sleep(Duration::from_millis(HEALTH_RETRY_MS)).await;
		}
		Err(format!(
			"quiver.core did not become ready after {}ms",
			HEALTH_RETRY_MS * HEALTH_MAX_ATTEMPTS as u64,
		))
	}
}
