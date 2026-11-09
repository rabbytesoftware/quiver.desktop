//! MCP server (stdio) that lets an agent drive the real Quiver app through the e2e bridge.
//!
//! Tools: `app_launch`, `app_stop`, `ui_tree`, `ui_find`, `ui_click`, `ui_type`, `ui_key`, `ui_scroll`,
//! `wait_signal`, `wait_node`, `state_dump`, `logs`, `screenshot`. See `docs/e2e.md`.

use anyhow::Result;
use quiver_e2e::app::AppProcess;
use quiver_e2e::client::Bridge;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Session {
    app: Option<AppProcess>,
    bridge: Option<Bridge>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut session = Session { app: None, bridge: None };
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut out = tokio::io::stdout();
    while let Some(line) = lines.next_line().await? {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let id = msg.get("id").cloned();
        let method = msg["method"].as_str().unwrap_or_default().to_string();
        let reply = match method.as_str() {
            "initialize" => Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "quiver-mcp", "version": env!("CARGO_PKG_VERSION") }
            })),
            "tools/list" => Some(json!({ "tools": tools() })),
            "tools/call" => Some(call(&mut session, &msg["params"]).await),
            _ => None,
        };
        if let (Some(id), Some(result)) = (id, reply) {
            let resp = json!({ "jsonrpc": "2.0", "id": id, "result": result });
            out.write_all(format!("{resp}\n").as_bytes()).await?;
            out.flush().await?;
        }
    }
    Ok(())
}

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description,
            "inputSchema": { "type": "object", "properties": props, "required": required } })
}

fn tools() -> Vec<Value> {
    let id = json!({ "id": { "type": "string", "description": "stable element id from ui_tree" } });
    vec![
        tool("app_launch", "Launch the app (built with --features e2e-bridge). env is an optional map.",
            json!({ "binary": { "type": "string" }, "env": { "type": "object" } }), &["binary"]),
        tool("app_stop", "Kill the launched app.", json!({}), &[]),
        tool("ui_tree", "Dump every element painted in the last frame (id, kind, label, bounds).", json!({}), &[]),
        tool("ui_find", "Find one element by id.", id.clone(), &["id"]),
        tool("ui_click", "Click an element by id.", id.clone(), &["id"]),
        tool("ui_type", "Type text, optionally into the element with this id.",
            json!({ "id": { "type": "string" }, "text": { "type": "string" } }), &["text"]),
        tool("ui_key", "Press a key chord, e.g. enter, cmd-k.", json!({ "keys": { "type": "string" } }), &["keys"]),
        tool("ui_scroll", "Scroll an element.",
            json!({ "id": { "type": "string" }, "dx": { "type": "number" }, "dy": { "type": "number" } }), &["id", "dy"]),
        tool("wait_signal", "Wait for a named app signal (never sleeps).",
            json!({ "signal": { "type": "string" }, "timeout_ms": { "type": "number" } }), &["signal"]),
        tool("wait_node", "Wait for an element to appear/disappear.",
            json!({ "id": { "type": "string" }, "present": { "type": "boolean" }, "timeout_ms": { "type": "number" } }), &["id"]),
        tool("state_dump", "Dump the app state as JSON.", json!({}), &[]),
        tool("logs", "Read the most recent app log lines.", json!({ "n": { "type": "number" } }), &[]),
        tool("screenshot", "Take a window screenshot (PNG).", json!({}), &[]),
    ]
}

fn text(s: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": s.into() }] })
}

fn error(s: impl Into<String>) -> Value {
    json!({ "isError": true, "content": [{ "type": "text", "text": s.into() }] })
}

async fn call(session: &mut Session, params: &Value) -> Value {
    let name = params["name"].as_str().unwrap_or_default();
    let args = &params["arguments"];
    match run(session, name, args).await {
        Ok(v) => v,
        Err(e) => error(e.to_string()),
    }
}

async fn run(session: &mut Session, name: &str, args: &Value) -> Result<Value> {
    if name == "app_launch" {
        let binary = PathBuf::from(args["binary"].as_str().unwrap_or_default());
        let env: Vec<(String, String)> = args["env"]
            .as_object()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string())).collect())
            .unwrap_or_default();
        let app = AppProcess::launch(&binary, &env).await?;
        session.bridge = Some(app.bridge().await?);
        let port = app.port();
        session.app = Some(app);
        return Ok(text(format!("launched; bridge on port {port}")));
    }
    if name == "app_stop" {
        if let Some(mut app) = session.app.take() {
            app.kill().await?;
        }
        session.bridge = None;
        return Ok(text("stopped"));
    }
    let bridge = session.bridge.as_mut().ok_or_else(|| anyhow::anyhow!("no app launched; call app_launch first"))?;
    let s = |k: &str| args[k].as_str().unwrap_or_default().to_string();
    let n = |k: &str, d: u64| args[k].as_u64().unwrap_or(d);
    Ok(match name {
        "ui_tree" => text(serde_json::to_string_pretty(&bridge.tree().await?)?),
        "ui_find" => text(serde_json::to_string_pretty(&bridge.find(&s("id")).await?)?),
        "ui_click" => {
            bridge.click(&s("id")).await?;
            text("clicked")
        }
        "ui_type" => {
            let id = args["id"].as_str();
            bridge.type_text(id, &s("text")).await?;
            text("typed")
        }
        "ui_key" => {
            bridge.key(&s("keys")).await?;
            text("pressed")
        }
        "ui_scroll" => {
            bridge.scroll(&s("id"), args["dx"].as_f64().unwrap_or(0.0) as f32, args["dy"].as_f64().unwrap_or(0.0) as f32).await?;
            text("scrolled")
        }
        "wait_signal" => {
            bridge.wait_signal(&s("signal"), n("timeout_ms", 30_000)).await?;
            text("signal received")
        }
        "wait_node" => {
            let present = args["present"].as_bool().unwrap_or(true);
            let node = bridge.wait_node(&s("id"), present, n("timeout_ms", 30_000)).await?;
            text(serde_json::to_string(&node)?)
        }
        "state_dump" => text(serde_json::to_string_pretty(&bridge.state().await?)?),
        "logs" => text(bridge.logs(n("n", 100) as usize).await?.join("\n")),
        "screenshot" => {
            use base64::Engine as _;
            let png = bridge.screenshot_png().await?;
            json!({ "content": [{ "type": "image", "mimeType": "image/png",
                "data": base64::engine::general_purpose::STANDARD.encode(png) }] })
        }
        other => return Err(anyhow::anyhow!("unknown tool {other}")),
    })
}
