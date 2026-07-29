use std::future::Future;
use std::sync::Arc;

use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use tokio::sync::{Mutex, RwLock};

use crate::connection::bridge::WsBridgeManager;
use crate::connection::local::LocalConnection;
use crate::connection::remote::RemoteConnection;
use crate::connection::types::{ConnectionConfig, QuiverConnection};

const KEYRING_SERVICE: &str = "quiver.desktop";
const STORE_KEY: &str = "connections";

pub struct ConnectionManager {
	active: RwLock<Box<dyn QuiverConnection>>,
	/// Serialises switches.
	///
	/// This used to be the write half of `active`: `switch_connection` took it
	/// before starting the new connection and released it after installing, so
	/// no two switches could interleave. That is exactly what wedged the app —
	/// `start()` dials a peer, a peer that accepts and then stalls holds the
	/// write lock indefinitely, and tokio's `RwLock` is fair, so every later
	/// READER (i.e. every `quiver://` request and every `ws_open`) queued behind
	/// the waiting writer and never came back.
	///
	/// So the serialisation moved here, to a lock no reader ever touches. A
	/// stalled `start()` now blocks nothing but other switches, which is the
	/// one thing it is supposed to block.
	switching: Mutex<()>,
}

impl Default for ConnectionManager {
	fn default() -> Self {
		Self::new()
	}
}

impl ConnectionManager {
	pub fn new() -> Self {
		Self {
			active: RwLock::new(Box::new(LocalConnection::new())),
			switching: Mutex::new(()),
		}
	}

	pub async fn start(&self, app: AppHandle) {
		// Takes `switching` even though it changes nothing: this is a long
		// await (the local sidecar's health wait runs to five seconds) taken
		// under the READ lock, and the invariant that keeps tokio's fair
		// `RwLock` harmless is that no writer is ever *waiting* on it while a
		// reader holds it. Every writer goes through `switching` first, so a
		// switch or a rename that lands mid-startup parks here instead of on
		// `active.write()`, where it would stall every request in the app.
		let _switching = self.switching.lock().await;
		self.active.read().await.start(&app).await;
	}

	pub async fn list_connections(&self, app: &AppHandle) -> Vec<ConnectionConfig> {
		let mut list = vec![self.active.read().await.config().clone()];
		list.extend(load_remote_configs(app).await);
		list
	}

	pub async fn get_connections(&self, app: &AppHandle) -> (Vec<ConnectionConfig>, String) {
		let active = self.active.read().await;
		let active_id = active.config().id.clone();
		let mut list = vec![active.config().clone()];
		drop(active);
		list.extend(load_remote_configs(app).await);
		(list, active_id)
	}

	pub async fn add_connection(
		&self,
		app: &AppHandle,
		name: String,
		url: String,
		token: String,
	) -> Result<ConnectionConfig, String> {
		let id = uuid::Uuid::new_v4().to_string();
		let conn = RemoteConnection::new(id.clone(), name, url, token.clone())
			.await
			.map_err(|e| e.to_string())?;
		let config = conn.config().clone();

		let entry = keyring::Entry::new(KEYRING_SERVICE, &id).map_err(|e| e.to_string())?;
		entry.set_password(&token).map_err(|e| e.to_string())?;

		let mut configs = load_remote_configs(app).await;
		configs.push(config.clone());
		save_remote_configs(app, &configs).await?;

		Ok(config)
	}

