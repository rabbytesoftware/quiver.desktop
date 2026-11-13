//! The bridge server: JSON-RPC over localhost TCP, delegating UI actions to a [`Driver`].
//!
//! GPUI-free on purpose: `quiver-app` implements [`Driver`] on top of the real window and this module
//! stays unit-testable with a fake.

use crate::protocol::{Node, Ping, Request, Response, Tree, PORT_LINE_PREFIX};
use crate::signals::Signals;
use crate::{Error, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// What the bridge can ask of the running app. Element ids are the stable ids components report.
#[async_trait]
pub trait Driver: Send + Sync + 'static {
    async fn tree(&self) -> Tree;
    async fn click(
        &self,
        id: &str,
        double: bool,
    ) -> std::result::Result<(), String>;
    async fn hover(
        &self,
        id: &str,
    ) -> std::result::Result<(), String>;
    /// The secondary button, through the window system (it opens the OS's own context menu).
    async fn right_click(
        &self,
        id: &str,
    ) -> std::result::Result<(), String> {
        Err(format!(
            "right-click on {id} is not supported by this driver"
        ))
    }
    /// Keys through the window system rather than the app's dispatcher.
    async fn os_key(
        &self,
        keys: &str,
    ) -> std::result::Result<(), String> {
        Err(format!("OS keys {keys:?} are not supported by this driver"))
    }
    async fn type_text(
        &self,
        id: Option<&str>,
        text: &str,
    ) -> std::result::Result<(), String>;
    async fn key(
        &self,
        keys: &str,
    ) -> std::result::Result<(), String>;
    async fn scroll(
        &self,
        id: &str,
        dx: f32,
        dy: f32,
    ) -> std::result::Result<(), String>;
    async fn state(&self) -> Value;
    async fn logs(
        &self,
        n: usize,
    ) -> Vec<String>;
    async fn screenshot_png(&self) -> std::result::Result<Vec<u8>, String>;
    async fn resize(
        &self,
        width: f32,
        height: f32,
    ) -> std::result::Result<(), String>;
}

/// Shared handles the app keeps to feed the bridge.
#[derive(Clone)]
pub struct BridgeHandles {
    pub signals: Signals,
    /// Bumped by the app on every painted frame; `wait.node` re-checks the tree on each bump.
    pub frames: watch::Sender<u64>,
}

impl BridgeHandles {
    pub fn new() -> Self {
        Self {
            signals: Signals::default(),
            frames: watch::channel(0).0,
        }
    }
}

impl Default for BridgeHandles {
    fn default() -> Self {
        Self::new()
    }
}

/// Binds `127.0.0.1:port` (`0` = any), prints the port line on stdout and serves forever.
pub async fn serve<D: Driver>(
    port: u16,
    driver: Arc<D>,
    handles: BridgeHandles,
) -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let bound = listener.local_addr()?.port();
    println!("{PORT_LINE_PREFIX}{bound}");
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(connection(stream, driver.clone(), handles.clone()));
    }
}

async fn connection<D: Driver>(
    stream: TcpStream,
    driver: Arc<D>,
    handles: BridgeHandles,
) {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let resp = match serde_json::from_str::<Request>(&line) {
            Ok(req) => {
                let id = req.id;
                match dispatch(&*driver, &handles, &req.method, req.params).await {
                    Ok(v) => Response::ok(id, v),
                    Err((code, msg)) => Response::err(id, code, msg),
                }
            }
            Err(e) => Response::err(0, -32700, e.to_string()),
        };
        let Ok(mut bytes) = serde_json::to_vec(&resp) else {
            continue;
        };
        bytes.push(b'\n');
        if write.write_all(&bytes).await.is_err() {
            break;
        }
    }
}

type RpcResult = std::result::Result<Value, (i64, String)>;

fn app_err(e: String) -> (i64, String) {
    (-32000, e)
}

