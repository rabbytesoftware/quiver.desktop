use super::process::{describe_event, pump_events, read_stream, CoreEvent};
use super::process::{health_ok, request_shutdown, take_spawned};
use super::*;
use crate::local::LocalHost;
use crate::transport::WsStream;
use crate::Error;
use ::http::{Request, Response};
use std::result::Result;
use std::sync::Mutex;
use std::time::Duration;

fn test_host() -> LocalHost {
    #[cfg(unix)]
    return LocalHost::Unix("/tmp/quiver-test.sock".into());
    #[cfg(windows)]
    return LocalHost::Pipe("quiver-test".into());
}

/// Answers `/v0/health` however the test needs it answered.
enum Peer {
    /// A daemon that is up.
    Healthy,
    /// Something is listening, but it is not quiver.core — the case the
    /// fixed local address makes possible, and the reason the decision to
    /// spawn is taken on a health ANSWER rather than on a bind.
    AStranger,
    /// Nothing is listening at all.
    Absent,
    /// Accepts the connection and never answers.
    Hung,
}

struct StubTransport(Peer);

#[async_trait::async_trait]
impl Transport for StubTransport {
    async fn request(
        &self,
        _req: Request<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, Error> {
        match self.0 {
            Peer::Healthy => Ok(Response::builder()
                .status(200)
                .body(br#"{"status":"ok"}"#.to_vec())
                .unwrap()),
            Peer::AStranger => Ok(Response::builder().status(404).body(Vec::new()).unwrap()),
            Peer::Absent => Err(Error::Connect("connection refused".into())),
            Peer::Hung => std::future::pending().await,
        }
    }

    async fn open_ws(
        &self,
        _path: &str,
    ) -> Result<WsStream, Error> {
        unreachable!("the health probe never opens a WebSocket")
    }
}

/// The probe `ensure_running` decides on. A daemon that answers is one the
/// app must NOT spawn a second copy of — that is the whole of finding D on
/// Windows, where the second copy used to land on a port of its own and
/// survive as an orphan.
#[tokio::test]
async fn a_daemon_that_answers_health_is_reported_running() {
    assert!(health_ok(&StubTransport(Peer::Healthy)).await);
}

/// Nothing listening: the app must spawn. This is the case that has to keep
/// working — a convergence check that reports "already running" when nothing
/// is would leave the app with no daemon at all.
#[tokio::test]
async fn an_unreachable_address_is_not_reported_running() {
    assert!(!health_ok(&StubTransport(Peer::Absent)).await);
}

/// The fixed local port is a shared name, and an unrelated process can hold
/// it. It cannot answer `/v0/health`, so the probe must say no — otherwise
/// the app skips its own spawn and proxies to a stranger, which is the one
/// outcome worse than not starting.
#[tokio::test]
async fn a_process_that_is_not_quiver_core_is_not_reported_running() {
    assert!(!health_ok(&StubTransport(Peer::AStranger)).await);
}

// ── Reaping the daemon ───────────────────────────────────────────────────

/// Stands in for a `Child`, which cannot be constructed without a running
/// process. Nothing about `take_spawned` depends on
/// what the handle is, which is exactly why it is generic.
#[derive(Debug, PartialEq)]
struct FakeChild(u32);

/// The property both callers of `reap` depend on. `ExitRequested` and `Exit`
/// both arrive on a normal quit, and a switch away from local can be followed
/// by one of them — so the handle must come out once and then be gone. A
/// second `kill()` on the same pid is a signal aimed at whatever the OS has
/// since given that number to.
#[test]
fn a_spawned_child_is_taken_exactly_once() {
    let slot = Mutex::new(Some(FakeChild(4242)));
    assert_eq!(take_spawned(&slot), Some(FakeChild(4242)));
    assert_eq!(
        take_spawned(&slot),
        None,
        "the second reap must find nothing: both exit events fire, and a \
			 re-kill targets a pid that is no longer ours"
    );
}

/// `teardown` takes `&self`, so this mutex is shared with every other caller,
/// and `kill()` consumes the child and blocks on the OS. The lock must
/// therefore be gone by the time the caller has a child to kill — which is
/// what returning it achieves, and what "lock, match, kill in the arm" would
/// not.
#[test]
fn the_lock_is_free_once_the_child_is_out_of_it() {
    let slot = Mutex::new(Some(FakeChild(1)));
    let child = take_spawned(&slot);
    assert!(child.is_some(), "the child must come out");
    assert!(
        slot.try_lock().is_ok(),
        "the kill runs after this point, so the lock must already be released"
    );
}

/// The ownership rule, through the real type. `ensure_running` adopts a daemon
/// that is already answering `/v0/health` rather than spawning one, so no
/// handle is recorded, and reaping must be a quiet no-op that does not even
/// ask the adopted daemon to shut down.
#[tokio::test]
async fn a_daemon_this_app_did_not_spawn_is_left_alone() {
    let manager = SidecarManager::new(LocalConfig {
        host: test_host(),
        dev_home: None,
        bundled_binary: None,
    });

    manager.reap(&StubTransport(Peer::Healthy)).await;

    assert!(
        manager
            .child
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_none(),
        "only `spawn` may fill this slot"
    );
}

const TICK: Duration = Duration::from_millis(50);

#[tokio::test]
async fn a_daemon_that_accepts_shutdown_is_not_killed() {
    assert!(request_shutdown(&StubTransport(Peer::Healthy), TICK).await);
}

/// Each of these must fall back to killing the remembered pid.
#[tokio::test]
async fn a_refused_or_unreachable_or_hung_shutdown_reports_failure() {
    assert!(!request_shutdown(&StubTransport(Peer::AStranger), TICK).await);
    assert!(!request_shutdown(&StubTransport(Peer::Absent), TICK).await);
    assert!(!request_shutdown(&StubTransport(Peer::Hung), TICK).await);
}

// ── The daemon's own output ──────────────────────────────────────────────

fn described(event: CoreEvent) -> (log::Level, String) {
    describe_event(&event).expect("this event must reach the log")
}

/// The daemon's stdout is where its startup lines come out. They used to go
/// nowhere, because the receiver carrying them was dropped.
#[test]
fn stdout_from_the_daemon_reaches_the_log_at_info() {
    let (level, line) = described(CoreEvent::Stdout(
        b"starting quiver daemon on unix://\n".to_vec(),
    ));
    assert_eq!(level, log::Level::Info);
    // Tagged, so a daemon line is distinguishable from a Quiver line, and
    // stripped of the newline the pipe kept — the logger adds its own.
    assert_eq!(line, "[core] starting quiver daemon on unix://");
}

/// The case the discarded receiver actually cost. A daemon that cannot bind
/// says so on stderr and exits; the app then reported only "did not become
/// ready after 5000ms" and threw the reason away.
#[test]
fn stderr_from_the_daemon_reaches_the_log_loudly() {
    let (level, line) = described(CoreEvent::Stderr(
        b"listen npipe \\\\.\\pipe\\quiver: address already in use\n".to_vec(),
    ));
    assert_eq!(level, log::Level::Warn);
    assert!(line.contains("address already in use"), "got {line:?}");
}

/// Not the daemon failing — the plugin failing to keep reading it. Distinct
/// enough to be worth its own level: nothing more will arrive on this stream.
#[test]
fn a_stream_error_is_logged_as_an_error() {
    let (level, line) = described(CoreEvent::Error("stdout is not utf-8".into()));
    assert_eq!(level, log::Level::Error);
    assert!(line.contains("stdout is not utf-8"), "got {line:?}");
}

/// A daemon that has exited leaves the app holding a transport aimed at an
/// address nothing answers on. The exit code is the whole of the diagnosis,
/// so it has to be in the line.
#[test]
fn a_terminated_daemon_names_its_exit_code_and_signal() {
    let (level, line) = described(CoreEvent::Terminated {
        code: Some(1),
        signal: None,
    });
    assert_eq!(level, log::Level::Warn);
    assert!(line.contains("code: Some(1)"), "got {line:?}");
    assert!(line.contains("signal: None"), "got {line:?}");
}

/// The pump has to consume the stream to the end and then stop on its own.
/// A pump that returned early would stop draining the pipe, and the plugin's
/// reader threads fill a bounded channel — so the daemon would eventually
/// block on its own stdout. A pump that never returned would leak a task per
/// spawn.
#[tokio::test]
async fn the_pump_drains_the_stream_and_ends_with_it() {
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    tx.send(CoreEvent::Stdout(b"one\n".to_vec()))
        .await
        .expect("the pump's channel must accept events");
    tx.send(CoreEvent::Stderr(b"two\n".to_vec()))
        .await
        .expect("the pump's channel must accept events");
    // The daemon is gone and the plugin has closed the channel. Without the
    // sender dropped this call never returns, which is the point.
    drop(tx);

    tokio::time::timeout(Duration::from_secs(5), pump_events(rx))
        .await
        .expect("the pump must end when the daemon's stream closes");
}

// ── Reading the daemon's streams ─────────────────────────────────────────

/// Each newline ends one event, whatever the chunking, and what the daemon wrote
/// without a trailing newline before it died is still delivered.
#[tokio::test]
async fn a_stream_is_read_line_by_line_and_the_unterminated_tail_is_kept() {
    use tokio::io::AsyncWriteExt;

    let (mut writer, reader) = tokio::io::duplex(64);
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    let reading = tokio::spawn(read_stream(Some(reader), tx, CoreEvent::Stdout));

    writer.write_all(b"one\ntw").await.unwrap();
    writer.write_all(b"o\nlast words").await.unwrap();
    drop(writer);
    reading.await.unwrap();

    let mut lines = Vec::new();
    while let Some(CoreEvent::Stdout(bytes)) = rx.recv().await {
        lines.push(String::from_utf8(bytes).unwrap());
    }
    assert_eq!(lines, ["one\n", "two\n", "last words"]);
}

#[tokio::test]
async fn a_missing_stream_ends_at_once() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    read_stream(None::<tokio::io::DuplexStream>, tx, CoreEvent::Stderr).await;
    assert!(rx.recv().await.is_none());
}

/// The daemon exits while a child of its own (a supervised arrow) still holds its stdout: the pipe never
/// reaches EOF, so only an explicit wait collects the exit. Left alone it stays a zombie, which
/// `kill(pid, 0)` reports as alive.
#[cfg(unix)]
#[tokio::test]
async fn a_daemon_that_exited_is_collected_even_while_its_pipes_stay_open() {
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};

    let mut child = tokio::process::Command::new("sh")
        .args(["-c", "sleep 30 & exit 0"])
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn sh");
    let pid = child.id().expect("pid");
    let _stdout = child.stdout.take();
    let slot = Arc::new(Mutex::new(Some(child)));
    super::process::reap_when_exited(Arc::clone(&slot)).await;
    assert!(
        !std::path::Path::new(&format!("/proc/{pid}")).exists(),
        "the exited daemon was left as a zombie"
    );
    let status = slot.lock().unwrap().as_mut().unwrap().try_wait().unwrap();
    assert!(status.is_some_and(|s| s.success()));
}
