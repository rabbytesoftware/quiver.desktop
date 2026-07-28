pub mod sidecar;

use std::sync::Arc;

use async_trait::async_trait;
use tauri::AppHandle;

use crate::connection::transport::Transport;
use crate::connection::types::{ConnectionConfig, CoreStatus, Emitter, QuiverConnection};

use self::sidecar::SidecarManager;

/// How the local daemon is reachable on this platform.
///
/// macOS and Linux use a unix socket. Windows cannot: Rust's async stack has no
/// AF_UNIX support there (design doc §2.2), so the daemon is bound to a
/// loopback port instead. That port is UNAUTHENTICATED — quiver.core has no
/// local auth — which is why it is pinned to 127.0.0.1 and never 0.0.0.0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalHost {
	Unix(String),
	Tcp(u16),
}

impl LocalHost {
	/// The value passed to `quiver daemon --host`.
	pub fn host_arg(&self) -> String {
		match self {
			// Bare `unix://` means core's own default path, which is what
			// default_socket_path() mirrors. Passing an explicit path would
			// diverge silently if core ever moved it.
			Self::Unix(_) => "unix://".into(),
			Self::Tcp(port) => format!("tcp://127.0.0.1:{port}"),
		}
	}
}

/// A free loopback port, learned by binding :0 and letting go.
///
/// The daemon does not report the port it bound — its entire startup output is
/// the gin route table and one `starting quiver daemon` line (verified against
/// stable-26.5.1) — so the app must choose. There is a TOCTOU window between
/// the drop and the daemon's bind; it is small, and this is the standard trade.
pub fn pick_free_port() -> Result<u16, String> {
	let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
	let port = listener.local_addr().map_err(|e| e.to_string())?.port();
	drop(listener);
	Ok(port)
}

#[cfg(unix)]
fn default_socket_path() -> String {
	let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
	format!("{}/.quiver/quiver.sock", home)
}

#[cfg(unix)]
fn local_host() -> LocalHost {
	LocalHost::Unix(default_socket_path())
}

#[cfg(windows)]
fn local_host() -> LocalHost {
	// A failed pick is not fatal here: the daemon would refuse the port and the
	// health wait would report it, which is a better error than panicking at
	// startup. 40257 is quiver.core's own documented example port.
	LocalHost::Tcp(pick_free_port().unwrap_or(40257))
}

#[cfg(unix)]
fn transport_for(host: &LocalHost) -> Arc<dyn Transport> {
	match host {
		LocalHost::Unix(path) => Arc::new(
			crate::connection::transport::unix::UnixTransport::new(path.clone()),
		),
		LocalHost::Tcp(port) => {
			Arc::new(crate::connection::transport::http::HttpTransport::new(
				format!("http://127.0.0.1:{port}"),
				None,
			))
		}
	}
}

#[cfg(windows)]
fn transport_for(host: &LocalHost) -> Arc<dyn Transport> {
	match host {
		LocalHost::Tcp(port) => {
			Arc::new(crate::connection::transport::http::HttpTransport::new(
				format!("http://127.0.0.1:{port}"),
				None,
			))
		}
		// Unreachable on Windows — local_host() never builds a Unix variant
		// there — but the match must be total.
		LocalHost::Unix(_) => {
			Arc::new(crate::connection::transport::http::HttpTransport::new(
				"http://127.0.0.1:40257",
				None,
			))
		}
	}
}

pub struct LocalConnection {
	config: ConnectionConfig,
	transport: Arc<dyn Transport>,
	sidecar: SidecarManager,
	host: LocalHost,
}

impl Default for LocalConnection {
	fn default() -> Self {
		Self::new()
	}
}

impl LocalConnection {
	pub fn new() -> Self {
		let host = local_host();
		Self {
			config: ConnectionConfig {
				id: "local".into(),
				name: "Local".into(),
				kind: "local".into(),
				url: None,
				api_version: "v0".into(),
			},
			transport: transport_for(&host),
			sidecar: SidecarManager::new(host.clone()),
			host,
		}
	}
}

#[async_trait]
impl QuiverConnection for LocalConnection {
	async fn start(&self, app: &AppHandle) {
		log::info!("[local] starting — host: {:?}", self.host);
		app.emit_core_status(CoreStatus::Starting);

		if let Err(e) = self.sidecar.spawn(app).await {
			log::error!("[local] sidecar spawn failed: {e}");
			app.emit_core_status(CoreStatus::Disconnected);
			return;
		}

		if let Err(e) = self.sidecar.wait_for_ready(self.transport.as_ref()).await {
			log::error!("[local] sidecar health check failed: {e}");
			app.emit_core_status(CoreStatus::Disconnected);
			return;
		}

		log::info!("[local] ready");
		app.emit_core_status(CoreStatus::Ready);
	}

	async fn teardown(&self) {}

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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unix_host_arg_is_the_bare_default_scheme() {
		// `unix://` with no path means "quiver.core's default", which is
		// ~/.quiver/quiver.sock — the same path default_socket_path() builds.
		// Passing an explicit path would silently diverge if core ever moved it.
		assert_eq!(LocalHost::Unix("/x/y.sock".into()).host_arg(), "unix://");
	}

	#[test]
	fn tcp_host_arg_pins_loopback_not_all_interfaces() {
		let arg = LocalHost::Tcp(51234).host_arg();
		assert_eq!(arg, "tcp://127.0.0.1:51234");
		assert!(
			!arg.contains("0.0.0.0"),
			"binding all interfaces would expose an unauthenticated daemon to the network"
		);
	}

	/// The daemon never prints the port it bound (verified against
	/// stable-26.5.1), so the app has to choose one and pass it in. Binding :0
	/// and dropping is how we learn a free number.
	#[test]
	fn pick_free_port_returns_a_usable_port() {
		// Two real listeners — `pick_free_port`'s and this test's — in the same
		// process as `connection::bridge`'s descriptor count, so this takes the
		// crate-wide guard like every other socket test. `blocking_lock` rather
		// than `.await`: this one is synchronous, and there is no runtime here
		// for the async form to belong to.
		let _serialised = crate::FD_TESTS.blocking_lock();

		let port = pick_free_port().expect("must find a free port");
		assert!(port > 0);
		std::net::TcpListener::bind(("127.0.0.1", port))
			.expect("the port pick_free_port returned must actually be bindable");
	}
}
