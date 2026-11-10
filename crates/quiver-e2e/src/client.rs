//! A client for the control bridge, used by scenarios and the MCP server.

use crate::protocol::{Node, Ping, Request, Response, Tree};
use crate::{Error, Result};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

/// A connected bridge. Calls are serialised: one request in flight at a time.
pub struct Bridge {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    next_id: u64,
}

impl Bridge {
    pub async fn connect(port: u16) -> Result<Self> {
        let stream = TcpStream::connect(("127.0.0.1", port)).await?;
        let (read, writer) = stream.into_split();
        Ok(Self {
            reader: BufReader::new(read),
            writer,
            next_id: 1,
        })
    }

    pub async fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let req = Request {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        };
        let mut line = serde_json::to_vec(&req)?;
        line.push(b'\n');
        self.writer.write_all(&line).await?;
        loop {
            let mut buf = String::new();
            if self.reader.read_line(&mut buf).await? == 0 {
                return Err(Error::Closed);
            }
            let resp: Response = serde_json::from_str(buf.trim())?;
            if resp.id != id {
                continue;
            }
            return match resp.error {
                Some(e) => Err(Error::Rpc {
                    code: e.code,
                    message: e.message,
                }),
                None => Ok(resp.result.unwrap_or(Value::Null)),
            };
        }
    }

    pub async fn ping(&mut self) -> Result<Ping> {
        Ok(serde_json::from_value(
            self.call("app.ping", json!({})).await?,
        )?)
    }

    pub async fn tree(&mut self) -> Result<Tree> {
        Ok(serde_json::from_value(
            self.call("ui.tree", json!({})).await?,
        )?)
    }

    pub async fn find(&mut self, id: &str) -> Result<Option<Node>> {
        Ok(serde_json::from_value(
            self.call("ui.find", json!({ "id": id })).await?,
        )?)
    }

    pub async fn click(&mut self, id: &str) -> Result<()> {
        self.call("ui.click", json!({ "id": id })).await.map(drop)
    }

    pub async fn type_text(&mut self, id: Option<&str>, text: &str) -> Result<()> {
        self.call("ui.type", json!({ "id": id, "text": text }))
            .await
            .map(drop)
    }

    pub async fn key(&mut self, keys: &str) -> Result<()> {
        self.call("ui.key", json!({ "keys": keys })).await.map(drop)
    }

    pub async fn scroll(&mut self, id: &str, dx: f32, dy: f32) -> Result<()> {
        self.call("ui.scroll", json!({ "id": id, "dx": dx, "dy": dy }))
            .await
            .map(drop)
    }

    /// Waits for a named signal; never a sleep. The app latches signals, so one that already fired returns at once.
    pub async fn wait_signal(&mut self, signal: &str, timeout_ms: u64) -> Result<()> {
        self.call(
            "wait.signal",
            json!({ "signal": signal, "timeout_ms": timeout_ms }),
        )
        .await
        .map(drop)
    }

    pub async fn wait_node(&mut self, id: &str, present: bool, timeout_ms: u64) -> Result<Node> {
        let v = self
            .call(
                "wait.node",
                json!({ "id": id, "present": present, "timeout_ms": timeout_ms }),
            )
            .await?;
        Ok(serde_json::from_value(v).unwrap_or_else(|_| Node {
            id: id.into(),
            kind: String::new(),
            label: String::new(),
            enabled: false,
            state: String::new(),
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }))
    }

    pub async fn state(&mut self) -> Result<Value> {
        self.call("state.dump", json!({})).await
    }

    pub async fn logs(&mut self, n: usize) -> Result<Vec<String>> {
        Ok(serde_json::from_value(
            self.call("logs.recent", json!({ "n": n })).await?,
        )?)
    }

    pub async fn screenshot_png(&mut self) -> Result<Vec<u8>> {
        use base64::Engine as _;
        let v = self.call("window.screenshot", json!({})).await?;
        let b64 = v["png_base64"].as_str().unwrap_or_default();
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| Error::Launch(e.to_string()))
    }
}
