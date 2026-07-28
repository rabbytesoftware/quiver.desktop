use std::sync::Arc;

use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tokio::sync::RwLock;

use crate::connection::local::LocalConnection;
use crate::connection::remote::RemoteConnection;
use crate::connection::types::{ConnectionConfig, QuiverConnection};

const KEYRING_SERVICE: &str = "quiver.desktop";
const STORE_KEY: &str = "connections";

pub struct ConnectionManager {
	active: RwLock<Box<dyn QuiverConnection>>,
}

impl Default for ConnectionManager {
	fn default() -> Self {
		Self::new()
	}
}

impl ConnectionManager {
	pub fn new() -> Self {
		Self {
			active: RwLock::new(Box::new(LocalConnection::new())),
		}
	}

	pub async fn start(&self, app: AppHandle) {
		self.active.read().await.start(&app).await;
	}

	pub async fn list_connections(&self, app: &AppHandle) -> Vec<ConnectionConfig> {
		let mut list = vec![self.active.read().await.config().clone()];
		list.extend(load_remote_configs(app).await);
		list
	}

	pub async fn get_connections(&self, app: &AppHandle) -> (Vec<ConnectionConfig>, String) {
		let active = self.active.read().await;
		let active_id = active.config().id.clone();
		let mut list = vec![active.config().clone()];
		drop(active);
		list.extend(load_remote_configs(app).await);
		(list, active_id)
	}

	pub async fn add_connection(
		&self,
		app: &AppHandle,
		name: String,
		url: String,
		token: String,
	) -> Result<ConnectionConfig, String> {
		let id = uuid::Uuid::new_v4().to_string();
		let conn = RemoteConnection::new(id.clone(), name, url, token.clone())
			.await
			.map_err(|e| e.to_string())?;
		let config = conn.config().clone();

		let entry = keyring::Entry::new(KEYRING_SERVICE, &id).map_err(|e| e.to_string())?;
		entry.set_password(&token).map_err(|e| e.to_string())?;

		let mut configs = load_remote_configs(app).await;
		configs.push(config.clone());
		save_remote_configs(app, &configs).await?;

		Ok(config)
	}

	pub async fn remove_connection(&self, app: &AppHandle, id: &str) -> Result<(), String> {
		if id == "local" {
			return Err("cannot remove local connection".into());
		}
		let mut configs = load_remote_configs(app).await;
		configs.retain(|c| c.id != id);
		save_remote_configs(app, &configs).await?;
		if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, id) {
			let _ = entry.delete_credential();
		}
		Ok(())
	}

	pub async fn switch_connection(&self, app: &AppHandle, id: &str) -> Result<(), String> {
		let new_conn = build_connection(app, id).await?;
		let mut guard = self.active.write().await;
		guard.teardown().await;
		*guard = new_conn;
		guard.start(app).await;
		Ok(())
	}

	pub async fn rename_connection(
		&self,
		app: &AppHandle,
		id: &str,
		name: String,
	) -> Result<(), String> {
		if id == "local" {
			return Err("cannot rename local connection".into());
		}
		let mut configs = load_remote_configs(app).await;
		match configs.iter_mut().find(|c| c.id == id) {
			Some(cfg) => cfg.name = name.clone(),
			None => return Err(format!("connection {id} not found")),
		}
		save_remote_configs(app, &configs).await?;
		// Keep active in-memory config in sync
		let mut guard = self.active.write().await;
		if guard.config().id == id {
			guard.set_name(name);
		}
		Ok(())
	}

	pub async fn transport(&self) -> Arc<dyn crate::connection::transport::Transport> {
		self.active.read().await.transport()
	}
}

async fn build_connection(app: &AppHandle, id: &str) -> Result<Box<dyn QuiverConnection>, String> {
	if id == "local" {
		return Ok(Box::new(LocalConnection::new()));
	}
	let configs = load_remote_configs(app).await;
	let cfg = configs
		.iter()
		.find(|c| c.id == id)
		.ok_or_else(|| format!("connection {id} not found"))?;
	let token = keyring::Entry::new(KEYRING_SERVICE, id)
		.and_then(|e| e.get_password())
		.map_err(|e| e.to_string())?;
	let conn = RemoteConnection::new(
		cfg.id.clone(),
		cfg.name.clone(),
		cfg.url.clone().unwrap_or_default(),
		token,
	)
	.await
	.map_err(|e| e.to_string())?;
	Ok(Box::new(conn))
}

// ── Persistence helpers ───────────────────────────────────────────────────────

async fn load_remote_configs(app: &AppHandle) -> Vec<ConnectionConfig> {
	let store = match app.store("connections.json") {
		Ok(s) => s,
		Err(_) => return vec![],
	};
	let value = match store.get(STORE_KEY) {
		Some(v) => v,
		None => return vec![],
	};
	serde_json::from_value(value).unwrap_or_default()
}

async fn save_remote_configs(app: &AppHandle, configs: &[ConnectionConfig]) -> Result<(), String> {
	let store = app.store("connections.json").map_err(|e| e.to_string())?;
	let value = serde_json::to_value(configs).map_err(|e| e.to_string())?;
	store.set(STORE_KEY, value);
	store.save().map_err(|e| e.to_string())
}
