use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter as TauriEmitter};

use crate::connection::http::{HttpClient, HttpError};

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
	fn emit_arrow_event(&self, payload: serde_json::Value);
	fn emit_runtime_update(&self, payload: serde_json::Value);
	fn emit_connection_changed(&self, payload: serde_json::Value);
}

impl Emitter for AppHandle {
	fn emit_core_status(&self, status: CoreStatus) {
		TauriEmitter::emit(self, "core://status", serde_json::json!({ "status": status })).ok();
	}

	fn emit_arrow_event(&self, payload: serde_json::Value) {
		TauriEmitter::emit(self, "arrow://event", payload).ok();
	}

	fn emit_runtime_update(&self, payload: serde_json::Value) {
		TauriEmitter::emit(self, "runtime://update", payload).ok();
	}

	fn emit_connection_changed(&self, payload: serde_json::Value) {
		TauriEmitter::emit(self, "connection://changed", payload).ok();
	}
}

// ── Command error ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CommandError {
	pub code: u16,
	pub message: String,
}

impl From<HttpError> for CommandError {
	fn from(e: HttpError) -> Self {
		match e {
			HttpError::Request(msg) => CommandError { code: 503, message: msg },
			HttpError::Api { code, message } => CommandError { code, message },
			HttpError::Parse(e) => CommandError { code: 500, message: e.to_string() },
		}
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

	#[test]
	fn command_error_serializes_code_and_message() {
		let err = CommandError { code: 404, message: "not found".into() };
		let json = serde_json::to_value(&err).unwrap();
		assert_eq!(json["code"], 404);
		assert_eq!(json["message"], "not found");
	}

	#[test]
	fn command_error_from_http_request_error_uses_503() {
		use crate::connection::http::HttpError;
		let err: CommandError = HttpError::Request("socket closed".into()).into();
		assert_eq!(err.code, 503);
	}

	#[test]
	fn command_error_from_http_api_error_preserves_code() {
		use crate::connection::http::HttpError;
		let err: CommandError = HttpError::Api { code: 422, message: "state violation".into() }.into();
		assert_eq!(err.code, 422);
		assert_eq!(err.message, "state violation");
	}
}
