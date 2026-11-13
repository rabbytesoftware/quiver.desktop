//! The OS-process half of supervising the daemon: killing it, asking it to
//! stop, reading its output, probing its health.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio::sync::mpsc;

use crate::transport::Transport;

/// Take the spawned child out of `slot`, if there is one — the whole of the
/// locked section of a reap, and nothing else.
///
/// Written this way so the lock CANNOT be held across the kill: the guard's life
/// ends with this call, and the caller receives an owned child it can only kill
/// afterwards. The alternative shape — lock, match, kill inside the `Some` arm —
/// reads almost identically and holds a shared mutex across a blocking OS call
/// on the app's exit path.
///
/// Generic over the handle because a real `CommandChild` cannot be built without
/// a live process, and this rule is worth testing.
///
/// A poisoned lock is recovered rather than propagated. The only thing behind it
/// is an `Option`, so a panic elsewhere cannot have left it half-updated, and
/// "the daemon does not get reaped, and the quit panics again" is a bad way to
/// react to somebody else's bug.
pub(super) fn take_spawned<C>(slot: &Mutex<Option<C>>) -> Option<C> {
    slot.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// Stop the process now. `start_kill` is synchronous (a signal / `TerminateProcess`),
/// which is what lets this run on the way out of the app without an await.
pub(super) fn kill(mut child: Child) {
    let pid = child
        .id()
        .map_or_else(|| "?".to_string(), |p| p.to_string());
    match child.start_kill() {
        Ok(()) => log::info!("[local] killed the quiver.core we spawned (pid {pid})"),
        Err(e) => log::warn!("[local] could not kill quiver.core (pid {pid}): {e}"),
    }
}

/// One `POST /v0/system/shutdown`: did the daemon accept it in time?
pub(super) async fn request_shutdown(
    transport: &dyn Transport,
    timeout: Duration,
) -> bool {
    let Ok(req) = ::http::Request::builder()
        .method("POST")
        .uri("quiver://localhost/v0/system/shutdown")
        .body(Vec::new())
    else {
        return false;
    };
    matches!(
        tokio::time::timeout(timeout, transport.request(req)).await,
        Ok(Ok(resp)) if resp.status().is_success()
    )
}

/// What the daemon's process did, as far as the log is concerned. Mirrors the
/// events the Tauri shell plugin delivered for a sidecar.
#[derive(Debug)]
pub(super) enum CoreEvent {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    /// Reading one of the daemon's streams failed.
    Error(String),
    Terminated {
        code: Option<i32>,
        signal: Option<i32>,
    },
}

/// Read the daemon's output for as long as it lasts and log it, then log how it
/// ended.
///
/// This is the half of spawning that must never be dropped on the floor: the
/// pipes are where quiver.core's stdout and stderr come out, and closing them
/// means everything the daemon said goes nowhere. A daemon that died on startup
/// — wrong flag, address in use, missing dependency — would then produce
/// exactly one symptom in this app, "quiver.core did not become ready after
/// 5000ms", with the actual reason discarded.
///
/// Ends when both streams close, which happens once the process has terminated.
/// So this task cannot outlive the daemon, and does not need cancelling.
pub(super) fn spawn_supervisor(
    slot: Arc<Mutex<Option<Child>>>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
) {
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(reap_when_exited(Arc::clone(&slot)));
    tokio::spawn(async move {
        let pump = tokio::spawn(pump_events(rx));
        tokio::join!(
            read_stream(stdout, tx.clone(), CoreEvent::Stdout),
            read_stream(stderr, tx.clone(), CoreEvent::Stderr),
        );
        // Both pipes are closed, so the process is exiting or gone. Its status is
        // read from the slot without waiting: if `reap` has already taken the
        // child, the app ended it and there is nothing to report.
        let status = slot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
            .and_then(|child| child.try_wait().ok().flatten());
        if let Some(status) = status {
            let _ = tx
                .send(CoreEvent::Terminated {
                    code: status.code(),
                    signal: signal_of(&status),
                })
                .await;
        }
        drop(tx);
        let _ = pump.await;
    });
}

const REAP_POLL: Duration = Duration::from_millis(200);

/// Waits for the spawned daemon to exit and collects its status, whatever its pipes are doing.
///
/// The pipes closing is not a reliable sign of the exit: the daemon's own children (a supervised arrow
/// that outlives a self-update swap) inherit them and keep them open, and even when they do close, that
/// happens a moment before the exit is collectable, so the single `try_wait` after them can find nothing.
/// An exited child nobody waits for stays a zombie, and a zombie still answers `kill(pid, 0)`: the
/// daemon replacing itself polls exactly that and gave up with "daemon N still running after 1m30s".
///
/// Ends when the child has been collected or `reap` has taken it out of the slot. The status stays
/// cached in the `Child` for the `try_wait` the supervisor does after the pipes close.
pub(super) async fn reap_when_exited(slot: Arc<Mutex<Option<Child>>>) {
    loop {
        {
            let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
            match guard.as_mut() {
                None => return,
                Some(child) => match child.try_wait() {
                    Ok(None) => {}
                    Ok(Some(_)) | Err(_) => return,
                },
            }
        }
        tokio::time::sleep(REAP_POLL).await;
    }
}

#[cfg(unix)]
fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    std::os::unix::process::ExitStatusExt::signal(status)
}