async fn dispatch<D: Driver>(
    d: &D,
    h: &BridgeHandles,
    method: &str,
    p: Value,
) -> RpcResult {
    let id = p["id"].as_str().unwrap_or_default();
    let timeout = Duration::from_millis(p["timeout_ms"].as_u64().unwrap_or(30_000));
    match method {
        "app.ping" => Ok(json!(Ping {
            version: env!("CARGO_PKG_VERSION").into(),
            pid: std::process::id(),
            frame: *h.frames.borrow()
        })),
        "ui.tree" => Ok(json!(d.tree().await)),
        "ui.find" => Ok(json!(d.tree().await.nodes.into_iter().find(|n| n.id == id))),
        "ui.click" => d.click(id, false).await.map(|_| json!({})).map_err(app_err),
        "ui.double_click" => d.click(id, true).await.map(|_| json!({})).map_err(app_err),
        "ui.hover" => d.hover(id).await.map(|_| json!({})).map_err(app_err),
        "ui.right_click" => d.right_click(id).await.map(|_| json!({})).map_err(app_err),
        "os.key" => d
            .os_key(p["keys"].as_str().unwrap_or_default())
            .await
            .map(|_| json!({}))
            .map_err(app_err),
        "ui.type" => d
            .type_text(p["id"].as_str(), p["text"].as_str().unwrap_or_default())
            .await
            .map(|_| json!({}))
            .map_err(app_err),
        "ui.key" => d
            .key(p["keys"].as_str().unwrap_or_default())
            .await
            .map(|_| json!({}))
            .map_err(app_err),
        "ui.scroll" => d
            .scroll(
                id,
                p["dx"].as_f64().unwrap_or(0.0) as f32,
                p["dy"].as_f64().unwrap_or(0.0) as f32,
            )
            .await
            .map(|_| json!({}))
            .map_err(app_err),
        "wait.signal" => {
            let name = p["signal"].as_str().unwrap_or_default();
            tokio::time::timeout(timeout, h.signals.wait(name))
                .await
                .map(|_| json!({}))
                .map_err(|_| (-32001, format!("timed out waiting for signal {name}")))
        }
        "wait.node" => {
            let present = p["present"].as_bool().unwrap_or(true);
            tokio::time::timeout(timeout, wait_node(d, h, &p, present))
                .await
                .map_err(|_| {
                    (
                        -32001,
                        format!("timed out waiting for node {id} present={present}"),
                    )
                })
        }
        "state.dump" => Ok(d.state().await),
        "logs.recent" => Ok(json!(d.logs(p["n"].as_u64().unwrap_or(100) as usize).await)),
        "window.screenshot" => {
            use base64::Engine as _;
            let png = d.screenshot_png().await.map_err(app_err)?;
            Ok(json!({ "png_base64": base64::engine::general_purpose::STANDARD.encode(png) }))
        }
        "window.resize" => d
            .resize(
                p["width"].as_f64().unwrap_or(1200.0) as f32,
                p["height"].as_f64().unwrap_or(800.0) as f32,
            )
            .await
            .map(|_| json!({}))
            .map_err(app_err),
        other => Err((-32601, format!("unknown method {other}"))),
    }
}

/// Which elements `wait.node` is about: `id` exactly, or every id starting with `prefix`, narrowed by
/// `label_contains` / `state_contains` (a node's free-form label and state text).
struct NodeFilter {
    id: Option<String>,
    prefix: Option<String>,
    label_contains: Option<String>,
    state_contains: Option<String>,
    /// Only nodes painted above / below this window y (`y_lt`, `y_gt`): how a scroll is observed.
    y_lt: Option<f32>,
    y_gt: Option<f32>,
    /// `enabled: true` keeps only nodes that accept input: a button whose form state has not caught up with
    /// the typing yet is painted disabled.
    enabled: Option<bool>,
}

impl NodeFilter {
    fn from_params(p: &Value) -> Self {
        let text = |k: &str| p[k].as_str().map(String::from);
        Self {
            id: text("id"),
            prefix: text("prefix"),
            label_contains: text("label_contains"),
            state_contains: text("state_contains"),
            y_lt: p["y_lt"].as_f64().map(|v| v as f32),
            y_gt: p["y_gt"].as_f64().map(|v| v as f32),
            enabled: p["enabled"].as_bool(),
        }
    }

    fn matches(
        &self,
        n: &Node,
    ) -> bool {
        self.id.as_ref().is_none_or(|id| &n.id == id)
            && self
                .prefix
                .as_ref()
                .is_none_or(|p| n.id.starts_with(p.as_str()))
            && self
                .label_contains
                .as_ref()
                .is_none_or(|t| n.label.contains(t.as_str()))
            && self
                .state_contains
                .as_ref()
                .is_none_or(|t| n.state.contains(t.as_str()))
            && self.y_lt.is_none_or(|y| n.y < y)
            && self.y_gt.is_none_or(|y| n.y > y)
            && self.enabled.is_none_or(|e| n.enabled == e)
    }
}

