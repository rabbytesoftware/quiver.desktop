//! Generic WebSocket bridge over whatever connection is active.
//!
//! The webview cannot open a `WebSocket` to quiver.core: its only reachable
//! endpoint is the `quiver://` URI-scheme proxy (see `proxy`), and the browser
//! `WebSocket` constructor rejects every scheme but `ws`/`wss`. So Rust is the
//! WebSocket client here — it dials the active connection's own transport (a
//! unix socket locally, TCP/TLS remotely; a WS upgrade is perfectly legal on
//! either) and bridges an arbitrary `/v0/...` WebSocket route both ways to the
//! webview.
//!
//!   * daemon → webview: each `Message::Text(text)` is pushed verbatim down a
//!     Tauri `Channel<String>` the frontend supplies at open time.
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
//! Note what this deliberately does *not* rely on: a dead `Channel`. `Channel::send`
//! bottoms out in `webview.eval()`, which succeeds for as long as the *webview* is
//! alive — a reloaded page is the same webview. It therefore cannot detect a reload,
//! only app shutdown, and a reader waiting to be told its page is gone would wait
//! forever. Cancellation has to be explicit; case (3) is why.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use tauri::ipc::Channel;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

use crate::connection::transport::Transport;

/// Sentinel pushed down the message Channel when the daemon closes the stream
/// (close frame, error, or EOF). The JS shim recognises it and fires `onclose`
/// so `wsManager` reconnects and the §6 cache re-seeds. A NUL prefix makes it
/// impossible to collide with a real JSON DTO frame (which always starts `{`).
pub const WS_CLOSE_SENTINEL: &str = "\u{0}quiver-ws-close";

/// Where daemon → webview frames go: a Tauri `Channel` in the app, a plain closure
/// under test. The indirection is what lets a connection's lifetime be exercised
/// without standing up a webview.
pub trait FrameSink: Send + Sync + 'static {
	fn send(&self, frame: String);
}

impl FrameSink for Channel<String> {
	fn send(&self, frame: String) {
		// Errors here mean the webview is gone, i.e. the app is shutting down. They
		// say nothing about the connection (in particular, a page reload does NOT
		// produce one), so there is nothing useful to do but drop the frame.
		let _ = Channel::send(self, frame);
	}
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
		self.connections
			.lock()
			.unwrap()
			.get(conn_id)
			.map(|c| c.tx.clone())
	}

	/// Enqueue a raw text frame (the JSON the frontend wants to publish) for a
	/// connection's writer task. The whole of `ws_send`, minus Tauri's `State`.
	pub fn send(&self, conn_id: &str, data: String) -> Result<(), String> {
		match self.sender(conn_id) {
			Some(tx) => tx
				.send(Message::Text(data))
				.map_err(|_| format!("ws connection {conn_id} is closed")),
			None => Err(format!("no open ws connection {conn_id}")),
		}
	}

	/// Retire one connection — teardown path (2), the frontend closing it. Removing
	/// the entry from the map is the whole of the work; see the module doc.
	pub fn close(&self, conn_id: &str) {
		self.connections.lock().unwrap().remove(conn_id);
	}

	/// Retires every connection. Called when a page load orphans them all: the JS
	/// that owned these ids is gone and will never call `ws_close` for them, and the
	/// new page opens fresh ids of its own.
	pub fn close_all(&self) {
		self.connections.lock().unwrap().clear();
	}
}

