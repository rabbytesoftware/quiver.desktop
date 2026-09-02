use std::sync::Mutex;
use std::time::Duration;

use tauri::async_runtime::Receiver;
use tauri::AppHandle;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

use crate::connection::transport::Transport;

use super::{quiver_home, LocalHost};

const HEALTH_RETRY_MS: u64 = 200;
const HEALTH_MAX_ATTEMPTS: u32 = 25;

pub struct SidecarManager {
	host: LocalHost,
	/// The daemon THIS app spawned, and the only one it may ever kill.
	///
	/// `CommandChild` has no `Drop` impl, so a handle dropped on the floor — which
	/// is what `spawn` used to do with it — leaves the process running with
	/// nothing left in the app that can address it. Holding it here is what makes
	/// [`SidecarManager::reap`] possible at all.
	///
	/// `None` covers two very different situations, and conflating them is the
	/// mistake this field exists to prevent: nothing has been spawned yet, and
	/// `ensure_running` found a daemon already listening and adopted it. In the
	/// second case there is a live daemon on the address this app is talking to
	/// and it still must not be killed — it belongs to whoever started it.
	///
	/// A `std::sync::Mutex`, not tokio's, and deliberately: the critical section
	/// is one `Option::take` with no await in it (see `take_spawned`), and an
	/// async lock would invite exactly the "held across the kill" shape that
	/// `teardown`'s `&self` makes tempting.
	child: Mutex<Option<CommandChild>>,
}

impl SidecarManager {
	pub fn new(host: LocalHost) -> Self {
		Self {
			host,
			child: Mutex::new(None),
		}
	}

	/// Bring the local daemon up, or notice that it already is.
	///
	/// The probe comes FIRST, and that is the whole point. `LocalConnection::new()`
	/// runs at startup and again on every switch back to local, and each run used
	/// to spawn unconditionally. On unix that was merely wasteful — the second
	/// daemon loses the race for a fixed socket path and exits — and on Windows,
	/// where the port used to be picked fresh each time, it was an orphaned daemon
	/// per switch (see `local::LOCAL_TCP_PORT`). With a fixed address on both
	/// platforms, one health probe answers "is there already one of these?" and
	/// makes the second spawn unnecessary rather than merely doomed.
	///
	/// The probe also decides what "ready" means, which is what keeps the fixed
	/// port honest: if something that is not quiver.core holds the address, it
	/// cannot answer `/v0/health`, so this reports failure rather than handing
	/// the app a transport pointed at a stranger.
	///
	/// The early return is also what makes the ownership rule in [`Self::reap`]
	/// hold: an adopted daemon never reaches `spawn`, so no child handle is
	/// recorded for it, so nothing later kills it.
	pub async fn ensure_running(
		&self,
		app: &AppHandle,
		transport: &dyn Transport,
	) -> Result<(), String> {
		if health_ok(transport).await {
			log::info!("[local] a daemon is already listening — not spawning another");
			return Ok(());
		}
		self.spawn(app).await?;
		self.wait_for_ready(transport).await
	}

	/// Start the daemon and keep hold of both halves of what that returns.
	///
	/// Private on purpose: `ensure_running` is the only way in, because it is
	/// the probe that decides whether a spawn is wanted at all, and because a
	/// second spawn on the same manager would displace the child handle the
	/// first one recorded.
	async fn spawn(&self, app: &AppHandle) -> Result<(), String> {
		// `sidecar(name)` resolves to `dirname(current_exe)/name`, so the
		// argument must be the bare binary name, NOT the `binaries/quiver`
		// path from tauri.conf.json's externalBin. `tauri dev` runs unbundled
		// and never catches a mistake here — use `make dev-bundle`.
		// A dev/preview build points the daemon it spawns at this checkout's
		// own `.quiver`, never the user's real one — quiver.core only infers
		// this on its own for an unstamped `go run` build, and this sidecar is
		// always a version-stamped release download, so it never would.
		let dev_home = quiver_home();

		let mut command = app
			.shell()
			.sidecar("quiver")
			.map_err(|e| e.to_string())?
			.args(["daemon", "--host", &self.host.host_arg(dev_home.is_some())]);

		if let Some(home) = dev_home {
			command = command.env("QUIVER_HOME", home);
		}

		let (events, child) = command.spawn().map_err(|e| e.to_string())?;

		log::info!("[local] spawned quiver.core (pid {})", child.pid());

		// Nothing should be here — `ensure_running` probes before spawning and
		// each `LocalConnection` builds its own manager — but a handle displaced
		// from this slot is a daemon nothing can ever kill, so it is reaped
		// rather than overwritten.
		self.reap();
		self.store(child);

		// The receiver is the daemon's stdout and stderr, and dropping it (which
		// is what this function used to do) throws both away. See `pump_events`.
		tauri::async_runtime::spawn(pump_events(events));
		Ok(())
	}

