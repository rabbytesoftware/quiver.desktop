//! A direct line to a quiver.core daemon, deliberately NOT through the app: the claim "core really holds the
//! row" must not be answered by anything inside the window it corroborates. Plain HTTP/1.1 over the daemon's
//! unix socket, or over loopback TCP for a daemon the test started itself.

use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};

#[derive(Clone)]
pub enum Endpoint {
    Unix(PathBuf),
    Tcp(u16),
}

#[derive(Clone)]
pub struct Daemon {
    endpoint: Endpoint,
    bearer: Option<String>,
}

pub struct Reply {
    pub status: u16,
    pub body: String,
}

impl Reply {
    /// The envelope's `data` (every v0 answer is `{success, error, data}`), or the raw document when unwrapped.
    pub fn data<T: DeserializeOwned>(&self) -> Option<T> {
        let doc: Value = serde_json::from_str(&self.body).ok()?;
        let inner = match doc.get("success") {
            Some(_) => doc.get("data").cloned().unwrap_or(Value::Null),
            None => doc,
        };
        serde_json::from_value(inner).ok()
    }

    pub fn json(&self) -> Value {
        self.data().unwrap_or(Value::Null)
    }
}

/// The catalog identity of a namespace, percent-encoded the way the product encodes it in a route.
pub fn enc(ns: &str) -> String {
    ns.replace('%', "%25")
        .replace('@', "%40")
        .replace('/', "%2F")
}

impl Daemon {
    pub fn unix(socket: PathBuf) -> Self {
        Self {
            endpoint: Endpoint::Unix(socket),
            bearer: None,
        }
    }

    pub fn tcp(port: u16) -> Self {
        Self {
            endpoint: Endpoint::Tcp(port),
            bearer: None,
        }
    }

    pub fn with_bearer(mut self, token: &str) -> Self {
        self.bearer = Some(token.to_string());
        self
    }

    pub async fn get(&self, path: &str) -> std::io::Result<Reply> {
        self.request("GET", path, None).await
    }

    pub async fn post(&self, path: &str, body: Option<Value>) -> std::io::Result<Reply> {
        self.request("POST", path, Some(body.unwrap_or(Value::Null)))
            .await
    }

    pub async fn patch(&self, path: &str, body: Value) -> std::io::Result<Reply> {
        self.request("PATCH", path, Some(body)).await
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> std::io::Result<Reply> {
        match &self.endpoint {
            Endpoint::Unix(p) => {
                exchange(
                    UnixStream::connect(p).await?,
                    method,
                    path,
                    &self.bearer,
                    body,
                )
                .await
            }
            Endpoint::Tcp(port) => {
                exchange(
                    TcpStream::connect(("127.0.0.1", *port)).await?,
                    method,
                    path,
                    &self.bearer,
                    body,
                )
                .await
            }
        }
    }

    pub async fn healthy(&self) -> bool {
        matches!(self.get("/v0/health").await, Ok(r) if r.status == 200)
    }
}

async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    method: &str,
    path: &str,
    bearer: &Option<String>,
    body: Option<Value>,
) -> std::io::Result<Reply> {
    let payload = body.filter(|b| !b.is_null()).map(|b| b.to_string());
    let mut head = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
    if let Some(token) = bearer {
        head.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    if let Some(p) = &payload {
        head.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            p.len()
        ));
    }
    head.push_str("\r\n");
    // One write, head and body together: the daemon may answer (and close) without reading a body it does not need,
    // and a body that lands after it closed makes the kernel reset the connection, which discards the answer the
    // client already received ("connection reset by peer" on a request the daemon logged as a 200).
    if let Some(p) = payload {
        head.push_str(&p);
    }
    stream.write_all(head.as_bytes()).await?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let chunked = head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked");
    Ok(Reply {
        status,
        body: if chunked {
            dechunk(body)
        } else {
            body.to_string()
        },
    })
}

fn dechunk(mut rest: &str) -> String {
    let mut out = String::new();
    while let Some((size, tail)) = rest.split_once("\r\n") {
        let Ok(n) = usize::from_str_radix(size.trim(), 16) else {
            break;
        };
        if n == 0 || tail.len() < n {
            break;
        }
        out.push_str(&tail[..n]);
        rest = tail[n..].trim_start_matches("\r\n");
    }
    out
}