	pub async fn remove_connection(&self, app: &AppHandle, id: &str) -> Result<(), String> {
		if id == "local" {
			return Err("cannot remove local connection".into());
		}
		let mut configs = load_remote_configs(app).await;
		configs.retain(|c| c.id != id);
		save_remote_configs(app, &configs).await?;
		if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, id) {
			let _ = entry.delete_credential();
		}
		Ok(())
	}

	pub async fn switch_connection(&self, app: &AppHandle, id: &str) -> Result<(), String> {
		let bridge = app.state::<WsBridgeManager>();
		self.switch_to(&bridge, async move {
			// Build first, then start, and only then hand the connection over
			// to be installed: a switch that cannot be completed must leave the
			// current connection and its streams exactly as they were, and
			// `switch_to` never touches `active` until this future is Ok.
			let conn = build_connection(app, id).await?;
			conn.start(app).await;
			Ok(conn)
		})
		.await
	}

	/// The lock discipline of a switch, with its two Tauri-shaped halves lifted
	/// out into `prepare`: `build_connection` needs an `AppHandle` for the
	/// store and the keyring, and `QuiverConnection::start` needs one to emit
	/// status and to spawn the sidecar. Neither can be built under test — but
	/// which lock is held across what is the half that wedged the app, and all
	/// of that is here, and drivable.
	///
	/// Three properties, in the order they matter:
	///
	///   1. `switching` is held for the whole switch, so two switches cannot
	///      interleave. It replaces the write lock in that role — see the field.
	///   2. NO lock on `active` is held across `prepare`. It dials a peer;
	///      a peer that accepts and then stalls makes it take forever, and a
	///      write lock held across that blocks every subsequent reader too,
	///      because tokio's `RwLock` is fair.
	///   3. A failing `prepare` returns before `active` is touched at all, so
	///      the previous connection and its streams survive the failure intact.
	async fn switch_to<F>(&self, bridge: &WsBridgeManager, prepare: F) -> Result<(), String>
	where
		F: Future<Output = Result<Box<dyn QuiverConnection>, String>>,
	{
		let _switching = self.switching.lock().await;
		let new_conn = prepare.await?;
		let mut guard = self.active.write().await;
		retire_streams_and_teardown(bridge, &**guard).await;
		*guard = new_conn;
		Ok(())
	}

	pub async fn rename_connection(
		&self,
		app: &AppHandle,
		id: &str,
		name: String,
	) -> Result<(), String> {
		if id == "local" {
			return Err("cannot rename local connection".into());
		}
		let mut configs = load_remote_configs(app).await;
		match configs.iter_mut().find(|c| c.id == id) {
			Some(cfg) => cfg.name = name.clone(),
			None => return Err(format!("connection {id} not found")),
		}
		save_remote_configs(app, &configs).await?;
		// Keep active in-memory config in sync. `switching` first, like every
		// other writer of `active` — see `start`: a writer that waits on the
		// RwLock itself blocks every reader behind it.
		let _switching = self.switching.lock().await;
		let mut guard = self.active.write().await;
		if guard.config().id == id {
			guard.set_name(name);
		}
		Ok(())
	}

	pub async fn transport(&self) -> Arc<dyn crate::connection::transport::Transport> {
		self.active.read().await.transport()
	}

	/// Tear the active connection down because the app is exiting.
	///
	/// The one thing this reaps today is the local daemon: `lib.rs` spawns it and
	/// nothing else can kill it (see `SidecarManager::reap`), so without a call on
	/// the exit path every quit leaves a `quiver daemon` behind for the session.
	/// `lib.rs`'s `RunEvent` handler is the caller, and it can arrive twice —
	/// `ExitRequested` and `Exit` both fire on a normal quit — which is why
	/// everything underneath is idempotent.
	///
	/// The READ lock, not `switching` and not the write lock. Two reasons, both
	/// about not hanging a quit:
	///
	///   * `switching` is held for the whole of a switch, and a switch to a peer
	///     that accepts and then stalls holds it for `HEALTH_TIMEOUT`. Waiting on
	///     it here would make quitting mid-switch wait too, for no benefit: a
	///     switch in flight has not replaced `active` yet, so the connection this
	///     reads is the one whose daemon needs killing either way.
	///   * a reader is safe to take because no lock on `active` is ever held
	///     across an unbounded await — the invariant `switch_to` documents and
	///     `a_switch_that_never_finishes_preparing_still_leaves_the_app_readable`
	///     enforces. The write lock is only held across
	///     `retire_streams_and_teardown`, which now ends in a `kill()`, and that
	///     is bounded by the OS.
	pub async fn shutdown(&self) {
		self.active.read().await.teardown().await;
	}
}