	/// Record the child, and let go of the lock. See [`take_spawned`] for why the
	/// critical section is kept to exactly this.
	fn store(&self, child: CommandChild) {
		*self.child.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
	}

	/// Kill the daemon this app spawned, if this app spawned one.
	///
	/// Reached from two places: `LocalConnection::teardown`, which
	/// `ConnectionManager::retire_streams_and_teardown` runs when the app
	/// switches away from local, and the `RunEvent::ExitRequested`/`Exit` handler
	/// in `lib.rs`, which runs when the app quits. Before either existed, the
	/// daemon simply outlived the app: `CommandChild` has no `Drop`, so quitting
	/// Quiver left a `quiver daemon` running until the machine was rebooted or
	/// somebody found it in Activity Monitor.
	///
	/// Both callers can fire, and either can fire twice — `ExitRequested` and
	/// `Exit` both arrive on a normal quit. Taking the handle out of the slot is
	/// what makes every call after the first a no-op, rather than a second kill
	/// aimed at a pid the OS may by then have given to something else.
	///
	/// TWO THINGS THIS DELIBERATELY DOES NOT DO.
	///
	/// It does not kill whatever holds the local address. The slot is filled only
	/// by `spawn`, and `ensure_running` returns before spawning when a daemon is
	/// already answering `/v0/health` — so a `quiver daemon` a developer started
	/// in a terminal, or one belonging to a second instance of this app, is not
	/// ours, is not in the slot, and survives. Killing by port or by process name
	/// would take those down too.
	///
	/// And it does not hold the lock across the kill. `take_spawned` hands the
	/// child back and drops the guard; `kill()` then consumes it and signals the
	/// OS outside any critical section — which matters because `teardown` takes
	/// `&self`, so the lock is shared with every other caller, and this runs on
	/// the way out of the app where a block is a hang the user cannot escape.
	pub fn reap(&self) {
		let Some(child) = take_spawned(&self.child) else {
			log::debug!(
				"[local] nothing to reap: this app did not spawn the local daemon"
			);
			return;
		};
		let pid = child.pid();
		match child.kill() {
			Ok(()) => {
				log::info!("[local] killed the quiver.core we spawned (pid {pid})")
			}
			Err(e) => log::warn!("[local] could not kill quiver.core (pid {pid}): {e}"),
		}
	}

