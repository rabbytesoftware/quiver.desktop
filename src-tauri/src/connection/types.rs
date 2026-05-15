use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter as TauriEmitter};

use crate::connection::http::HttpClient;

// ── Connection config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CoreStatus {
	Starting,
	Ready,
	Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
	pub id: String,
	pub name: String,
	pub kind: String,
	pub url: Option<String>,
	pub api_version: String,
}

// ── WS target ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum WsTarget {
	Tcp(String),
	Unix(String),
}

// ── QuiverConnection trait ───────────────────────────────────────────────────

#[async_trait]
pub trait QuiverConnection: Send + Sync {
	async fn start(&self, app: &AppHandle);
	async fn teardown(&self);
	fn http(&self) -> Arc<HttpClient>;
	fn config(&self) -> &ConnectionConfig;
	fn set_name(&mut self, name: String);
}

// ── Emitter trait ────────────────────────────────────────────────────────────

pub trait Emitter: Send + Sync + 'static {
	fn emit_core_status(&self, status: CoreStatus);
	/// Bulk-hydrate arrows at startup from HTTP fetch.
	fn emit_arrow_hydrate(&self, items: Vec<serde_json::Value>);
	/// Forward a raw arrow WS event (upserted or removed) to TypeScript.
	fn emit_arrow_event(&self, payload: serde_json::Value);
	fn emit_runtime_update(&self, payload: serde_json::Value);
}

impl Emitter for AppHandle {
	fn emit_core_status(&self, status: CoreStatus) {
		TauriEmitter::emit(
			self,
			"core://status",
			serde_json::json!({ "status": status }),
		)
		.ok();
	}

	fn emit_arrow_hydrate(&self, items: Vec<serde_json::Value>) {
		TauriEmitter::emit(self, "arrow://hydrate", items).ok();
	}

	fn emit_arrow_event(&self, payload: serde_json::Value) {
		TauriEmitter::emit(self, "arrow://event", payload).ok();
	}

	fn emit_runtime_update(&self, payload: serde_json::Value) {
		TauriEmitter::emit(self, "runtime://update", payload).ok();
	}
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn core_status_serializes_to_snake_case() {
		let json = serde_json::to_string(&CoreStatus::Disconnected).unwrap();
		assert_eq!(json, r#""disconnected""#);
	}
}