/// The teardown half of a connection switch: retire every bridged stream, then
/// tear down the connection they were dialled over.
///
/// Every open stream belongs to the OUTGOING connection — its id addresses that
/// peer, and the frontend reopens against the new one once the switch is done —
/// so leaving them behind strands a socket apiece exactly the way a page reload
/// did before `on_page_load` retired them (see `connection::bridge`, teardown
/// paths (2) and (3)).
///
/// Streams first, connection second. That ordering has a subject now:
/// `local::LocalConnection::teardown` kills the daemon, and killing the daemon
/// takes down every socket the local connection's streams are riding on. A reader
/// that sees its socket die (rather than being cancelled) treats it as the daemon
/// ending the stream and announces `WS_CLOSE_SENTINEL`, which tells the JS shim
/// to reconnect. `close_all()` first is what cancels those readers instead.
///
/// Two things this comment will not overclaim, because neither is tested:
///
///   * the ordering is still not *demonstrated*. Reaching it needs a live daemon
///     whose death closes a real bridged socket, which the suite cannot stand up;
///     swapping these two lines leaves it green. What is tested is each half —
///     `a_switch_retires_the_outgoing_connections_streams` for the retirement,
///     and `sidecar::tests` for the reap.
///   * retiring first NARROWS the race rather than closing it. `close_all()` only
///     makes the reader's cancel channel ready; the kill immediately after makes
///     its socket ready too, and `bridge`'s `tokio::select!` is unbiased, so a
///     reader woken with both pending may still take the socket arm and announce.
///     Left alone deliberately: the consequence is one spurious reconnect, and
///     `ws_open` resolves the transport per open and queues on `active`'s read
///     lock behind this switch's writer — so it lands on the NEW connection,
///     which is where the frontend was going anyway. `biased;` in that `select!`
///     would make it exact, at the cost of a change to code this item is not
///     about.
///
/// It is a free function rather than two lines inline because
/// `switch_connection` needs an `AppHandle` from its first line onwards and so
/// cannot be driven under test; this can.
async fn retire_streams_and_teardown(bridge: &WsBridgeManager, outgoing: &dyn QuiverConnection) {
	bridge.close_all();
	outgoing.teardown().await;
}

async fn build_connection(app: &AppHandle, id: &str) -> Result<Box<dyn QuiverConnection>, String> {
	if id == "local" {
		return Ok(Box::new(LocalConnection::new()));
	}
	let configs = load_remote_configs(app).await;
	let cfg = configs
		.iter()
		.find(|c| c.id == id)
		.ok_or_else(|| format!("connection {id} not found"))?;
	let token = keyring::Entry::new(KEYRING_SERVICE, id)
		.and_then(|e| e.get_password())
		.map_err(|e| e.to_string())?;
	let conn = RemoteConnection::new(
		cfg.id.clone(),
		cfg.name.clone(),
		cfg.url.clone().unwrap_or_default(),
		token,
	)
	.await
	.map_err(|e| e.to_string())?;
	Ok(Box::new(conn))
}

// ── Persistence helpers ───────────────────────────────────────────────────────

async fn load_remote_configs(app: &AppHandle) -> Vec<ConnectionConfig> {
	let store = match app.store("connections.json") {
		Ok(s) => s,
		Err(_) => return vec![],
	};
	let value = match store.get(STORE_KEY) {
		Some(v) => v,
		None => return vec![],
	};
	serde_json::from_value(value).unwrap_or_default()
}