/// Re-checks the tree on every frame the app paints until the filter's presence matches.
async fn wait_node<D: Driver>(
    d: &D,
    h: &BridgeHandles,
    params: &Value,
    present: bool,
) -> Value {
    let filter = NodeFilter::from_params(params);
    let mut frames = h.frames.subscribe();
    loop {
        let found: Option<Node> = d.tree().await.nodes.into_iter().find(|n| filter.matches(n));
        if found.is_some() == present {
            return json!(found);
        }
        if frames.changed().await.is_err() {
            return json!(null);
        }
    }
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        std::io::Error::other(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Bridge;

    struct Fake;

    #[async_trait]
    impl Driver for Fake {
        async fn tree(&self) -> Tree {
            Tree {
                frame: 1,
                nodes: vec![Node {
                    id: "ok".into(),
                    kind: "button".into(),
                    label: "OK".into(),
                    enabled: true,
                    state: String::new(),
                    x: 1.0,
                    y: 2.0,
                    w: 3.0,
                    h: 4.0,
                }],
            }
        }
        async fn click(
            &self,
            id: &str,
            _d: bool,
        ) -> std::result::Result<(), String> {
            if id == "ok" {
                Ok(())
            } else {
                Err(format!("no element {id}"))
            }
        }
        async fn hover(
            &self,
            _: &str,
        ) -> std::result::Result<(), String> {
            Ok(())
        }
        async fn type_text(
            &self,
            _: Option<&str>,
            _: &str,
        ) -> std::result::Result<(), String> {
            Ok(())
        }
        async fn key(
            &self,
            _: &str,
        ) -> std::result::Result<(), String> {
            Ok(())
        }
        async fn scroll(
            &self,
            _: &str,
            _: f32,
            _: f32,
        ) -> std::result::Result<(), String> {
            Ok(())
        }
        async fn state(&self) -> Value {
            json!({"screen": "home"})
        }
        async fn logs(
            &self,
            _: usize,
        ) -> Vec<String> {
            vec!["hello".into()]
        }
        async fn screenshot_png(&self) -> std::result::Result<Vec<u8>, String> {
            Ok(vec![1, 2, 3])
        }
        async fn resize(
            &self,
            _: f32,
            _: f32,
        ) -> std::result::Result<(), String> {
            Ok(())
        }
    }

    async fn start() -> (Bridge, BridgeHandles) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let handles = BridgeHandles::new();
        let (driver, h) = (Arc::new(Fake), handles.clone());
        tokio::spawn(async move {
            loop {
                let (s, _) = listener.accept().await.unwrap();
                tokio::spawn(connection(s, driver.clone(), h.clone()));
            }
        });
        (Bridge::connect(port).await.unwrap(), handles)
    }

    #[tokio::test]
    async fn tree_find_click_and_errors_round_trip() {
        let (mut b, _) = start().await;
        assert_eq!(b.tree().await.unwrap().nodes.len(), 1);
        assert!(b.find("ok").await.unwrap().is_some());
        assert!(b.find("missing").await.unwrap().is_none());
        b.click("ok").await.unwrap();
        assert!(matches!(
            b.click("missing").await,
            Err(Error::Rpc { code: -32000, .. })
        ));
        assert_eq!(b.state().await.unwrap()["screen"], "home");
        assert_eq!(b.screenshot_png().await.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn wait_signal_returns_when_the_app_raises_it() {
        let (mut b, h) = start().await;
        h.signals.raise("core.ready");
        b.wait_signal("core.ready", 5_000).await.unwrap();
    }

    #[tokio::test]
    async fn wait_signal_times_out_on_a_signal_that_never_fires() {
        let (mut b, _) = start().await;
        assert!(matches!(
            b.wait_signal("never", 50).await,
            Err(Error::Rpc { code: -32001, .. })
        ));
    }

    #[tokio::test]
    async fn wait_node_absent_resolves_for_a_missing_node() {
        let (mut b, _) = start().await;
        b.wait_node("missing", false, 5_000).await.unwrap();
    }
}
