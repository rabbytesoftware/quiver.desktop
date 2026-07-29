use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

use crate::connection::transport::Transport;

use super::LocalHost;

const HEALTH_RETRY_MS: u64 = 200;
const HEALTH_MAX_ATTEMPTS: u32 = 25;

pub struct SidecarManager {
	host: LocalHost,
}

impl SidecarManager {
	pub fn new(host: LocalHost) -> Self {
		Self { host }
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

	pub async fn spawn(&self, app: &AppHandle) -> Result<(), String> {
		// `sidecar(name)` resolves to `dirname(current_exe)/name`, so the
		// argument must be the bare binary name, NOT the `binaries/quiver`
		// path from tauri.conf.json's externalBin. `tauri dev` runs unbundled
		// and never catches a mistake here — use `make dev-bundle`.
		app.shell()
			.sidecar("quiver")
			.map_err(|e| e.to_string())?
			.args(["daemon", "--host", &self.host.host_arg()])
			.spawn()
			.map_err(|e| e.to_string())?;
		Ok(())
	}

	pub async fn wait_for_ready(&self, transport: &dyn Transport) -> Result<(), String> {
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
}
