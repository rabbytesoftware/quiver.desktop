//! Wire types of the control bridge: newline-delimited JSON-RPC 2.0 over a localhost TCP socket.
//!
//! Methods (params -> result):
//! - `app.ping` -> [`Ping`]
//! - `ui.tree` -> [`Tree`] (every element painted in the last frame, with its stable id)
//! - `ui.find` `{id}` -> `Node | null`
//! - `ui.click` `{id}`, `ui.hover` `{id}`, `ui.double_click` `{id}`
//! - `ui.type` `{id?, text}` focuses `id` (when given) and types `text`
//! - `ui.key` `{keys}` presses a chord such as `"enter"`, `"cmd-k"`, `"ctrl-shift-p"`
//! - `ui.scroll` `{id, dx, dy}` (logical pixels)
//! - `wait.signal` `{signal, timeout_ms}` waits for a named signal raised by the app (latched: a signal
//!   that already fired returns at once). Never a sleep.
//! - `wait.node` `{id, present, timeout_ms}` waits for an element to appear or disappear
//! - `state.dump` -> arbitrary JSON snapshot of the app state
//! - `logs.recent` `{n}` -> `[String]`
//! - `window.screenshot` -> `{png_base64}`
//! - `window.resize` `{width, height}`

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Environment variable that tells the app which port to serve the bridge on (`0` picks a free one).
pub const PORT_ENV: &str = "QUIVER_E2E_PORT";
/// The line the app prints on stdout once the bridge listens: `QUIVER_BRIDGE_PORT=<port>`.
pub const PORT_LINE_PREFIX: &str = "QUIVER_BRIDGE_PORT=";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl Response {
    pub fn ok(id: u64, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }

    pub fn err(id: u64, code: i64, message: impl Into<String>) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: None, error: Some(RpcError { code, message: message.into() }) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ping {
    pub version: String,
    pub pid: u32,
    pub frame: u64,
}

/// One element in the view tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub enabled: bool,
    pub state: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tree {
    pub frame: u64,
    pub nodes: Vec<Node>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        let req = Request { jsonrpc: "2.0".into(), id: 7, method: "ui.click".into(), params: serde_json::json!({"id":"a"}) };
        let back: Request = serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
        assert_eq!(back.id, 7);
        assert_eq!(back.params["id"], "a");
    }

    #[test]
    fn error_response_omits_result() {
        let json = serde_json::to_value(Response::err(1, -32601, "nope")).unwrap();
        assert!(json.get("result").is_none());
        assert_eq!(json["error"]["code"], -32601);
    }
}