async fn save_remote_configs(app: &AppHandle, configs: &[ConnectionConfig]) -> Result<(), String> {
	let store = app.store("connections.json").map_err(|e| e.to_string())?;
	let value = serde_json::to_value(configs).map_err(|e| e.to_string())?;
	store.set(STORE_KEY, value);
	store.save().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::bridge::{open_bridge, FrameSink};
	use crate::connection::transport::http::HttpTransport;
	use crate::connection::transport::Transport;
	use std::sync::atomic::{AtomicBool, Ordering};
	use std::sync::Mutex;
	use std::time::Duration;
	use tokio::net::TcpListener;

	/// Stands in for a connection either side of a switch. The one thing a real
	/// one does that no test can reach is `start` — it needs an `AppHandle` —
	/// and `switch_to` takes that half as a future precisely so it does not have
	/// to be reached here.
	struct StubConnection {
		config: ConnectionConfig,
		torn_down: Arc<AtomicBool>,
		transport: Arc<dyn Transport>,
	}

	impl StubConnection {
		/// `id` is what a test reads back to tell which connection is active.
		/// The transport is real but points nowhere: `switch_to` never dials,
		/// and the tests below only need `transport()` to RESOLVE — that it
		/// resolves at all, while a switch is mid-flight, is the property
		/// finding B is about.
		fn new(id: &str, torn_down: Arc<AtomicBool>) -> Self {
			Self {
				config: ConnectionConfig {
					id: id.into(),
					name: id.into(),
					kind: "local".into(),
					url: None,
					api_version: "v0".into(),
				},
				torn_down,
				transport: Arc::new(HttpTransport::new("http://127.0.0.1:1", None)),
			}
		}

		fn boxed(id: &str, torn_down: Arc<AtomicBool>) -> Box<dyn QuiverConnection> {
			Box::new(Self::new(id, torn_down))
		}
	}

	#[async_trait::async_trait]
	impl QuiverConnection for StubConnection {
		async fn start(&self, _app: &AppHandle) {
			unreachable!(
				"a switch's start half needs a real AppHandle; not under test here"
			)
		}

		async fn teardown(&self) {
			self.torn_down.store(true, Ordering::Relaxed);
		}

		fn transport(&self) -> Arc<dyn Transport> {
			Arc::clone(&self.transport)
		}

		fn config(&self) -> &ConnectionConfig {
			&self.config
		}

		fn set_name(&mut self, name: String) {
			self.config.name = name;
		}
	}

	/// A manager whose active connection is a stub, so the lock discipline can
	/// be driven without a daemon or an `AppHandle`. `ConnectionManager::new()`
	/// would install a real `LocalConnection` instead.
	fn manager_with(active: Box<dyn QuiverConnection>) -> ConnectionManager {
		ConnectionManager {
			active: RwLock::new(active),
			// Spelt out: this module's tests shadow `Mutex` with `std`'s, which
			// `Recorder` below needs.
			switching: tokio::sync::Mutex::new(()),
		}
	}

	async fn active_id(m: &ConnectionManager) -> String {
		m.active.read().await.config().id.clone()
	}

	/// Records the frames a stream announced, so the test can assert on what the
	/// frontend was NOT told.
	struct Recorder(Arc<Mutex<Vec<String>>>);

	impl FrameSink for Recorder {
		fn send(&self, frame: String) {
			self.0.lock().unwrap().push(frame)
		}
	}

	/// A switch leaves every stream of the outgoing connection behind unless it
	/// retires them: their ids address a peer the app has just left, and their
	/// JS will never call `ws_close` for them because it reopens fresh ids
	/// against the new connection instead. Nothing else can end them — the peer
	/// here never reacts, which is exactly what a wedged or remote daemon looks
	/// like — so the reader finishing IS the proof that the switch retired it,
	/// and with it that the socket came back.
	#[tokio::test]
	async fn a_switch_retires_the_outgoing_connections_streams() {
		let _serialised = crate::FD_TESTS.lock().await;

		let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
		let addr = listener.local_addr().expect("addr");
		tokio::spawn(async move {
			if let Ok((stream, _)) = listener.accept().await {
				if let Ok(_ws) = tokio_tungstenite::accept_async(stream).await {
					// Upgraded, then never polled again.
					std::future::pending::<()>().await;
				}
			}
		});
		let transport = HttpTransport::new(format!("http://{addr}"), None);

		let bridge = WsBridgeManager::new();
		let announced = Arc::new(Mutex::new(Vec::new()));
		let reader = open_bridge(
			&transport,
			"c1".to_string(),
			"/v0/x".to_string(),
			Recorder(Arc::clone(&announced)),
			&bridge,
		)
		.await
		.expect("the stream must open");

		let torn_down = Arc::new(AtomicBool::new(false));
		let outgoing = StubConnection::new("local", Arc::clone(&torn_down));

		retire_streams_and_teardown(&bridge, &outgoing).await;

		// A timeout rather than a plain await: without the retirement this hangs
		// forever, and a hung suite is a worse signal than a failed assertion.
		tokio::time::timeout(Duration::from_secs(5), reader)
			.await
			.expect(
				"a switch must end the outgoing connection's readers: a stream left open \
				 holds a socket to a peer the app has switched away from, for the life of \
				 the process",
			)
			.expect("the reader task must not panic");

		assert!(
			torn_down.load(Ordering::Relaxed),
			"the outgoing connection must still be torn down"
		);
		// Cloned out of the mutex before asserting: `assert!` evaluates its format
		// arguments while the condition's temporary guard is still alive, and
		// locking a std `Mutex` twice on one thread deadlocks.
		let announced = announced.lock().unwrap().clone();
		assert!(
			announced.is_empty(),
			"a switch must retire streams silently: the close sentinel is what makes the \
			 shim reconnect, and reconnecting mid-switch dials the peer being left. \
			 Got {announced:?}"
		);
	}

	// ── The lock discipline of a switch (finding B) ──────────────────────────

	/// The whole of finding B, stated as a test.
	///
	/// A switch's `prepare` step dials the new peer, and a peer that accepts a
	/// connection and then stalls makes it take forever. It used to run under
	/// `active`'s WRITE lock — so tokio's fair `RwLock` parked every subsequent
	/// reader behind the waiting writer, and every `quiver://` request and every
	/// `ws_open` in the app hung, past every timeout, until the app was killed.
	///
	/// Here `prepare` never resolves, and the manager must stay readable
	/// throughout. Move the `active.write()` in `switch_to` back above
	/// `prepare.await` and this stops finishing.
	#[tokio::test]
	async fn a_switch_that_never_finishes_preparing_still_leaves_the_app_readable() {
		let bridge = Arc::new(WsBridgeManager::new());
		let manager = Arc::new(manager_with(StubConnection::boxed(
			"local",
			Arc::new(AtomicBool::new(false)),
		)));

		let switching = tokio::spawn({
			let manager = Arc::clone(&manager);
			let bridge = Arc::clone(&bridge);
			async move {
				manager.switch_to(&bridge, async {
					std::future::pending::<()>().await;
					unreachable!("a stalled peer never answers")
				})
				.await
			}
		});

		// Give the switch a chance to reach (and park in) its prepare step
		// before reading, so the read really does race a switch in flight.
		tokio::task::yield_now().await;

		tokio::time::timeout(Duration::from_secs(5), manager.transport())
			.await
			.expect(
				"a switch that is still dialling must not hold any lock on the active \
				 connection: every quiver:// request in the app takes that read lock",
			);

		switching.abort();
	}

	/// A switch that cannot be completed must leave the previous connection —
	/// and its streams — exactly as they were. `switch_to` must not touch
	/// `active` until `prepare` has succeeded, which is why the build and start
	/// halves live inside `prepare` rather than around it.
	#[tokio::test]
	async fn a_switch_whose_preparation_fails_changes_nothing() {
		let _serialised = crate::FD_TESTS.lock().await;

		let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
		let addr = listener.local_addr().expect("addr");
		tokio::spawn(async move {
			if let Ok((stream, _)) = listener.accept().await {
				if let Ok(_ws) = tokio_tungstenite::accept_async(stream).await {
					std::future::pending::<()>().await;
				}
			}
		});

		let bridge = WsBridgeManager::new();
		let reader = open_bridge(
			&HttpTransport::new(format!("http://{addr}"), None),
			"c1".to_string(),
			"/v0/x".to_string(),
			Recorder(Arc::new(Mutex::new(Vec::new()))),
			&bridge,
		)
		.await
		.expect("the stream must open");

		let torn_down = Arc::new(AtomicBool::new(false));
		let manager = manager_with(StubConnection::boxed("local", Arc::clone(&torn_down)));

		let err = manager
			.switch_to(&bridge, async { Err("no such connection".to_string()) })
			.await
			.expect_err("a failed preparation must fail the switch");
		assert_eq!(err, "no such connection");

		assert_eq!(
			active_id(&manager).await,
			"local",
			"a failed switch must leave the previous connection active"
		);
		assert!(
			!torn_down.load(Ordering::Relaxed),
			"a failed switch must not tear down the connection it failed to replace"
		);
		assert!(
			!reader.is_finished(),
			"a failed switch must leave the previous connection's streams open: the \
			 frontend is still talking to that peer"
		);

		bridge.close_all();
		let _ = tokio::time::timeout(Duration::from_secs(5), reader).await;
	}

	/// A successful switch installs the new connection and retires the outgoing
	/// one — the ordering `retire_streams_and_teardown` documents, now reached
	/// through the path production actually takes.
	#[tokio::test]
	async fn a_successful_switch_installs_the_new_connection_and_retires_the_old() {
		let torn_down = Arc::new(AtomicBool::new(false));
		let manager = manager_with(StubConnection::boxed("local", Arc::clone(&torn_down)));
		let bridge = WsBridgeManager::new();

		manager.switch_to(&bridge, async {
			Ok(StubConnection::boxed(
				"remote-1",
				Arc::new(AtomicBool::new(false)),
			))
		})
		.await
		.expect("the switch must succeed");

		assert_eq!(active_id(&manager).await, "remote-1");
		assert!(
			torn_down.load(Ordering::Relaxed),
			"the outgoing connection must be torn down"
		);
	}

	// ── The exit path ────────────────────────────────────────────────────────

	/// Quitting the app must reap what the app started. `lib.rs`'s `RunEvent`
	/// handler has nothing to reach the active connection with but this, and
	/// before it existed the local daemon simply outlived every session.
	///
	/// It runs twice on the normal path — `ExitRequested` then `Exit` — which is
	/// safe because everything it reaches is idempotent; the handle only comes out
	/// of its slot once (`sidecar::tests::a_spawned_child_is_taken_exactly_once`).
	#[tokio::test]
	async fn shutdown_tears_down_the_active_connection() {
		let torn_down = Arc::new(AtomicBool::new(false));
		let manager = manager_with(StubConnection::boxed("local", Arc::clone(&torn_down)));

		manager.shutdown().await;

		assert!(
			torn_down.load(Ordering::Relaxed),
			"quitting must tear the active connection down: nothing else can kill \
			 the daemon this app spawned"
		);
	}

	/// Quitting while a switch is still dialling must not hang the app. The switch
	/// holds `switching` for its whole duration and a stalled peer holds it for
	/// the health timeout — so `shutdown` deliberately does not take it, and takes
	/// a read lock on a connection no writer can be holding.
	///
	/// Make `shutdown` wait on `switching` (or on `active.write()`) and this stops
	/// finishing: the quit would sit behind a peer that never answers.
	#[tokio::test]
	async fn shutdown_during_a_stalled_switch_still_reaps() {
		let bridge = Arc::new(WsBridgeManager::new());
		let torn_down = Arc::new(AtomicBool::new(false));
		let manager = Arc::new(manager_with(StubConnection::boxed(
			"local",
			Arc::clone(&torn_down),
		)));

		let switching = tokio::spawn({
			let manager = Arc::clone(&manager);
			let bridge = Arc::clone(&bridge);
			async move {
				manager.switch_to(&bridge, async {
					std::future::pending::<()>().await;
					unreachable!("a stalled peer never answers")
				})
				.await
			}
		});

		// Let the switch reach its prepare step, so the quit really does race a
		// switch in flight.
		tokio::task::yield_now().await;

		tokio::time::timeout(Duration::from_secs(5), manager.shutdown())
			.await
			.expect("a quit must not wait on a switch that is still dialling");
		assert!(
			torn_down.load(Ordering::Relaxed),
			"and it must still reap the connection that is active"
		);

		switching.abort();
	}

	/// `switching` exists only to keep the serialisation the write lock used to
	/// provide, now that the write lock is no longer held across the dialling
	/// half of a switch. Two switches that interleave would install in an order
	/// unrelated to the order they were asked for, and could retire the streams
	/// of a connection the other has already replaced.
	///
	/// Each `prepare` records when it entered and left; overlapping intervals
	/// are the defect. Drop the `switching` lock from `switch_to` and the two
	/// yields below let them interleave, which turns this red.
	#[tokio::test]
	async fn two_switches_do_not_interleave() {
		let bridge = Arc::new(WsBridgeManager::new());
		let manager = Arc::new(manager_with(StubConnection::boxed(
			"local",
			Arc::new(AtomicBool::new(false)),
		)));
		let log = Arc::new(Mutex::new(Vec::<String>::new()));

		let switch = |id: &'static str| {
			let manager = Arc::clone(&manager);
			let bridge = Arc::clone(&bridge);
			let log = Arc::clone(&log);
			async move {
				manager.switch_to(&bridge, async move {
					log.lock().unwrap().push(format!("enter {id}"));
					// Two yields: ample opportunity for the other
					// switch's prepare to slip in, if anything let it.
					tokio::task::yield_now().await;
					tokio::task::yield_now().await;
					log.lock().unwrap().push(format!("leave {id}"));
					Ok(StubConnection::boxed(
						id,
						Arc::new(AtomicBool::new(false)),
					))
				})
				.await
			}
		};

		let (a, b) = tokio::join!(switch("a"), switch("b"));
		a.expect("switch a must succeed");
		b.expect("switch b must succeed");

		let log = log.lock().unwrap().clone();
		assert!(
			log == ["enter a", "leave a", "enter b", "leave b"]
				|| log == ["enter b", "leave b", "enter a", "leave a"],
			"switches must not interleave; got {log:?}"
		);
	}
}
