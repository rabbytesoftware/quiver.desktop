use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter as TauriEmitter};

use crate::connection::http::HttpClient;

// ── Domain types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CoreStatus {
	Starting,
	Ready,
	Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ArrowState {
	Absent,
	Installing,
	Updating,
	Ready,
	Running,
	Stopping,
	Draining,
	Detached,
	Uninstalling,
	Removed,
	Outdated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
	Pending,
	Running,
	Completed,
	Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepProgress {
	pub index: u32,
	pub title: String,
	pub status: StepStatus,
	#[serde(rename = "type")]
	pub step_type: String,
	pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRun {
	pub method: String,
	pub pid: Option<u32>,
	#[serde(default)]
	pub variables: HashMap<String, String>,
	#[serde(default)]
	pub steps: Vec<StepProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastOutcome {
	pub method: String,
	pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowListItem {
	pub namespace: String,
	pub name: String,
	pub version: String,
	pub state: ArrowState,
	pub active_run: Option<ActiveRun>,
	pub last_outcome: Option<LastOutcome>,
}

// ── Event payloads (Rust → TypeScript) ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreStatusPayload {
	pub status: CoreStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowRemovePayload {
	pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeUpdatePayload {
	pub namespace: String,
	pub state: ArrowState,
	pub active_run: Option<ActiveRun>,
	pub last_outcome: Option<LastOutcome>,
}

// ── Connection config ─────────────────────────────────────────────────────────

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
}

// ── Emitter trait ────────────────────────────────────────────────────────────

pub trait Emitter: Send + Sync + 'static {
	fn emit_core_status(&self, status: CoreStatus);
	fn emit_arrow_hydrate(&self, items: Vec<ArrowListItem>);
	fn emit_arrow_remove(&self, namespace: String);
	fn emit_runtime_update(&self, payload: RuntimeUpdatePayload);
}

impl Emitter for AppHandle {
	fn emit_core_status(&self, status: CoreStatus) {
		TauriEmitter::emit(self, "core://status", CoreStatusPayload { status }).ok();
	}
	fn emit_arrow_hydrate(&self, items: Vec<ArrowListItem>) {
		TauriEmitter::emit(self, "arrow://hydrate", items).ok();
	}
	fn emit_arrow_remove(&self, namespace: String) {
		TauriEmitter::emit(self, "arrow://remove", ArrowRemovePayload { namespace }).ok();
	}
	fn emit_runtime_update(&self, payload: RuntimeUpdatePayload) {
		TauriEmitter::emit(self, "runtime://update", payload).ok();
	}
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn arrow_state_deserializes_from_snake_case() {
		let state: ArrowState = serde_json::from_str(r#""installing""#).unwrap();
		assert_eq!(state, ArrowState::Installing);
	}

	#[test]
	fn step_status_deserializes() {
		let s: StepStatus = serde_json::from_str(r#""running""#).unwrap();
		assert_eq!(s, StepStatus::Running);
	}

	#[test]
	fn core_status_serializes_to_snake_case() {
		let json = serde_json::to_string(&CoreStatus::Disconnected).unwrap();
		assert_eq!(json, r#""disconnected""#);
	}
}