	/// See [`Self::spawn`]: private for the same reason.
	async fn wait_for_ready(&self, transport: &dyn Transport) -> Result<(), String> {
		for _ in 0..HEALTH_MAX_ATTEMPTS {
			if health_ok(transport).await {
				return Ok(());
			}
			tokio::time::sleep(Duration::from_millis(HEALTH_RETRY_MS)).await;
		}
		Err(format!(
			"quiver.core did not become ready after {}ms",
			HEALTH_RETRY_MS * HEALTH_MAX_ATTEMPTS as u64,
		))
	}
}

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
/// an `AppHandle` and a live process, and this rule is worth testing.
///
/// A poisoned lock is recovered rather than propagated. The only thing behind it
/// is an `Option`, so a panic elsewhere cannot have left it half-updated, and
/// "the daemon does not get reaped, and the quit panics again" is a bad way to
/// react to somebody else's bug.
fn take_spawned<C>(slot: &Mutex<Option<C>>) -> Option<C> {
	slot.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// Drain the daemon's event stream into the log, for as long as it lasts.
///
/// This is the half of `.spawn()` that used to be dropped on the floor.
/// `Command::spawn` returns `(Receiver<CommandEvent>, CommandChild)`, and the
/// receiver is where quiver.core's stdout and stderr come out: dropping it
/// closed the channel, so everything the daemon said went nowhere. A daemon that
/// died on startup — wrong flag, address in use, missing dependency — produced
/// exactly one symptom in this app, "quiver.core did not become ready after
/// 5000ms", with the actual reason discarded a few lines earlier.
///
/// Ends when the channel closes, which the plugin does once the process has
/// terminated and its reader threads are done. So this task cannot outlive the
/// daemon, and does not need cancelling.
async fn pump_events(mut events: Receiver<CommandEvent>) {
	while let Some(event) = events.recv().await {
		if let Some((level, line)) = describe_event(&event) {
			log::log!(level, "{line}");
		}
	}
}

/// One event as a log line: at what level, and saying what.
///
/// Lifted out of [`pump_events`] so the mapping can be asserted directly —
/// `spawn` needs an `AppHandle` and a real binary, so anything left inside it is
/// beyond reach of the suite.
fn describe_event(event: &CommandEvent) -> Option<(log::Level, String)> {
	match event {
		CommandEvent::Stdout(bytes) => Some((log::Level::Info, core_line(bytes))),
		// Warn, not info: quiver.core's own logs come out here, and this is the
		// stream that carries the reason a start failed.
		CommandEvent::Stderr(bytes) => Some((log::Level::Warn, core_line(bytes))),
		CommandEvent::Error(e) => Some((log::Level::Error, format!("[core] {e}"))),
		// The daemon is gone. Worth a line even when it exited cleanly: the app
		// carries on holding a transport pointed at an address nothing answers
		// on any more, and this is the only notice of that.
		CommandEvent::Terminated(status) => Some((
			log::Level::Warn,
			format!(
				"[core] daemon exited — code: {:?}, signal: {:?}",
				status.code, status.signal
			),
		)),
		// `CommandEvent` is `#[non_exhaustive]`: a variant added by a future
		// version of the plugin is something this app has no opinion about, and
		// must not be a compile error either.
		_ => None,
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
async fn health_ok(transport: &dyn Transport) -> bool {
	let req = match tauri::http::Request::builder()
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::transport::{TransportError, WsStream};
	use tauri::http::{Request, Response};
	use tauri_plugin_shell::process::TerminatedPayload;

	/// Answers `/v0/health` however the test needs it answered.
	enum Peer {
		/// A daemon that is up.
		Healthy,
		/// Something is listening, but it is not quiver.core — the case the
		/// fixed port on Windows makes possible, and the reason the decision to
		/// spawn is taken on a health ANSWER rather than on a bind.
		AStranger,
		/// Nothing is listening at all.
		Absent,
	}

	struct StubTransport(Peer);

	#[async_trait::async_trait]
	impl Transport for StubTransport {
		async fn request(
			&self,
			_req: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>, TransportError> {
			match self.0 {
				Peer::Healthy => Ok(Response::builder()
					.status(200)
					.body(br#"{"status":"ok"}"#.to_vec())
					.unwrap()),
				Peer::AStranger => Ok(Response::builder()
					.status(404)
					.body(Vec::new())
					.unwrap()),
				Peer::Absent => {
					Err(TransportError::Connect("connection refused".into()))
				}
			}
		}

		async fn open_ws(&self, _path: &str) -> Result<WsStream, TransportError> {
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

	/// Stands in for `CommandChild`, which cannot be constructed without an
	/// `AppHandle` and a running process. Nothing about `take_spawned` depends on
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
	/// that is already answering `/v0/health` rather than spawning one, and takes
	/// its early return before `spawn` — so no handle is recorded, and reaping must
	/// be a quiet no-op. A daemon this app did not start is not this app's to stop:
	/// killing by port or by process name would take down a developer's own
	/// `quiver daemon`, and a `reap` that unwrapped instead of returning would take
	/// down the app's quit with a panic.
	#[test]
	fn a_daemon_this_app_did_not_spawn_is_left_alone() {
		let manager = SidecarManager::new(LocalHost::Tcp(40257));

		manager.reap();

		assert!(
			manager.child
				.lock()
				.unwrap_or_else(|e| e.into_inner())
				.is_none(),
			"only `spawn` may fill this slot"
		);
	}

	// ── The daemon's own output ──────────────────────────────────────────────

	fn described(event: CommandEvent) -> (log::Level, String) {
		describe_event(&event).expect("this event must reach the log")
	}

	/// The daemon's stdout is where its startup lines come out. They used to go
	/// nowhere, because the receiver carrying them was dropped.
	#[test]
	fn stdout_from_the_daemon_reaches_the_log_at_info() {
		let (level, line) = described(CommandEvent::Stdout(
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
		let (level, line) = described(CommandEvent::Stderr(
			b"listen tcp 127.0.0.1:40257: bind: address already in use\n".to_vec(),
		));
		assert_eq!(level, log::Level::Warn);
		assert!(line.contains("address already in use"), "got {line:?}");
	}

	/// Not the daemon failing — the plugin failing to keep reading it. Distinct
	/// enough to be worth its own level: nothing more will arrive on this stream.
	#[test]
	fn a_stream_error_is_logged_as_an_error() {
		let (level, line) = described(CommandEvent::Error("stdout is not utf-8".into()));
		assert_eq!(level, log::Level::Error);
		assert!(line.contains("stdout is not utf-8"), "got {line:?}");
	}

	/// A daemon that has exited leaves the app holding a transport aimed at an
	/// address nothing answers on. The exit code is the whole of the diagnosis,
	/// so it has to be in the line.
	#[test]
	fn a_terminated_daemon_names_its_exit_code_and_signal() {
		let (level, line) = described(CommandEvent::Terminated(TerminatedPayload {
			code: Some(1),
			signal: None,
		}));
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
		let (tx, rx) = tauri::async_runtime::channel(4);
		tx.send(CommandEvent::Stdout(b"one\n".to_vec()))
			.await
			.expect("the pump's channel must accept events");
		tx.send(CommandEvent::Stderr(b"two\n".to_vec()))
			.await
			.expect("the pump's channel must accept events");
		// The daemon is gone and the plugin has closed the channel. Without the
		// sender dropped this call never returns, which is the point.
		drop(tx);

		tokio::time::timeout(Duration::from_secs(5), pump_events(rx))
			.await
			.expect("the pump must end when the daemon's stream closes");
	}
}