#[cfg(not(unix))]
fn signal_of(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// Forward one stream's lines, as they arrive, until it closes.
pub(super) async fn read_stream<R>(
    stream: Option<R>,
    tx: mpsc::Sender<CoreEvent>,
    wrap: fn(Vec<u8>) -> CoreEvent,
) where
    R: AsyncRead + Unpin,
{
    let Some(mut stream) = stream else { return };
    let mut pending = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                pending.extend_from_slice(&chunk[..n]);
                while let Some(newline) = pending.iter().position(|b| *b == b'\n') {
                    let line: Vec<u8> = pending.drain(..=newline).collect();
                    if tx.send(wrap(line)).await.is_err() {
                        return;
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(CoreEvent::Error(e.to_string())).await;
                break;
            }
        }
    }
    if !pending.is_empty() {
        let _ = tx.send(wrap(pending)).await;
    }
}

/// Drain the daemon's event stream into the log, for as long as it lasts.
pub(super) async fn pump_events(mut events: mpsc::Receiver<CoreEvent>) {
    while let Some(event) = events.recv().await {
        if let Some((level, line)) = describe_event(&event) {
            log::log!(level, "{line}");
        }
    }
}

/// One event as a log line: at what level, and saying what.
///
/// Lifted out of [`pump_events`] so the mapping can be asserted directly —
/// `spawn` needs a real binary, so anything left inside it is beyond reach of
/// the suite.
pub(super) fn describe_event(event: &CoreEvent) -> Option<(log::Level, String)> {
    match event {
        CoreEvent::Stdout(bytes) => Some((log::Level::Info, core_line(bytes))),
        // Warn, not info: quiver.core's own logs come out here, and this is the
        // stream that carries the reason a start failed.
        CoreEvent::Stderr(bytes) => Some((log::Level::Warn, core_line(bytes))),
        CoreEvent::Error(e) => Some((log::Level::Error, format!("[core] {e}"))),
        // The daemon is gone. Worth a line even when it exited cleanly: the app
        // carries on holding a transport pointed at an address nothing answers
        // on any more, and this is the only notice of that.
        CoreEvent::Terminated { code, signal } => Some((
            log::Level::Warn,
            format!("[core] daemon exited — code: {code:?}, signal: {signal:?}"),
        )),
    }
}

/// A line the daemon wrote, tagged and stripped of the newline the pipe kept.
fn core_line(bytes: &[u8]) -> String {
    format!("[core] {}", String::from_utf8_lossy(bytes).trim_end())
}

/// One `/v0/health` call: is a daemon answering on this transport right now?
///
/// Every failure is the same answer — no daemon here — so they collapse to
/// `false`: a refused connection, a peer that is not quiver.core, a request
/// that will not even build. The caller's job is to spawn one or to keep
/// waiting, and neither depends on which.
pub(super) async fn health_ok(transport: &dyn Transport) -> bool {
    let req = match ::http::Request::builder()
        .method("GET")
        .uri("quiver://localhost/v0/health")
        .body(Vec::new())
    {
        Ok(r) => r,
        Err(_) => return false,
    };
    match transport.request(req).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
