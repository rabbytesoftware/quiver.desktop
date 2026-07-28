use std::sync::Arc;

use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use tokio::sync::RwLock;

use crate::connection::bridge::WsBridgeManager;
use crate::connection::local::LocalConnection;
use crate::connection::remote::RemoteConnection;
use crate::connection::types::{ConnectionConfig, QuiverConnection};

const KEYRING_SERVICE: &str = "quiver.desktop";
const STORE_KEY: &str = "connections";

pub struct ConnectionManager {
	active: RwLock<Box<dyn QuiverConnection>>,
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
		}
	}

	pub async fn start(&self, app: AppHandle) {
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
		// Build first: a switch that cannot be completed must leave the current
		// connection and its streams exactly as they were.
		let new_conn = build_connection(app, id).await?;
		let mut guard = self.active.write().await;
		retire_streams_and_teardown(&app.state::<WsBridgeManager>(), &**guard).await;
		*guard = new_conn;
		guard.start(app).await;
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
		// Keep active in-memory config in sync
		let mut guard = self.active.write().await;
		if guard.config().id == id {
			guard.set_name(name);
		}
		Ok(())
	}

	pub async fn transport(&self) -> Arc<dyn crate::connection::transport::Transport> {
		self.active.read().await.transport()
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
/// Streams first, connection second — as intent, not as a demonstrated property.
/// Today the order cannot matter: both `teardown()` implementations
/// (`local::LocalConnection`, `remote::RemoteConnection`) are empty, this is their
/// only caller, and swapping these two lines leaves the whole suite green. Nothing
/// tests the ordering, and this comment should not pretend otherwise.
///
/// It is written this way for the day `teardown()` stops being a no-op — killing
/// the sidecar, dropping a TLS session. Then it takes the streams' sockets down
/// with it, and a reader that sees its socket die announces `WS_CLOSE_SENTINEL`,
/// which tells the shim to RECONNECT, mid-switch, to the peer being switched away
/// from. Retiring first cancels those readers silently. Whoever gives `teardown()`
/// a body owns writing the test that makes this ordering real.
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

	/// Stands in for the connection being switched away from. Everything a real
	/// one does at switch time that needs an `AppHandle` (`start`) or a live
	/// daemon (`transport`) is out of this test's reach and out of its way.
	struct StubConnection {
		config: ConnectionConfig,
		torn_down: Arc<AtomicBool>,
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
			unreachable!("the outgoing connection is never dialled again")
		}

		fn config(&self) -> &ConnectionConfig {
			&self.config
		}

		fn set_name(&mut self, name: String) {
			self.config.name = name;
		}
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
		let outgoing = StubConnection {
			config: ConnectionConfig {
				id: "local".into(),
				name: "Local".into(),
				kind: "local".into(),
				url: None,
				api_version: "v0".into(),
			},
			torn_down: Arc::clone(&torn_down),
		};

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
}
