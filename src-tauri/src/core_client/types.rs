use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter as TauriEmitter};

// ── Domain types ────────────────────────────────────────────────────────────

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

// ── Event payloads (Rust → TypeScript) ──────────────────────────────────────

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

// ── HTTP response shapes (from quiver.core REST API) ────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ApiEnvelope<T> {
    pub success: bool,
    pub error: Option<String>,
    pub data: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstalledVersion {
    #[serde(rename = "ref")]
    pub installed_ref: String,
    pub version: String,
    pub state: ArrowState,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArrowListResponseItem {
    pub namespace: String,
    pub name: String,
    pub versions: Vec<InstalledVersion>,
}

// ── WS message shapes (from quiver.core WebSocket) ──────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ArrowWsMessage {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub removed: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeWsMessage {
    pub namespace: String,
    pub state: ArrowState,
    pub active_run: Option<ActiveRunWs>,
    pub last_return: Option<LastReturnWs>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActiveRunWs {
    pub method: String,
    pub pid: Option<u32>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub steps: Vec<StepProgress>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LastReturnWs {
    pub method: String,
    pub outcome: String,
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
        self.emit("core://status", CoreStatusPayload { status }).ok();
    }

    fn emit_arrow_hydrate(&self, items: Vec<ArrowListItem>) {
        self.emit("arrow://hydrate", items).ok();
    }

    fn emit_arrow_remove(&self, namespace: String) {
        self.emit("arrow://remove", ArrowRemovePayload { namespace }).ok();
    }

    fn emit_runtime_update(&self, payload: RuntimeUpdatePayload) {
        self.emit("runtime://update", payload).ok();
    }
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_state_deserializes_from_snake_case() {
        let state: ArrowState = serde_json::from_str(r#""installing""#).unwrap();
        assert_eq!(state, ArrowState::Installing);
    }

    #[test]
    fn runtime_ws_message_parses_active_run() {
        let json = r#"{
            "namespace": "github.com/user/repo@v1.0.0",
            "state": "installing",
            "active_run": {
                "method": "_install",
                "steps": [{"index": 0, "title": "Fetch", "status": "running", "type": "fetch"}]
            },
            "last_return": null
        }"#;
        let msg: RuntimeWsMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.namespace, "github.com/user/repo@v1.0.0");
        assert_eq!(msg.state, ArrowState::Installing);
        assert!(msg.active_run.is_some());
    }

    #[test]
    fn arrow_ws_message_parses_removed_flag() {
        let json = r#"{"namespace": "github.com/user/repo@v1.0.0", "name": "X", "version": "1.0.0", "removed": true}"#;
        let msg: ArrowWsMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.removed, Some(true));
    }

    #[test]
    fn installed_version_deserializes_ref_field() {
        let json = r#"{"ref": "v1.0.0", "version": "1.0.0", "state": "ready"}"#;
        let v: InstalledVersion = serde_json::from_str(json).unwrap();
        assert_eq!(v.installed_ref, "v1.0.0");
    }
}
