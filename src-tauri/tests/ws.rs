mod common;

use quiverdesktop_lib::connection::types::{CoreStatus, Emitter, WsTarget};
use quiverdesktop_lib::connection::ws::runtime::run_runtime_ws;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct CaptureEmitter {
	runtime_updates: Mutex<Vec<serde_json::Value>>,
	arrow_events: Mutex<Vec<serde_json::Value>>,
}

impl Emitter for CaptureEmitter {
	fn emit_core_status(&self, _: CoreStatus) {}
	fn emit_arrow_event(&self, value: serde_json::Value) {
		self.arrow_events.lock().unwrap().push(value);
	}
	fn emit_runtime_update(&self, payload: serde_json::Value) {
		self.runtime_updates.lock().unwrap().push(payload);
	}
	fn emit_connection_changed(&self, _: serde_json::Value) {}
}

#[tokio::test]
async fn runtime_ws_emits_update_on_message() {
	let port = common::free_port();
	let msg = serde_json::json!({
	    "namespace": "github.com/user/repo@v1.0.0",
	    "state": "installing",
	    "active_run": {"method": "_install", "steps": [], "variables": {}},
	    "last_return": null
	})
	.to_string();

	common::spawn_ws_server(port, vec![msg]).await;
	tokio::time::sleep(std::time::Duration::from_millis(50)).await;

	let emitter = Arc::new(CaptureEmitter::default());
	let emitter_clone = Arc::clone(&emitter);
	let ws_url = common::ws_base_url(port);

	let handle = tokio::spawn(async move {
		tokio::time::timeout(
			std::time::Duration::from_secs(2),
			run_runtime_ws(WsTarget::Tcp(ws_url), emitter_clone),
		)
		.await
		.ok();
	});

	tokio::time::sleep(std::time::Duration::from_millis(500)).await;
	handle.abort();

	let updates = emitter.runtime_updates.lock().unwrap();
	assert!(!updates.is_empty());
	assert_eq!(updates[0]["namespace"], "github.com/user/repo@v1.0.0");
	assert_eq!(updates[0]["state"], "installing");
}

#[tokio::test]
async fn runtime_ws_reconnects_after_disconnect() {
	let port = common::free_port();
	common::spawn_ws_server(port, vec![]).await;
	tokio::time::sleep(std::time::Duration::from_millis(20)).await;

	let emitter = Arc::new(CaptureEmitter::default());
	let ws_url = common::ws_base_url(port);

	let handle = tokio::spawn(async move {
		tokio::time::timeout(
			std::time::Duration::from_millis(700),
			run_runtime_ws(WsTarget::Tcp(ws_url), emitter),
		)
		.await
		.ok();
	});

	handle.await.ok();
}
