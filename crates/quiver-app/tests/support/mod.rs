//! A fake quiver.core on the LOCAL connection's unix socket, a wiremock one for REMOTE connections,
//! and a `ServicesConfig` pointing at both. The local socket is the dev socket the connection layer
//! derives from a checkout directory, so a test scopes it with a temp dir and the daemon looks
//! "already running": `ensure_running` adopts it instead of spawning anything.
#![cfg(unix)]
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use quiver_app::services::{ChannelEvents, KeyringSecrets, NativeReleaseSource, ServicesConfig};
use quiver_core_client::bridge::WsBridgeManager;
use quiver_core_client::stores::{MemorySecretStore, MemorySettingsStore};
use quiver_core_client::{ConnectionManager, LocalConfig, LocalHost, ManagerDeps};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::task::JoinHandle;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub body: String,
}

pub enum Reply {
    Json(u16, String),
    /// Accept the request and never answer it.
    Stall,
    /// Upgrade to a WebSocket, send these text frames, then close with 1000 "completed".
    WebSocket(Vec<String>),
}

pub type Routes = Arc<dyn Fn(&Recorded) -> Reply + Send + Sync>;

/// Answers `{"success":true,"data":null}` to everything.
pub fn accept_everything() -> Routes {
    Arc::new(|_| Reply::Json(200, r#"{"success":true,"data":null}"#.into()))
}

pub struct FakeDaemon {
    pub requests: Arc<Mutex<Vec<Recorded>>>,
    seen: UnboundedReceiver<Recorded>,
    task: JoinHandle<()>,
}

impl Drop for FakeDaemon {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl FakeDaemon {
    pub fn bind(
        socket: &Path,
        routes: Routes,
    ) -> Self {
        let _ = std::fs::remove_file(socket);
        let listener = UnixListener::bind(socket).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (seen_tx, seen) = unbounded_channel();
        let log = requests.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let (routes, log, seen_tx) = (routes.clone(), log.clone(), seen_tx.clone());
                tokio::spawn(async move {
                    serve(stream, routes, log, seen_tx).await;
                });
            }
        });
        Self {
            requests,
            seen,
            task,
        }
    }

    /// Resolves when a request matching `wanted` arrives (or already did, since the last wait).
    pub async fn wait_for(
        &mut self,
        wanted: impl Fn(&Recorded) -> bool,
    ) -> Recorded {
        loop {
            let request = self.seen.recv().await.expect("daemon stopped");
            if wanted(&request) {
                return request;
            }
        }
    }

    pub fn paths(&self) -> Vec<(String, String)> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| (r.method.clone(), r.path.clone()))
            .collect()
    }
}

async fn serve(
    mut stream: UnixStream,
    routes: Routes,
    log: Arc<Mutex<Vec<Recorded>>>,
    seen: tokio::sync::mpsc::UnboundedSender<Recorded>,
) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let (head_len, content_len) = loop {
        let Ok(n) = stream.read(&mut chunk).await else {
            return;
        };
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..end]).to_lowercase();
            let len = head
                .lines()
                .find_map(|l| l.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            break (end + 4, len);
        }
    };
    while buf.len() < head_len + content_len {
        let Ok(n) = stream.read(&mut chunk).await else {
            return;
        };
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let head = String::from_utf8_lossy(&buf[..head_len]).into_owned();
    let mut request_line = head.lines().next().unwrap_or_default().split(' ');
    let recorded = Recorded {
        method: request_line.next().unwrap_or_default().to_string(),
        path: request_line.next().unwrap_or_default().to_string(),
        body: String::from_utf8_lossy(&buf[head_len..head_len + content_len]).into_owned(),
    };
    log.lock().unwrap().push(recorded.clone());
    let _ = seen.send(recorded.clone());

    match routes(&recorded) {
        Reply::Json(status, body) => {
            let response = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.shutdown().await;
        }
        Reply::Stall => std::future::pending::<()>().await,
        Reply::WebSocket(frames) => {
            use futures_util::SinkExt;
            use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
            use tokio_tungstenite::tungstenite::protocol::CloseFrame;
            use tokio_tungstenite::tungstenite::Message;
            // The request head was already consumed to route it, so replay it to the handshake.
            let replay = Replay {
                head: std::io::Cursor::new(buf),
                stream,
            };
            let Ok(mut ws) = tokio_tungstenite::accept_async(replay).await else {
                return;
            };
            for frame in frames {
                let _ = ws.send(Message::Text(frame)).await;
            }
            let _ = ws
                .close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: "completed".into(),
                }))
                .await;
        }
    }
}

/// A stream that first yields bytes already read from the socket, then the socket itself.
struct Replay {
    head: std::io::Cursor<Vec<u8>>,
    stream: UnixStream,
}

impl tokio::io::AsyncRead for Replay {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let position = self.head.position() as usize;
        let remaining = &self.head.get_ref()[position..];
        if !remaining.is_empty() {
            let n = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..n]);
            self.head.set_position((position + n) as u64);
            return std::task::Poll::Ready(Ok(()));
        }
        std::pin::Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for Replay {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        data: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.stream).poll_write(cx, data)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

/// A scoped local connection: its socket path is derived from the temp checkout directory.
pub struct LocalSetup {
    pub checkout: tempfile::TempDir,
    pub data: tempfile::TempDir,
    pub socket: PathBuf,
}

impl LocalSetup {
    pub fn new() -> Self {
        let checkout = tempfile::tempdir().unwrap();
        let socket = socket_of(&local_config(&checkout));
        Self {
            checkout,
            data: tempfile::tempdir().unwrap(),
            socket,
        }
    }

    pub fn local_config(&self) -> LocalConfig {
        local_config(&self.checkout)
    }

    /// A connection manager on the local connection, not started, with in-memory stores.
    pub fn manager(&self) -> (Arc<ConnectionManager>, Arc<ChannelEvents>) {
        let events = Arc::new(ChannelEvents::new());
        let manager = ConnectionManager::new(ManagerDeps {
            local: self.local_config(),
            settings: Arc::new(MemorySettingsStore::new()),
            secrets: Arc::new(MemorySecretStore::new()),
            events: events.clone(),
            bridge: Arc::new(WsBridgeManager::new()),
            self_announce: None,
        });
        (Arc::new(manager), events)
    }

    pub fn config(&self) -> ServicesConfig {
        ServicesConfig {
            data_dir: self.data.path().to_path_buf(),
            local: local_config(&self.checkout),
            secrets: Arc::new(KeyringSecrets::new(Arc::new(
                quiver_platform::MemorySecretStore::new(),
            ))),
            pages: None,
            release: Arc::new(NativeReleaseSource::new()),
            platform: "linux/amd64".into(),
        }
    }
}

impl Drop for LocalSetup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket);
    }
}

fn local_config(checkout: &tempfile::TempDir) -> LocalConfig {
    LocalConfig::new(checkout.path().to_str(), None)
}

fn socket_of(config: &LocalConfig) -> PathBuf {
    let LocalHost::Unix(path) = config.host();
    PathBuf::from(path)
}

/// A remote daemon that can be paired with (any code, token `remote-token`), is healthy, and
/// answers everything else with an empty success envelope. Every request it gets is recorded.
pub async fn remote_daemon() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v0/auth/pairing/redeem"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"success": true, "data": {"token": "remote-token"}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/versions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"success": true, "data": {"api": {"supported": ["v0"]}}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v0/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .mount(&server)
        .await;
    Mock::given(wiremock::matchers::any())
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"success": true, "data": null})),
        )
        .mount(&server)
        .await;
    server
}
