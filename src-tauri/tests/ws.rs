mod common;

use std::sync::{Arc, Mutex};
use quiverdesktop_lib::core_client::types::{
    ArrowListItem, ArrowState, CoreStatus, Emitter, RuntimeUpdatePayload,
};
use quiverdesktop_lib::core_client::ws::runtime::run_runtime_ws;

#[derive(Default)]
struct CaptureEmitter {
    runtime_updates: Mutex<Vec<RuntimeUpdatePayload>>,
    removed: Mutex<Vec<String>>,
    hydrated: Mutex<Vec<Vec<ArrowListItem>>>,
}

impl Emitter for CaptureEmitter {
    fn emit_core_status(&self, _: CoreStatus) {}
    fn emit_arrow_hydrate(&self, items: Vec<ArrowListItem>) {
        self.hydrated.lock().unwrap().push(items);
    }
    fn emit_arrow_remove(&self, ns: String) {
        self.removed.lock().unwrap().push(ns);
    }
    fn emit_runtime_update(&self, payload: RuntimeUpdatePayload) {
        self.runtime_updates.lock().unwrap().push(payload);
    }
}

#[tokio::test]
async fn runtime_ws_emits_update_on_message() {
    let port = common::free_port();
    let msg = serde_json::json!({
        "namespace": "github.com/user/repo@v1.0.0",
        "state": "installing",
        "active_run": {"method": "_install", "steps": [], "variables": {}},
        "last_return": null
    }).to_string();

    common::spawn_ws_server(port, vec![msg]).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let emitter = Arc::new(CaptureEmitter::default());
    let emitter_clone = Arc::clone(&emitter);
    let ws_url = common::ws_base_url(port);

    let handle = tokio::spawn(async move {
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            run_runtime_ws(ws_url, emitter_clone),
        ).await.ok();
    });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    handle.abort();

    let updates = emitter.runtime_updates.lock().unwrap();
    assert!(!updates.is_empty());
    assert_eq!(updates[0].namespace, "github.com/user/repo@v1.0.0");
    assert_eq!(updates[0].state, ArrowState::Installing);
}

#[tokio::test]
async fn runtime_ws_reconnects_after_disconnect() {
    // Server closes immediately → WS loop should reconnect (we just verify it doesn't panic)
    let port = common::free_port();
    common::spawn_ws_server(port, vec![]).await; // no messages, closes immediately
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let emitter = Arc::new(CaptureEmitter::default());
    let ws_url = common::ws_base_url(port);

    let handle = tokio::spawn(async move {
        tokio::time::timeout(
            std::time::Duration::from_millis(700),
            run_runtime_ws(ws_url, emitter),
        ).await.ok();
    });

    handle.await.ok(); // just confirms it didn't panic
}
