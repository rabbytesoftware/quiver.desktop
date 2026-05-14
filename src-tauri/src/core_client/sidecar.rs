use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use crate::core_client::http::HttpClient;

const HEALTH_RETRY_INTERVAL_MS: u64 = 200;
const HEALTH_MAX_ATTEMPTS: u32 = 25;

pub struct SidecarManager {
    port: u16,
}

impl SidecarManager {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    pub fn ws_base_url(&self) -> String {
        format!("ws://localhost:{}", self.port)
    }

    pub async fn spawn(&self, app: &AppHandle) -> Result<(), String> {
        let port_str = self.port.to_string();
        app.shell()
            .sidecar("binaries/quiver")
            .map_err(|e| e.to_string())?
            .args(["daemon", "--port", &port_str])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn wait_for_ready(&self) -> Result<(), String> {
        let client = HttpClient::new(self.base_url());
        for _ in 0..HEALTH_MAX_ATTEMPTS {
            if client.health().await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(HEALTH_RETRY_INTERVAL_MS)).await;
        }
        Err(format!(
            "quiver.core did not become ready after {}ms",
            HEALTH_RETRY_INTERVAL_MS * HEALTH_MAX_ATTEMPTS as u64
        ))
    }
}