/// Open a WebSocket over the active connection's transport for `path` (a full
/// `/v0/...` route) and start streaming its frames to `on_message`. Each text frame
/// is forwarded RAW — the whole DTO object — because the frontend parses the
/// complete frame itself.
///
/// This is the transport half of `ws_open`, free of Tauri's `State` so it can be
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
) -> Result<tokio::task::JoinHandle<()>, String> {
	// The transport owns the dial and the upgrade: which host, which scheme and which
	// credentials those need is exactly what differs between a local unix socket and
	// a remote https peer, and the bridge is deliberately ignorant of all of it.
	let ws = transport.open_ws(&path).await.map_err(|e| {
		log::error!("ws_open: opening {path} failed: {e}");
		format!("open ws {path}: {e}")
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
	manager.connections.lock().unwrap().insert(
		conn_id.clone(),
		Connection {
			generation,
			tx,
			_cancel: cancel,
		},
	);

	// Reader task: forward each text frame verbatim to the webview channel until the
	// daemon closes the stream or this connection is retired out from under us.
	//
	// (Crowbar spells this `loop` + `break`; the two are equivalent, but tarpaulin
	// instruments the `loop` keyword and the arm that breaks out of it with counters
	// that never fire even when the arm's own body provably does, leaving two lines
	// of this file permanently and falsely uncovered. See the task report.)
	let connections = Arc::clone(&manager.connections);
	let reader = tokio::spawn(async move {
		let mut daemon_closed = false;
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
			let mut conns = connections.lock().unwrap();
			if conns.get(&conn_id)
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
			on_message.send(WS_CLOSE_SENTINEL.to_string());
		}
	});

	Ok(reader)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::connection::transport::http::HttpTransport;
	use std::time::Duration;
	use tokio::net::TcpListener;

	// Every test below that opens a socket takes `crate::FD_TESTS`, the
	// crate-wide guard — not just the one that counts descriptors. `open_fds()`
	// is process-wide, so a listener held by ANY concurrent test lands in the
	// count; see the guard's own doc for what that cost when it was scoped to
	// this module alone.

	/// File descriptors this process holds open (`/dev/fd` on macOS, a symlink to
	/// `/proc/self/fd` on Linux). Process-wide, and therefore only meaningful while
	/// `FD_TESTS` is held — see the note there.
	fn open_fds() -> usize {
		std::fs::read_dir("/dev/fd").map(|d| d.count()).unwrap_or(0)
	}

	/// How much a run of descriptor counts rose for reasons other than its own
	/// single biggest jump.
	///
	/// No single count is trustworthy, even with `crate::FD_TESTS` held: every
	/// `#[tokio::test]` in this binary holds an IO driver (a kqueue and a wakeup
	/// pipe) for as long as it runs, and the guard makes the OTHER socket tests
	/// sit there blocked — holding theirs — for as long as the counting one runs.
	/// Measured at `RUST_TEST_THREADS=24`: a single run reads 103 descriptors
	/// throughout and dips to 99 once or twice as unrelated tests come and go.
	///
	/// So the question has to be "did the count grow", never "how high is it".
	/// The shape of the growth is what answers it. A leak is PER CYCLE: it rises
	/// a little, again and again, for as long as the loop runs. Those blocked
	/// suite-mates are an ARRIVAL: they take their descriptors once and then sit
	/// there, so the count steps up and goes flat, however far it stepped.
	///
	/// Summing the rises and discarding the largest keeps the first and drops
	/// the second. Comparing two halves of the run cannot: it only asks where
	/// the level ended up, so one arrival that outlives the halfway point reads
	/// exactly like a leak. That is not theoretical — see `DRIFT`.
	/// Rises are measured against the running MAXIMUM, not against the previous
	/// sample. A leak only ever shows up as the count reaching levels it has
	/// not reached before, so that is the thing worth measuring — and it is the
	/// only reading that survives a transient DIP.
	///
	/// Consecutive-sample deltas do not. CI produced
	/// `[.. 25, 25, 21, 24, 24, 25, 25 ..]` on this branch, which touches no
	/// Rust: descriptors momentarily released and immediately reclaimed. Net
	/// change zero, peak never above the 26 seen earlier — yet the recovery
	/// alone counted +3 and +1, because falls were never credited against the
	/// rises they precede. Two runs failed that way, on identical code, while
	/// macOS, Windows and tarpaulin passed.
	///
	/// Against the running maximum the same dip contributes nothing, and both
	/// shapes the threshold was calibrated on are unchanged — see
	/// `an_arrival_is_not_a_leak_but_a_slow_leak_is`, whose expected values
	/// still hold exactly: an arrival is one jump to a new plateau, and a leak
	/// keeps setting new maxima all run long.
	fn growth_beyond_one_arrival(counts: &[usize]) -> usize {
		let mut peak = match counts.first() {
			Some(&first) => first,
			None => return 0,
		};
		let mut rises: Vec<usize> = Vec::new();
		for &count in &counts[1..] {
			if count > peak {
				rises.push(count - peak);
				peak = count;
			}
		}
		rises.sort_unstable();
		rises.pop();
		rises.iter().sum()
	}

	/// The statistic itself, against the two shapes it has to tell apart. The
	/// arrival is the run CI actually produced on Rust this branch never
	/// touched; the leak is the rate the threshold was calibrated for.
	#[test]
	fn an_arrival_is_not_a_leak_but_a_slow_leak_is() {
		let mut arrival = vec![21_usize; 93];
		arrival.extend(std::iter::repeat_n(25, 6));
		arrival.extend(std::iter::repeat_n(26, 101));
		assert_eq!(arrival.len(), 200);
		assert_eq!(
			growth_beyond_one_arrival(&arrival),
			1,
			"five descriptors taken once and held is one jump, not a leak"
		);

		// The shape CI actually produced, twice, on a branch that touches no Rust:
		// a transient dip and its recovery, peaking no higher than it already had.
		// Measured against consecutive samples this scored 3 and failed the run.
		let mut dip = vec![25_usize; 36];
		dip.extend(std::iter::repeat_n(26, 38));
		dip.extend(std::iter::repeat_n(25, 9));
		dip.push(21);
		dip.extend([24, 24]);
		dip.extend(std::iter::repeat_n(25, 114));
		assert_eq!(dip.len(), 200);
		assert_eq!(
			growth_beyond_one_arrival(&dip),
			0,
			"reclaiming descriptors it had already released is not growth"
		);

		// One descriptor every 37 closes — the weakest leak this test claims to see.
		let leak: Vec<usize> = (0..200).map(|i| 21 + i / 37).collect();
		assert_eq!(
			growth_beyond_one_arrival(&leak),
			4,
			"a leak spreads its rises, so discarding the largest barely dents it"
		);
	}

	/// A transport pointed at a live loopback listener. TCP rather than a unix
	/// socket so these tests run identically on every platform: the bridge only
	/// ever sees the `WsStream` a transport hands back, so which kind of socket
	/// carries it is not what is under test here.
	async fn listening() -> (TcpListener, HttpTransport) {
		let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
		let addr = listener.local_addr().expect("addr");
		(listener, HttpTransport::new(format!("http://{addr}"), None))
	}

	struct ClosureSink<F>(F);

	impl<F: Fn(String) + Send + Sync + 'static> FrameSink for ClosureSink<F> {
		fn send(&self, frame: String) {
			(self.0)(frame)
		}
	}

	/// A sink that fires `tx` when the close sentinel arrives, so a test can block on
	/// the connection actually being torn down rather than on a clock.
	fn sentinel_sink(tx: oneshot::Sender<()>) -> impl FrameSink {
		let tx = Mutex::new(Some(tx));
		ClosureSink(move |frame: String| {
			if frame == WS_CLOSE_SENTINEL {
				if let Some(tx) = tx.lock().unwrap().take() {
					let _ = tx.send(());
				}
			}
		})
	}

	fn silent_sink() -> impl FrameSink {
		ClosureSink(|_| {})
	}

	/// Records every frame it is handed, so a test can assert on what did NOT
	/// reach the shim as well as on what did.
	fn recording_sink(seen: &Arc<Mutex<Vec<String>>>) -> impl FrameSink {
		let seen = Arc::clone(seen);
		ClosureSink(move |frame: String| seen.lock().unwrap().push(frame))
	}

	/// The daemon closing a stream on its own initiative — what it does on every
	/// watcher release and workspace switch.
	async fn accept_then_close(listener: &TcpListener) {
		if let Ok((stream, _)) = listener.accept().await {
			if let Ok(ws) = tokio_tungstenite::accept_async(stream).await {
				drop(ws);
			}
		}
	}

	/// A wedged daemon: it completes the upgrade and then never touches the socket again
	/// — never reads it, never answers, never closes it.
	///
	/// Getting this peer right is the whole point of the two tests below. Retiring a
	/// connection ends the writer, which half-closes our socket (`SHUT_WR`) — and a
	/// *healthy* daemon reacts to that EOF by closing its end, which wakes our reader all
	/// by itself. So a fake daemon that reads its socket cannot tell a working teardown
	/// from a broken one: both look identical. This one never reads, so nothing but our
	/// own cancellation can free the descriptor, which is precisely the case cancellation
	/// exists for — and the case a wedged daemon actually presents.
	fn spawn_wedged_daemon(listener: TcpListener) {
		tokio::spawn(async move {
			if let Ok((stream, _)) = listener.accept().await {
				if let Ok(_ws) = tokio_tungstenite::accept_async(stream).await {
					// Hold the connection open forever, without ever polling it.
					std::future::pending::<()>().await;
				}
			}
		});
	}

	#[tokio::test]
	async fn daemon_initiated_close_retires_the_connection() {
		let _serialised = crate::FD_TESTS.lock().await;

		let (listener, transport) = listening().await;
		let manager = WsBridgeManager::new();

		let (tx, rx) = oneshot::channel();
		let opened = open_bridge(
			&transport,
			"c1".to_string(),
			"/v0/x".to_string(),
			sentinel_sink(tx),
			&manager,
		);
		let (opened, _) = tokio::join!(opened, accept_then_close(&listener));
		opened.unwrap();

		rx.await.expect("the shim must be told the daemon closed");

		assert!(
			manager.connections.lock().unwrap().is_empty(),
			"the daemon closed the stream, so the connection must be retired; a connection \
			 left in the map strands its writer task on the socket's write half and burns \
			 one of the app's descriptors for good"
		);
	}

	/// The production bug, as a regression test: 215 of these closes in one window is what
	/// a pre-crash daemon log shows, against an app whose descriptor limit was 256.
	///
	/// This is the one test that has to count descriptors. A stranded writer still lets its
	/// reader finish, so nothing local to the connection reveals it — only the process's
	/// descriptor table does.
	#[tokio::test]
	async fn daemon_initiated_closes_do_not_leak_file_descriptors() {
		let _serialised = crate::FD_TESTS.lock().await;

		/// Enough closes that a leak on a FRACTION of them still separates the
		/// two halves, which is the lever that sets how small a leak this test
		/// can see: for one descriptor every `N`th close the halves' medians
		/// differ by `CYCLES / 2N`, so `CYCLES` buys sensitivity directly.
		/// Measured cost: the test runs in 60-90ms here against 10-20ms at 20
		/// cycles, and the whole suite goes from ~0.05s to ~0.10s.
		const CYCLES: usize = 200;

		/// How much the count may rise, on top of its single largest jump,
		/// without being called a leak.
		///
		/// This once compared the medians of the run's two halves, and the
		/// margin was set on the reasoning that the only sustained shift — the
		/// other socket tests piling up on `crate::FD_TESTS` — saturates within
		/// the first cycles, i.e. inside the first half, where it cancels out.
		///
		/// That is an assumption about how fast the SUITE runs, and under
		/// `cargo tarpaulin` it is false: ptrace slows everything enough that
		/// the pile-up lands mid-run instead. CI caught it twice on Rust this
		/// branch never touched — five descriptors appearing at cycle 66 in one
		/// run and cycle 93 in the next, then FLAT for the remaining hundred
		/// cycles. Both moved the late median by the whole five. Every plain
		/// `cargo test` job on the same commit passed.
		///
		/// Discarding the largest jump costs nothing against a real leak, which
		/// spreads its rises across the run: at one descriptor every 37 closes
		/// there are five separate rises of one, and dropping a single one of
		/// them still leaves four. So 2 stays what it was — margin for a slower
		/// or busier machine — and the sensitivity it was measured against (one
		/// in 37 closes, 20/20 runs; one in 38 escapes, 0/15) is unchanged.
		/// The setting they replace (20 cycles, drift 6) needed SEVEN leaks per
		/// TEN closes: a bridge leaking on half of the 215 closes in this test's
		/// own header would have stranded 107 descriptors and passed.
		const DRIFT: usize = 2;

		let (listener, transport) = listening().await;
		let manager = WsBridgeManager::new();
		let mut counts = Vec::with_capacity(CYCLES);

		// Every cycle is one reconnect: the shim opens a fresh conn_id, the daemon closes it.
		for i in 0..CYCLES {
			let (tx, rx) = oneshot::channel();
			let opened = open_bridge(
				&transport,
				format!("c{i}"),
				"/v0/x".to_string(),
				sentinel_sink(tx),
				&manager,
			);
			let (opened, _) = tokio::join!(opened, accept_then_close(&listener));
			opened.unwrap();
			rx.await.unwrap();

			// Sampled after the connection is fully retired — the sentinel is sent only
			// once both split halves are gone — so a leak-free cycle returns to the level
			// the previous one started from, and a leaking one does not.
			counts.push(open_fds());
		}

		let growth = growth_beyond_one_arrival(&counts);
		assert!(
			growth <= DRIFT,
			"{CYCLES} daemon-initiated closes leaked file descriptors: the count rose by \
			 {growth} beyond its largest single jump ({counts:?}). Once the app's limit is \
			 reached quiver.core cannot be dialled at all — which the health watchdog reads \
			 as a dead backend and kills a daemon that was never unhealthy."
		);
	}

	/// The reader parks on `read.next()`. Against a daemon that has stopped reading, the
	/// half-close that retiring performs is never noticed and never answered, so nothing but
	/// explicit cancellation can end the reader — and this test hangs.
	///
	/// The reader finishing IS the assertion: it awaits the writer, so by the time it returns
	/// both split halves are dropped and the socket is closed by ownership. Counting
	/// descriptors here would add nothing and would only make the test race every other test
	/// that opens one.
	#[tokio::test]
	async fn explicit_close_releases_the_socket_against_a_daemon_that_never_reacts() {
		let _serialised = crate::FD_TESTS.lock().await;

		let (listener, transport) = listening().await;
		spawn_wedged_daemon(listener);
		let manager = WsBridgeManager::new();

		let reader = open_bridge(
			&transport,
			"c1".to_string(),
			"/v0/x".to_string(),
			silent_sink(),
			&manager,
		)
		.await
		.unwrap();

		// Exactly what `ws_close` does, and the only thing it does.
		manager.close("c1");

		reader.await.expect(
			"retiring a connection must end its reader, whatever the daemon does",
		);
	}

	/// A page load orphans every connection the old page owned: its JS is gone and will never
	/// call `ws_close` for ids it no longer remembers. Nothing else can notice — a `Channel`
	/// keeps working across a reload — so `close_all` is all that stands between a reload and
	/// a permanently stranded socket.
	#[tokio::test]
	async fn close_all_retires_connections_a_reloaded_page_abandoned() {
		let _serialised = crate::FD_TESTS.lock().await;

		let (listener, transport) = listening().await;
		spawn_wedged_daemon(listener);
		let manager = WsBridgeManager::new();

		let seen = Arc::new(Mutex::new(Vec::new()));
		let reader = open_bridge(
			&transport,
			"c1".to_string(),
			"/v0/x".to_string(),
			recording_sink(&seen),
			&manager,
		)
		.await
		.unwrap();

		manager.close_all();

		reader.await.expect("a page load must end its readers");

		assert!(manager.connections.lock().unwrap().is_empty());

		// The reader has finished, so anything it was going to announce, it has.
		let announced = seen.lock().unwrap().clone();
		assert!(
			announced.is_empty(),
			"a page load must not announce a close: the sentinel is what makes the \
			 shim reconnect, and the ids it would reconnect belong to a page that no \
			 longer exists. Got {announced:?}"
		);
	}

	// ── Beyond Crowbar's four lifetime tests ──────────────────────────────
	// The four above are the descriptor-lifetime regression suite and say
	// nothing about what the bridge carries, because Crowbar's peers close or
	// wedge before a single frame moves. The rest of this module covers what
	// the ported code actually does with a live stream — and each of these
	// paths is reached in the app only through `lib.rs`'s command wrappers,
	// which are coverage-excluded, so it has to be asserted from here.

	/// The bridge's actual job. Frames are forwarded RAW because the frontend
	/// parses the whole DTO itself, so a sink that sees anything but the exact
	/// bytes the daemon sent is a broken bridge. A non-text frame is not a DTO:
	/// it must be skipped, and must NOT be mistaken for the stream ending.
	#[tokio::test]
	async fn text_frames_reach_the_sink_verbatim_and_other_frames_are_skipped() {
		let _serialised = crate::FD_TESTS.lock().await;

		const FRAME: &str = r#"{"id":"a","name":"arrow"}"#;

		let (listener, transport) = listening().await;
		let manager = WsBridgeManager::new();

		let seen = Arc::new(Mutex::new(Vec::new()));
		let recorded = Arc::clone(&seen);
		let (closed_tx, closed_rx) = oneshot::channel();
		let closed_tx = Mutex::new(Some(closed_tx));
		let sink = ClosureSink(move |frame: String| {
			recorded.lock().unwrap().push(frame.clone());
			if frame == WS_CLOSE_SENTINEL {
				if let Some(tx) = closed_tx.lock().unwrap().take() {
					let _ = tx.send(());
				}
			}
		});

		let peer = async {
			let (stream, _) = listener.accept().await.expect("accept");
			let mut ws = tokio_tungstenite::accept_async(stream)
				.await
				.expect("upgrade");
			ws.send(Message::Binary(vec![0xde, 0xad]))
				.await
				.expect("send binary");
			ws.send(Message::Text(FRAME.to_string()))
				.await
				.expect("send text");
			ws.close(None).await.expect("close");
		};

		let opened = open_bridge(
			&transport,
			"c1".to_string(),
			"/v0/x".to_string(),
			sink,
			&manager,
		);
		let (opened, _) = tokio::join!(opened, peer);
		opened.unwrap();

		closed_rx
			.await
			.expect("the daemon closed, so the sentinel must arrive");

		assert_eq!(
			*seen.lock().unwrap(),
			vec![FRAME.to_string(), WS_CLOSE_SENTINEL.to_string()],
			"the sink must see the text frame byte-for-byte and nothing else before \
			 the sentinel: a skipped frame that reached it would be parsed as a DTO, \
			 and one that ended the stream would trigger a needless reconnect"
		);
	}

	/// The webview → daemon leg (`ws_send`). Asserting on what the daemon
	/// RECEIVED, rather than on `send` returning `Ok`, is the only way to tell a
	/// delivered frame from one that was merely queued for a writer task that
	/// never wrote it.
	#[tokio::test]
	async fn a_frame_sent_by_the_webview_reaches_the_daemon_verbatim() {
		let _serialised = crate::FD_TESTS.lock().await;

		const FRAME: &str = r#"{"op":"subscribe","id":"a"}"#;

		let (listener, transport) = listening().await;
		let manager = WsBridgeManager::new();

		let peer = tokio::spawn(async move {
			let (stream, _) = listener.accept().await.expect("accept");
			let mut ws = tokio_tungstenite::accept_async(stream)
				.await
				.expect("upgrade");
			ws.next().await.expect("a frame").expect("a readable frame")
		});

		let reader = open_bridge(
			&transport,
			"c1".to_string(),
			"/v0/x".to_string(),
			silent_sink(),
			&manager,
		)
		.await
		.unwrap();

		manager.send("c1", FRAME.to_string())
			.expect("a live connection must accept frames");

		assert_eq!(
			peer.await.unwrap(),
			Message::Text(FRAME.to_string()),
			"the daemon must receive the frame the webview published, unaltered"
		);

		manager.close("c1");
		reader.await.expect("the reader must retire with it");
	}

	/// The shim sends on ids it believes are open; a stale one must come back as
	/// an error it can act on, not a silently dropped frame.
	#[test]
	fn sending_on_a_connection_that_was_never_opened_is_an_error() {
		let manager = WsBridgeManager::new();

		let err = manager.send("nope", "{}".to_string()).unwrap_err();

		assert_eq!(err, "no open ws connection nope");
	}

	/// The one state in which a live map entry has a dead writer: the writer task
	/// ended (its socket died) before the reader retired the entry. Distinct from
	/// the case above — the id IS open — and the shim is told so, rather than
	/// having the frame vanish into a channel nobody drains.
	#[test]
	fn sending_over_a_connection_whose_writer_has_ended_is_an_error() {
		let manager = WsBridgeManager::new();
		let (tx, rx) = mpsc::unbounded_channel();
		let (cancel, _cancelled) = oneshot::channel();
		drop(rx); // the writer task is gone, but its entry is not
		manager.connections.lock().unwrap().insert(
			"c1".to_string(),
			Connection {
				generation: 0,
				tx,
				_cancel: cancel,
			},
		);

		let err = manager.send("c1", "{}".to_string()).unwrap_err();

		assert_eq!(err, "ws connection c1 is closed");
	}

	/// A dial that fails must fail the open, with an error naming the route and
	/// the cause — and must leave nothing behind: an entry for a connection that
	/// never existed would accept `ws_send` calls forever and never deliver one.
	#[tokio::test]
	async fn a_dial_that_fails_leaves_no_connection_behind() {
		let _serialised = crate::FD_TESTS.lock().await;

		let (listener, transport) = listening().await;
		drop(listener); // the port is now both free and known-unused

		let manager = WsBridgeManager::new();
		let err = open_bridge(
			&transport,
			"c1".to_string(),
			"/v0/x".to_string(),
			silent_sink(),
			&manager,
		)
		.await
		.unwrap_err();

		assert!(
			err.contains("/v0/x") && err.contains("connect failed"),
			"the error must name the route and the cause; got {err:?}"
		);
		assert!(
			manager.connections.lock().unwrap().is_empty(),
			"a failed open must not register a connection"
		);
	}

	/// A writer whose socket has died must stop draining. The reader awaits the
	/// writer, so a writer that will not stop is a reader that never finishes —
	/// and a descriptor that never comes back, which is the whole failure this
	/// file exists to prevent. The connection's channel is deliberately still
	/// open here (this test holds a sender), so nothing BUT the failed write can
	/// end the writer.
	#[tokio::test]
	async fn a_writer_whose_socket_died_stops_draining() {
		let _serialised = crate::FD_TESTS.lock().await;

		let (listener, transport) = listening().await;
		// The peer resets only when told to. It has to complete the upgrade
		// first, and the RST has to come after WE have finished reading the
		// upgrade's response: an RST does not merely close the connection, it
		// also empties the receiving side's buffer, and on Windows that is
		// enforced to the byte — the 101 the peer had already sent is thrown
		// away with it and `open_bridge` fails at the handshake with
		// WSAECONNRESET, before this test reaches the writer it is about. (Unix
		// hands over bytes it has already buffered regardless, which is why the
		// unsynchronised version only ever failed on the Windows runner.) The
		// signal below is sent once `open_bridge` has returned, i.e. once the
		// handshake is entirely in our hands and the only thing left for the
		// socket to carry is the reset itself.
		let (kill, killed) = oneshot::channel::<()>();
		tokio::spawn(async move {
			let (stream, _) = listener.accept().await.expect("accept");
			// tokio deprecates `set_linger` because a NON-ZERO linger blocks the
			// thread when the socket drops. Zero is the opposite: close returns
			// immediately and sends the RST instead of a FIN, which is the only
			// deterministic way to make our next write fail. A clean FIN would
			// not do — writing to a half-closed socket succeeds, and the writer
			// would never learn. The alternative — closing with unread data
			// queued — depends on that data arriving first, which nothing here
			// can observe.
			#[allow(deprecated)]
			stream.set_linger(Some(Duration::ZERO))
				.expect("linger 0 makes close send an RST");
			let ws = tokio_tungstenite::accept_async(stream)
				.await
				.expect("upgrade");
			let _ = killed.await;
			drop(ws);
		});

		let manager = WsBridgeManager::new();
		let reader = open_bridge(
			&transport,
			"c1".to_string(),
			"/v0/x".to_string(),
			silent_sink(),
			&manager,
		)
		.await
		.unwrap();

		// The upgrade is complete on both sides, so nothing is left in flight
		// for the reset to destroy: from here it can only mean what this test
		// says it means, which is that the socket is dead.
		kill.send(())
			.expect("the peer must still be holding the socket open");

		let tx = manager
			.sender("c1")
			.expect("a live connection has a sender");

		// The reader retires the entry the moment it sees the reset, so an empty
		// map means the RST has been processed and the socket is unwritable.
		tokio::time::timeout(Duration::from_secs(5), async {
			while !manager.connections.lock().unwrap().is_empty() {
				tokio::time::sleep(Duration::from_millis(1)).await;
			}
		})
		.await
		.expect("the reader must retire a connection whose socket was reset");

		tx.send(Message::Text("{}".to_string()))
			.expect("the writer task is still draining its channel");

		tokio::time::timeout(Duration::from_secs(5), reader)
			.await
			.expect("a write that cannot succeed must end the writer, and with it the reader")
			.unwrap();

		drop(tx);
	}

	/// The app's own sink. `Channel::new` needs no webview, so the impl the
	/// running app actually uses is exercised here rather than only its stand-in.
	#[test]
	fn a_channel_sink_hands_the_frame_to_the_channel_verbatim() {
		const FRAME: &str = r#"{"id":"a"}"#;

		let seen = Arc::new(Mutex::new(Vec::new()));
		let recorded = Arc::clone(&seen);
		let channel = Channel::<String>::new(move |body| {
			if let tauri::ipc::InvokeResponseBody::Json(json) = body {
				recorded.lock().unwrap().push(json);
			}
			Ok(())
		});

		FrameSink::send(&channel, FRAME.to_string());

		// A `Channel<String>` serialises what it is given, so the frame reaches the
		// webview as a JSON string — one encoding step, applied to the whole frame.
		assert_eq!(
			*seen.lock().unwrap(),
			vec![serde_json::to_string(FRAME).unwrap()]
		);
	}

	/// A `Channel::send` failure means the webview is gone, i.e. the app is
	/// shutting down; it says nothing about the connection. The frame is dropped
	/// rather than propagated — a reader task that unwound here would abandon its
	/// socket without ever reaching the teardown at the end of the task.
	#[test]
	fn a_channel_sink_drops_a_frame_the_webview_cannot_take() {
		let attempts = Arc::new(AtomicU64::new(0));
		let counted = Arc::clone(&attempts);
		let channel = Channel::<String>::new(move |_body| {
			counted.fetch_add(1, Ordering::Relaxed);
			Err(tauri::Error::WebviewNotFound)
		});

		FrameSink::send(&channel, "{}".to_string());

		assert_eq!(
			attempts.load(Ordering::Relaxed),
			1,
			"the frame must be attempted, and the failure swallowed"
		);
	}
}
