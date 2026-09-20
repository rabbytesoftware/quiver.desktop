use tauri::{AppHandle, Emitter as TauriEmitter};

use crate::connection::types::{CoreStatus, CoreUpdateStatus, Emitter};

impl Emitter for AppHandle {
	fn emit_core_status(&self, status: CoreStatus) {
		TauriEmitter::emit(
			self,
			"core://status",
			serde_json::json!({ "status": status }),
		)
		.ok();
	}

	fn emit_connection_changed(&self, payload: serde_json::Value) {
		TauriEmitter::emit(self, "connection://changed", payload).ok();
	}

	fn emit_core_update_status(&self, status: CoreUpdateStatus) {
		TauriEmitter::emit(
			self,
			"core://update_status",
			serde_json::json!({ "status": status }),
		)
		.ok();
	}
}
