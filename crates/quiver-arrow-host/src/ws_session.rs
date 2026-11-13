//! The WebSocket relay of one arrow page. The shim in the page replaces `WebSocket` and posts what the
//! page does as JSON (`ws-open`, `ws-send`, `ws-close`); this turns each into the daemon socket it stands
//! for and posts the daemon's side back (`ws-open`, `ws-message`, `ws-error`, `ws-close`).
//!
//! The page's host is fixed when the session is made, never read from the message, and the daemon path is
//! built by [`crate::ws_target`], so a page can only reach its own arrow's surface.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinHandle;

use crate::hosts::ArrowHosts;
use crate::upstream::{ArrowUpstream, WsRelay};
use crate::ws::ws_target;

/// What the shim posts. `__arrowShim` marks messages that are the shim's.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum FromPage {
    #[serde(rename = "hello")]
    Hello,
    #[serde(rename = "ws-open")]
    Open { id: String, path: String },
    #[serde(rename = "ws-send")]
    Send { id: String, data: String },
    #[serde(rename = "ws-close")]
    Close { id: String },
}

enum ToSocket {
    Send(String),
    Close,
}

struct Socket {
    commands: UnboundedSender<ToSocket>,
    task: JoinHandle<()>,
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// One page's sockets. Dropping it closes every one of them.
pub struct WsSession {
    host: String,
    hosts: Arc<ArrowHosts>,
    upstream: Arc<dyn ArrowUpstream>,
    /// JSON for the shim, one message per item.
    out: UnboundedSender<String>,
    sockets: HashMap<String, Socket>,
}

fn to_page(
    kind: &str,
    id: &str,
    extra: serde_json::Value,
) -> String {
    let mut message = json!({"__arrowShim": 1, "type": kind, "id": id});
    if let (Some(base), Some(more)) = (message.as_object_mut(), extra.as_object()) {
        base.extend(more.clone());
    }
    message.to_string()
}

impl WsSession {
    pub fn new(
        host: String,
        hosts: Arc<ArrowHosts>,
        upstream: Arc<dyn ArrowUpstream>,
        out: UnboundedSender<String>,
    ) -> Self {
        Self {
            host,
            hosts,
            upstream,
            out,
            sockets: HashMap::new(),
        }
    }

    /// Handles one message the page posted. Anything that is not the shim's is ignored.
    pub fn handle(
        &mut self,
        body: &str,
    ) {
        let Ok(message) = serde_json::from_str::<FromPage>(body) else {
            return;
        };
        match message {
            // A new document in the page: whatever the previous one left open is orphaned.
            FromPage::Hello => self.sockets.clear(),
            FromPage::Open { id, path } => self.open(id, &path),
            FromPage::Send { id, data } => {
                if let Some(socket) = self.sockets.get(&id) {
                    let _ = socket.commands.send(ToSocket::Send(data));
                }
            }
            FromPage::Close { id } => {
                if let Some(socket) = self.sockets.get(&id) {
                    let _ = socket.commands.send(ToSocket::Close);
                }
            }
        }
    }

