//! Generic WebSocket bridge over whatever connection is active.
//!
//! A page cannot open a `WebSocket` to quiver.core: its only reachable
//! endpoint is the `quiver://` URI-scheme proxy (see `proxy`), and the browser
//! `WebSocket` constructor rejects every scheme but `ws`/`wss`. So Rust is the
//! WebSocket client here — it dials the active connection's own transport (a
//! unix socket locally, TCP/TLS remotely; a WS upgrade is perfectly legal on
//! either) and bridges an arbitrary `/v0/...` WebSocket route both ways to the
//! webview.
//!
//!   * daemon → UI: each `Message::Text(text)` is pushed verbatim to the
//!     [`FrameSink`] the caller supplies at open time.
//!   * webview → daemon: `ws_send` enqueues the supplied text frame for the
//!     connection's writer task.
//!
//! Connections are keyed by a client-supplied id (`crypto.randomUUID` on the JS
//! side) so multiple scoped streams can coexist.
//!
//! # Connection lifetime
//!
//! A connection's socket stays open while *either* split half is alive, so both
//! the reader and the writer task must finish before its descriptor comes back. The
//! app is not rich in descriptors — macOS starts a GUI app at launchd's 256, and Go
//! raises its own limit while Rust does not — so a connection left half-alive burns
//! one for the life of the process.
//!
//! A connection can end three ways, and all three must converge on that teardown:
//!
//!   1. the daemon closes it (every watcher release, every workspace switch),
//!   2. the frontend closes it (`ws_close`),
//!   3. the page it belongs to goes away (a reload orphans every connection on it).
//!
//! So a [`Connection`] owns everything the connection needs to die: dropping it drops
//! the sender, which ends the writer task, and drops the cancel channel, which ends
//! the reader task. **Removing a connection from the map is therefore the one and only
//! teardown primitive** — the reader retires itself on (1), and `ws_close` and
//! [`WsBridgeManager::close_all`] do the removing for (2) and (3).
//!
//! Note what this deliberately does *not* rely on: a dead sink. Sending to a
//! webview-backed sink succeeds for as long as the *webview* is
//! alive — a reloaded page is the same webview. It therefore cannot detect a reload,
//! only app shutdown, and a reader waiting to be told its page is gone would wait
//! forever. Cancellation has to be explicit; case (3) is why.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

use crate::transport::Transport;
use crate::{Error, Result};

/// Sentinel pushed to the frame sink when the daemon closes the stream
/// (close frame, error, or EOF). The JS shim recognises it and fires `onclose`
/// so `wsManager` reconnects and the §6 cache re-seeds. A NUL prefix makes it
/// impossible to collide with a real JSON DTO frame (which always starts `{`).
pub const WS_CLOSE_SENTINEL: &str = "\u{0}quiver-ws-close";

/// The sentinel, extended with the daemon's close code and reason when its close
/// frame carried them (`sentinel NUL code NUL reason`). The discovery stream ends
/// with `1000 completed`, which is how the shim tells a finished pass from a
/// dropped one.
fn close_sentinel(info: Option<&(u16, String)>) -> String {
    match info {
        Some((code, reason)) => format!("{WS_CLOSE_SENTINEL}\u{0}{code}\u{0}{reason}"),
        None => WS_CLOSE_SENTINEL.to_string(),
    }
}

/// Where daemon → UI frames go: whatever the app hands in (a channel to the
/// shell, a plain recorder under test). The indirection is what lets a
/// connection's lifetime be exercised without standing up a UI. An implementation
/// must drop a frame silently when its consumer is gone: that says nothing about
/// the connection.
pub trait FrameSink: Send + Sync + 'static {
    fn send(&self, frame: String);
}

/// Everything a live connection needs in order to die. Dropping it ends both of the
/// connection's tasks, and so releases its socket: the sender going away ends the
/// writer, and the cancel channel going away ends the reader.
struct Connection {
    generation: u64,
    tx: mpsc::UnboundedSender<Message>,
    /// Held, never used. Its `Drop` is the signal.
    _cancel: oneshot::Sender<()>,
}

type Connections = Arc<Mutex<HashMap<String, Connection>>>;

/// A poisoned map is still a map: its only state is the entries themselves, so a
/// panic elsewhere cannot have left it half-updated.
fn lock(connections: &Connections) -> std::sync::MutexGuard<'_, HashMap<String, Connection>> {
    connections.lock().unwrap_or_else(|e| e.into_inner())
}

/// Maps a connection id to its live connection. One writer task per connection
/// serialises all outbound frames so command handlers never touch the socket.
#[derive(Default)]
pub struct WsBridgeManager {
    connections: Connections,
    next_generation: AtomicU64,
}

impl WsBridgeManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn sender(&self, conn_id: &str) -> Option<mpsc::UnboundedSender<Message>> {
        lock(&self.connections).get(conn_id).map(|c| c.tx.clone())
    }

    /// Enqueue a raw text frame (the JSON the frontend wants to publish) for a
    /// connection's writer task. The whole of `ws_send`.
    pub fn send(&self, conn_id: &str, data: String) -> Result<()> {
        match self.sender(conn_id) {
            Some(tx) => tx
                .send(Message::Text(data))
                .map_err(|_| Error::WsClosed(conn_id.to_string())),
            None => Err(Error::NoSuchWs(conn_id.to_string())),
        }
    }

    /// Retire one connection — teardown path (2), the frontend closing it. Removing
    /// the entry from the map is the whole of the work; see the module doc.
    pub fn close(&self, conn_id: &str) {
        lock(&self.connections).remove(conn_id);
    }

    /// Retires every connection. Called when a page load orphans them all: the JS
    /// that owned these ids is gone and will never call `ws_close` for them, and the
    /// new page opens fresh ids of its own.
    pub fn close_all(&self) {
        lock(&self.connections).clear();
    }
}

