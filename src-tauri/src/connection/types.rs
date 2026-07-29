use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::AppHandle;

use crate::connection::transport::Transport;

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

// ── QuiverConnection trait ───────────────────────────────────────────────────

#[async_trait]
pub trait QuiverConnection: Send + Sync {
	async fn start(&self, app: &AppHandle);
	async fn teardown(&self);
	fn transport(&self) -> Arc<dyn Transport>;
	fn config(&self) -> &ConnectionConfig;
	fn set_name(&mut self, name: String);
}

// ── Emitter trait ────────────────────────────────────────────────────────────

pub trait Emitter: Send + Sync + 'static {
	fn emit_core_status(&self, status: CoreStatus);
	fn emit_connection_changed(&self, payload: serde_json::Value);
}

// ── Command error ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CommandError {
	pub code: u16,
	pub message: String,
}

impl From<String> for CommandError {
	fn from(message: String) -> Self {
		CommandError { code: 503, message }
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
		let err = CommandError {
			code: 404,
			message: "not found".into(),
		};
		let json = serde_json::to_value(&err).unwrap();
		assert_eq!(json["code"], 404);
		assert_eq!(json["message"], "not found");
	}

	#[test]
	fn command_error_from_string_uses_503() {
		let err: CommandError = "socket closed".to_string().into();
		assert_eq!(err.code, 503);
		assert_eq!(err.message, "socket closed");
	}
}