    fn open(
        &mut self,
        id: String,
        path: &str,
    ) {
        let Ok(target) = ws_target(&self.hosts, &self.host, path) else {
            let _ = self.out.send(to_page("ws-error", &id, json!({})));
            let _ = self
                .out
                .send(to_page("ws-close", &id, json!({"code": 1008})));
            return;
        };
        let (commands, inbox) = unbounded_channel();
        let task = tokio::spawn(run_socket(
            self.upstream.clone(),
            target,
            id.clone(),
            inbox,
            self.out.clone(),
        ));
        self.sockets.insert(id, Socket { commands, task });
    }
}

/// Owns one daemon socket until either side ends it.
async fn run_socket(
    upstream: Arc<dyn ArrowUpstream>,
    target: String,
    id: String,
    mut inbox: tokio::sync::mpsc::UnboundedReceiver<ToSocket>,
    out: UnboundedSender<String>,
) {
    let mut relay: Box<dyn WsRelay> = match upstream.open_ws(&target).await {
        Ok(relay) => relay,
        Err(e) => {
            log::debug!("arrow websocket {target}: {e}");
            let _ = out.send(to_page("ws-error", &id, json!({})));
            let _ = out.send(to_page("ws-close", &id, json!({"code": 1006})));
            return;
        }
    };
    let _ = out.send(to_page("ws-open", &id, json!({})));
    loop {
        tokio::select! {
            frame = relay.recv() => match frame {
                Some(text) => {
                    let _ = out.send(to_page("ws-message", &id, json!({"data": text})));
                }
                None => {
                    let _ = out.send(to_page("ws-close", &id, json!({"code": 1006})));
                    return;
                }
            },
            command = inbox.recv() => match command {
                Some(ToSocket::Send(text)) => {
                    if relay.send(text).await.is_err() {
                        let _ = out.send(to_page("ws-error", &id, json!({})));
                    }
                }
                Some(ToSocket::Close) | None => {
                    relay.close().await;
                    let _ = out.send(to_page("ws-close", &id, json!({"code": 1000})));
                    return;
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use tokio::sync::mpsc::UnboundedReceiver;

    use super::*;
    use crate::message::{ArrowRequest, ArrowResponse};
    use crate::{Error, Result};

    /// An upstream whose sockets echo what is sent, and remember the path they were opened on.
    #[derive(Default)]
    struct Echo {
        opened: Mutex<Vec<String>>,
    }

    struct EchoSocket {
        queue: std::collections::VecDeque<String>,
        wake: Option<tokio::sync::Notify>,
    }

    #[async_trait]
    impl WsRelay for EchoSocket {
        async fn send(
            &mut self,
            text: String,
        ) -> Result<()> {
            self.queue.push_back(text);
            if let Some(wake) = &self.wake {
                wake.notify_one();
            }
            Ok(())
        }
        async fn recv(&mut self) -> Option<String> {
            loop {
                if let Some(text) = self.queue.pop_front() {
                    return Some(text);
                }
                self.wake.as_ref()?.notified().await;
            }
        }
        async fn close(&mut self) {}
    }

    #[async_trait]
    impl ArrowUpstream for Echo {
        async fn request(
            &self,
            _: ArrowRequest,
        ) -> Result<ArrowResponse> {
            Err(Error::Upstream("not used".into()))
        }
        async fn open_ws(
            &self,
            path: &str,
        ) -> Result<Box<dyn WsRelay>> {
            if path.contains("refused") {
                return Err(Error::Upstream("refused".into()));
            }
            self.opened.lock().unwrap().push(path.to_owned());
            Ok(Box::new(EchoSocket {
                queue: Default::default(),
                wake: Some(tokio::sync::Notify::new()),
            }))
        }
    }

    fn session() -> (WsSession, UnboundedReceiver<String>, Arc<Echo>, String) {
        let hosts = Arc::new(ArrowHosts::new());
        let host = hosts.register("github.com/user/chat");
        let upstream = Arc::new(Echo::default());
        let (out, rx) = unbounded_channel();
        let session = WsSession::new(host.clone(), hosts, upstream.clone(), out);
        (session, rx, upstream, host)
    }

    async fn next(rx: &mut UnboundedReceiver<String>) -> serde_json::Value {
        let line = rx.recv().await.expect("a message for the page");
        serde_json::from_str(&line).unwrap()
    }

    #[tokio::test]
    async fn a_socket_opens_on_the_arrows_own_path_and_frames_go_both_ways() {
        let (mut s, mut rx, upstream, _) = session();
        s.handle(r#"{"__arrowShim":1,"type":"ws-open","id":"a-1","path":"/ws?u=bob"}"#);
        let opened = next(&mut rx).await;
        assert_eq!(
            (opened["type"].as_str(), opened["id"].as_str()),
            (Some("ws-open"), Some("a-1"))
        );
        assert_eq!(
            upstream.opened.lock().unwrap().as_slice(),
            ["/v0/ui/github.com%2Fuser%2Fchat/ws?u=bob"]
        );

        s.handle(r#"{"type":"ws-send","id":"a-1","data":"hi"}"#);
        let echoed = next(&mut rx).await;
        assert_eq!(
            (echoed["type"].as_str(), echoed["data"].as_str()),
            (Some("ws-message"), Some("hi"))
        );

        s.handle(r#"{"type":"ws-close","id":"a-1"}"#);
        let closed = next(&mut rx).await;
        assert_eq!(
            (closed["type"].as_str(), closed["code"].as_u64()),
            (Some("ws-close"), Some(1000))
        );
    }

    #[tokio::test]
    async fn an_unsafe_path_never_reaches_the_daemon() {
        let (mut s, mut rx, upstream, _) = session();
        s.handle(r#"{"type":"ws-open","id":"x","path":"/../v0/health"}"#);
        assert_eq!(next(&mut rx).await["type"], "ws-error");
        assert_eq!(next(&mut rx).await["code"], 1008);
        assert!(upstream.opened.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_refused_socket_errors_then_closes_abnormally() {
        let (mut s, mut rx, _, _) = session();
        s.handle(r#"{"type":"ws-open","id":"r","path":"/refused"}"#);
        assert_eq!(next(&mut rx).await["type"], "ws-error");
        assert_eq!(next(&mut rx).await["code"], 1006);
    }

    #[tokio::test]
    async fn a_new_document_drops_the_sockets_of_the_old_one_and_noise_is_ignored() {
        let (mut s, mut rx, _, _) = session();
        s.handle("not json");
        s.handle(r#"{"type":"something-else"}"#);
        s.handle(r#"{"type":"ws-open","id":"o","path":"/ws"}"#);
        assert_eq!(next(&mut rx).await["type"], "ws-open");
        s.handle(r#"{"type":"hello"}"#);
        s.handle(r#"{"type":"ws-send","id":"o","data":"late"}"#);
        tokio::task::yield_now().await;
        assert!(rx.try_recv().is_err(), "a dropped socket says nothing more");
    }
}