/// Open a WebSocket over the active connection's transport for `path` (a full
/// `/v0/...` route) and start streaming its frames to `on_message`. Each text frame
/// is forwarded RAW — the whole DTO object — because the frontend parses the
/// complete frame itself.
///
/// This is the transport half of `ws_open`, free of any UI state so it can be
/// driven directly against a real server.
///
/// Returns the reader task's handle. The app ignores it — the connection outlives the
/// call — but it is the only honest signal that a retired connection has finished
/// letting go of its socket, so tests block on it rather than on a peer that may never
/// react (which is exactly the case cancellation exists for).
pub async fn open_bridge<S: FrameSink>(
    transport: &dyn Transport,
    conn_id: String,
    path: String,
    on_message: S,
    manager: &WsBridgeManager,
) -> Result<tokio::task::JoinHandle<()>> {
    // The transport owns the dial and the upgrade: which host, which scheme and which
    // credentials those need is exactly what differs between a local unix socket and
    // a remote https peer, and the bridge is deliberately ignorant of all of it.
    let ws = transport.open_ws(&path).await.map_err(|e| {
        log::error!("ws_open: opening {path} failed: {e}");
        Error::OpenWs {
            path: path.clone(),
            source: Box::new(e),
        }
    })?;
    let (mut write, mut read) = ws.split();

    // Writer task: drain the mpsc and push frames at the socket one at a time. It
    // ends when every sender is dropped — which is what retiring the connection does.
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
        let _ = write.close().await;
    });

    let generation = manager.next_generation.fetch_add(1, Ordering::Relaxed);
    let (cancel, mut cancelled) = oneshot::channel();

    // Register before the reader can retire itself, so a connection the daemon closes
    // instantly still finds its own entry to remove. A re-used id would otherwise
    // strand the previous connection, so the displaced one is dropped — which retires
    // it in full, by the same one primitive.
    lock(&manager.connections).insert(
        conn_id.clone(),
        Connection {
            generation,
            tx,
            _cancel: cancel,
        },
    );

    // Reader task: forward each text frame verbatim to the frame sink until the
    // daemon closes the stream or this connection is retired out from under us.
    //
    // (Crowbar spells this `loop` + `break`; the two are equivalent, but tarpaulin
    // instruments the `loop` keyword and the arm that breaks out of it with counters
    // that never fire even when the arm's own body provably does, leaving two lines
    // of this file permanently and falsely uncovered. See the task report.)
    let connections = Arc::clone(&manager.connections);
    let reader = tokio::spawn(async move {
        let mut daemon_closed = false;
        let mut close_info: Option<(u16, String)> = None;
        while !daemon_closed {
            tokio::select! {
                // The connection was removed from the map: `ws_close`, a page load, or a
                // re-used id. Whoever removed it has already retired it; just let go of
                // the read half so the socket can close.
                //
                // Ending the writer half-closes the socket, and a healthy daemon answers
                // that by closing its end — which would wake this reader anyway. But a
                // daemon that has stopped reading (the wedge the watchdog exists for)
                // never will, and then the reader parks here for the life of the app,
                // holding the descriptor. Teardown must not depend on the peer.
                _ = &mut cancelled => break,
                frame = read.next() => match frame {
                    Some(Ok(Message::Text(text))) => on_message.send(text),
                    // Not a DTO — skip it and keep reading. Notably NOT the
                    // stream ending: a ping or a binary frame must not be
                    // mistaken for a close.
                    Some(Ok(Message::Close(frame))) => {
                        close_info = frame.map(|f| (u16::from(f.code), f.reason.to_string()));
                    }
                    Some(Ok(_)) => (),
                    // A close frame (`None` once the handshake completes) and a
                    // socket error are the same event to us: the daemon ended it.
                    Some(Err(_)) | None => daemon_closed = true,
                },
            }
        }
        drop(read);

        // The daemon ended it, so nobody else will retire it: remove the entry, which
        // drops the sender and lets the writer finish. Guard on the generation — if the
        // id has since been re-used, the entry belongs to a live connection and is not
        // ours to remove. When we were cancelled instead, whoever cancelled us already
        // removed it, so there is nothing to do here.
        if daemon_closed {
            let mut conns = lock(&connections);
            if conns
                .get(&conn_id)
                .is_some_and(|c| c.generation == generation)
            {
                conns.remove(&conn_id);
            }
        }

        // Both paths wait for the writer, so this task finishing means the socket's
        // other half is really gone — i.e. the descriptor is back. That makes the
        // reader's completion the connection's single, observable teardown point.
        let _ = writer.await;

        // Only tell the shim to reconnect if the daemon is what ended this. A cancelled
        // connection was closed, superseded, or orphaned by a page load, and in each case
        // whoever did it already knows. Announcing after the writer means a reconnect can
        // never race ahead of the descriptor it is about to need.
        if daemon_closed {
            on_message.send(close_sentinel(close_info.as_ref()));
        }
    });

    Ok(reader)
}

#[cfg(test)]
mod tests;
