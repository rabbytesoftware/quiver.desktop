pub mod sidecar;

use std::sync::Arc;

use async_trait::async_trait;
use tauri::AppHandle;

use crate::connection::transport::Transport;
use crate::connection::types::{ConnectionConfig, CoreStatus, Emitter, QuiverConnection};

use self::sidecar::SidecarManager;

/// How the local daemon is reachable on this platform.
///
/// macOS and Linux use a unix socket. Windows uses a named pipe, which
/// quiver.core creates with a DACL granting only the current user and binds
/// with remote clients refused — the same "reachable by you and nobody else"
/// guarantee the unix socket's file mode gives. (Windows used to bind an
/// unauthenticated loopback port instead, reachable by every local user.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalHost {
	#[cfg(unix)]
	Unix(String),
	#[cfg(windows)]
	Pipe(String),
}

impl LocalHost {
	/// The value passed to `quiver daemon --host`. `dev_override_active` is
	/// `quiver_home().is_some()` at the real call site, taken as a parameter
	/// so the two paths this can take are both directly testable — `cargo
	/// test` is itself a debug build, so a call with no parameter could never
	/// exercise the production branch at all.
	#[cfg_attr(windows, allow(unused_variables))]
	pub fn host_arg(&self, dev_override_active: bool) -> String {
		match self {
			#[cfg(unix)]
			Self::Unix(path) => {
				if dev_override_active {
					// In a dev build, core's own bare-`unix://` default now
					// resolves under QUIVER_HOME (see `quiver_home`), which is
					// this checkout's own long, unshortened path — too long for
					// a unix socket (see `dev_socket_override`). `path` is
					// already the short override `default_socket_path()`
					// computed, so it has to be passed explicitly here instead.
					format!("unix://{path}")
				} else {
					// In production, bare `unix://` means core's own default
					// path, which is what default_socket_path() mirrors there
					// too. Passing an explicit path would diverge silently if
					// core ever moved it.
					"unix://".into()
				}
			}
			// Always explicit: the pipe namespace is machine-wide, so there is
			// no per-home default for core to derive, and a dev build's own
			// name (see `dev_pipe_name`) has to reach core as-is.
			#[cfg(windows)]
			Self::Pipe(name) => format!("npipe://{name}"),
		}
	}
}

/// The pipe quiver.core binds by default on Windows, and so the one a shipped
/// build addresses. FIXED, never picked per construction.
///
/// `LocalConnection::new()` runs at startup AND on every switch back to local,
/// so an address that changed each time would spawn a daemon per construction
/// and orphan the last. A fixed name makes every construction address the same
/// daemon, and a second spawn fails to create the pipe (core creates it with
/// `FILE_FLAG_FIRST_PIPE_INSTANCE`) and exits instead of forking the app's
/// view of "local". A stranger squatting the name cannot answer `/v0/health`,
/// which `SidecarManager::ensure_running` decides on, so the app reports the
/// local core as unreachable instead of proxying to it.
#[cfg(windows)]
pub const LOCAL_PIPE_NAME: &str = "quiver";

/// The directory quiver.core should treat as its home in THIS build, when it
/// must differ from the user's real one. `None` means "don't override" --
/// use quiver.core's own default, `$HOME/.quiver`.
///
/// Anchored to `manifest_dir` (this checkout's own `src-tauri/`, passed in as
/// `CARGO_MANIFEST_DIR` at the real call site) rather than the process's
/// runtime cwd: `make dev-bundle` launches its `.app` via `open`, which does
/// not reliably inherit the terminal's cwd, so only a compile-time anchor is
/// right for every dev target, not just `tauri dev`. `is_debug_build` is
/// `cfg!(debug_assertions)` at the call site -- true for `tauri dev` and
/// `tauri build --debug` (`dev-desktop`, `dev-mock`, `dev-bundle`), false for
/// the real `tauri build` (`build-app`), so a shipped production build is
/// never scoped away from a user's real installed arrows and config.
fn dev_quiver_home(manifest_dir: &str, is_debug_build: bool) -> Option<std::path::PathBuf> {
	is_debug_build.then(|| std::path::Path::new(manifest_dir).join(".quiver"))
}

/// `dev_quiver_home` at its real call site: this build's actual profile and
/// this checkout's actual location, baked in at compile time.
fn quiver_home() -> Option<std::path::PathBuf> {
	dev_quiver_home(env!("CARGO_MANIFEST_DIR"), cfg!(debug_assertions))
}

/// Unix domain socket paths are capped at roughly 104 bytes (`sockaddr_un`),
/// and a checkout under this project's own workspace tooling runs well past
/// that on its own — 173 bytes, measured, for one worktree — so the socket
/// cannot live under `dev_quiver_home`'s checkout-anchored directory the way
/// the rest of a dev build's data can. It lives in the OS temp dir instead,
/// named by a hash of the checkout path: short and safely within the limit,
/// yet still unique per worktree and identical across reruns of the same one
/// — the same stability `dev_quiver_home` gives everything else. `None` in a
/// release build, for the same reason `dev_quiver_home` is.
#[cfg(unix)]
fn dev_socket_override(manifest_dir: &str, is_debug_build: bool) -> Option<std::path::PathBuf> {
	use std::hash::{Hash, Hasher};

	if !is_debug_build {
		return None;
	}
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	manifest_dir.hash(&mut hasher);
	Some(std::env::temp_dir().join(format!("quiver-dev-{:016x}.sock", hasher.finish())))
}

/// A dev build's pipe name: unique per checkout and stable across reruns of
/// the same one, so a debug build never adopts (or collides with) the
/// installed app's daemon on `LOCAL_PIPE_NAME` — the same isolation
/// `dev_socket_override` gives a unix dev build. `None` in a release build.
#[cfg(windows)]
fn dev_pipe_name(manifest_dir: &str, is_debug_build: bool) -> Option<String> {
	use std::hash::{Hash, Hasher};

	if !is_debug_build {
		return None;
	}
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	manifest_dir.hash(&mut hasher);
	Some(format!("quiver-dev-{:016x}", hasher.finish()))
}

#[cfg(unix)]
fn default_socket_path() -> String {
	if let Some(path) = dev_socket_override(env!("CARGO_MANIFEST_DIR"), cfg!(debug_assertions))
	{
		return path.to_string_lossy().into_owned();
	}
	let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
	format!("{}/.quiver/quiver.sock", home)
}

#[cfg(unix)]
fn local_host() -> LocalHost {
	LocalHost::Unix(default_socket_path())
}

#[cfg(windows)]
fn local_host() -> LocalHost {
	let name = dev_pipe_name(env!("CARGO_MANIFEST_DIR"), cfg!(debug_assertions))
		.unwrap_or_else(|| LOCAL_PIPE_NAME.to_string());
	LocalHost::Pipe(name)
}

#[cfg(unix)]
fn transport_for(host: &LocalHost) -> Arc<dyn Transport> {
	match host {
		LocalHost::Unix(path) => Arc::new(
			crate::connection::transport::unix::UnixTransport::new(path.clone()),
		),
	}
}

#[cfg(windows)]
fn transport_for(host: &LocalHost) -> Arc<dyn Transport> {
	match host {
		LocalHost::Pipe(name) => {
			Arc::new(crate::connection::transport::pipe::PipeTransport::new(name))
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

		// `ensure_running`, not `spawn` + `wait_for_ready`: `new()` runs again
		// on every switch back to local, and an unconditional spawn is what
		// left Windows hosting a daemon per switch. See `LOCAL_PIPE_NAME`.
		if let Err(e) = self
			.sidecar
			.ensure_running(app, self.transport.as_ref())
			.await
		{
			log::error!("[local] sidecar did not become ready: {e}");
			app.emit_core_status(CoreStatus::Disconnected);
			return;
		}

		log::info!("[local] ready");
		app.emit_core_status(CoreStatus::Ready);
	}

	/// Stop the daemon this connection spawned, and only that one. See
	/// [`SidecarManager::reap`].
	async fn teardown(&self) {
		self.sidecar.reap(self.transport.as_ref()).await;
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_debug_build_scopes_home_under_the_checkout_it_was_built_from() {
		assert_eq!(
			dev_quiver_home("/Users/dev/quiver.desktop/src-tauri", true),
			Some(std::path::PathBuf::from(
				"/Users/dev/quiver.desktop/src-tauri/.quiver"
			))
		);
	}

	/// The property production ships on: a real `tauri build` (not `--debug`)
	/// must never override quiver.core's own default, or a shipped app would
	/// start scoping a user's real installed arrows and config away from
	/// their actual `~/.quiver`.
	#[test]
	fn a_release_build_never_overrides_the_real_home() {
		assert_eq!(
			dev_quiver_home("/Users/dev/quiver.desktop/src-tauri", false),
			None
		);
	}

	#[cfg(unix)]
	#[test]
	fn unix_host_arg_is_the_bare_default_scheme_in_production() {
		// `unix://` with no path means "quiver.core's default", which is
		// ~/.quiver/quiver.sock — the same path default_socket_path() builds.
		// Passing an explicit path would silently diverge if core ever moved it.
		assert_eq!(
			LocalHost::Unix("/x/y.sock".into()).host_arg(false),
			"unix://"
		);
	}

	/// The other half of the fix in this module: a dev build's own default
	/// resolves under QUIVER_HOME (its own long checkout path) if left bare,
	/// which is exactly what breaks the bind (see `dev_socket_override`). The
	/// explicit path must be passed instead, not core's default.
	#[cfg(unix)]
	#[test]
	fn unix_host_arg_is_the_explicit_short_override_in_a_dev_build() {
		assert_eq!(
			LocalHost::Unix("/tmp/quiver-dev-abc123.sock".into()).host_arg(true),
			"unix:///tmp/quiver-dev-abc123.sock"
		);
	}

	/// The name reaches core verbatim, as an `npipe://` URI: core has no
	/// per-home pipe default to derive, and the dev name in particular must not
	/// be replaced by core's own.
	#[cfg(windows)]
	#[test]
	fn pipe_host_arg_names_the_pipe_explicitly() {
		assert_eq!(
			LocalHost::Pipe("quiver".into()).host_arg(false),
			"npipe://quiver"
		);
		assert_eq!(
			LocalHost::Pipe("quiver-dev-abc123".into()).host_arg(true),
			"npipe://quiver-dev-abc123"
		);
	}

	#[cfg(windows)]
	#[test]
	fn dev_pipe_name_is_none_in_a_release_build() {
		assert_eq!(
			dev_pipe_name("C:\\dev\\quiver.desktop\\src-tauri", false),
			None
		);
	}

	#[cfg(windows)]
	#[test]
	fn dev_pipe_name_is_stable_per_checkout_and_distinct_between_them() {
		let a = "C:\\dev\\quiver.desktop-a\\src-tauri";
		let b = "C:\\dev\\quiver.desktop-b\\src-tauri";
		assert_eq!(dev_pipe_name(a, true), dev_pipe_name(a, true));
		assert_ne!(dev_pipe_name(a, true), dev_pipe_name(b, true));
	}

	/// A dev build must never share the shipped build's pipe, or `tauri dev`
	/// would adopt (and then fail to isolate from) the installed app's daemon.
	#[cfg(windows)]
	#[test]
	fn dev_pipe_name_never_collides_with_the_shipped_name() {
		let name = dev_pipe_name("C:\\dev\\quiver.desktop\\src-tauri", true).unwrap();
		assert_ne!(name, LOCAL_PIPE_NAME);
		assert!(name.starts_with("quiver-dev-"), "got {name:?}");
	}

	#[cfg(unix)]
	#[test]
	fn dev_socket_override_is_none_in_a_release_build() {
		assert_eq!(
			dev_socket_override("/Users/dev/quiver.desktop/src-tauri", false),
			None
		);
	}

	/// The property that matters for `default_socket_path()`: the same
	/// checkout must always get the same socket path, or every restart of
	/// the same worktree would leave the previous run's daemon unreachable
	/// and orphaned — the orphaned-daemon failure `LOCAL_PIPE_NAME`'s
	/// own doc describes.
	#[cfg(unix)]
	#[test]
	fn dev_socket_override_is_the_same_path_for_the_same_checkout() {
		let manifest_dir = "/Users/dev/quiver.desktop/src-tauri";
		assert_eq!(
			dev_socket_override(manifest_dir, true),
			dev_socket_override(manifest_dir, true)
		);
	}

	#[cfg(unix)]
	#[test]
	fn dev_socket_override_differs_between_checkouts() {
		assert_ne!(
			dev_socket_override("/Users/dev/quiver.desktop-a/src-tauri", true),
			dev_socket_override("/Users/dev/quiver.desktop-b/src-tauri", true)
		);
	}

	/// The whole reason this override exists: a checkout under this
	/// project's own workspace tooling is long enough that the real
	/// `.quiver`-anchored path (173 bytes, measured) exceeds `sockaddr_un`'s
	/// ~104-byte limit and fails to bind with EINVAL. The override must stay
	/// short regardless of how long the checkout path itself is.
	#[cfg(unix)]
	#[test]
	fn dev_socket_override_stays_well_under_the_unix_socket_path_limit() {
		let long_manifest_dir = "/Users/char2cs/.crowbar/projects/cb81ea03-54b7-4093-8bdd-fdd6b183e91d/github.com/rabbytesoftware/quiver.desktop/feature/remote-control/worktree/src-tauri";
		let path = dev_socket_override(long_manifest_dir, true)
			.expect("must override in a dev build");
		assert!(
			path.to_string_lossy().len() < 100,
			"got a {}-byte path, too close to sockaddr_un's ~104-byte limit: {path:?}",
			path.to_string_lossy().len()
		);
	}

	/// The defect this guards: `local_host()` used to call `pick_free_port()` on
	/// Windows, so every `LocalConnection::new()` — startup, and every switch
	/// back to local — addressed a DIFFERENT daemon and spawned one, leaving the
	/// previous ones running and unreachable.
	///
	/// Two calls, one assertion: the local daemon's address must not depend on
	/// when it was asked for. It goes red on Windows the moment a free-port pick
	/// comes back, and red on unix if the socket path ever picks up a nonce.
	#[test]
	fn the_local_daemon_address_is_the_same_on_every_construction() {
		assert_eq!(
			local_host(),
			local_host(),
			"a local address that changes per construction spawns a daemon per \
			 construction and orphans the last one"
		);
	}

	/// And it is the address this platform is documented to use — the pairing a
	/// compiler cannot check, written out per platform in literals rather than
	/// derived from the thing under test.
	///
	/// `cargo test` is itself a debug build, so this can only ever observe the
	/// dev-scoped override (see `dev_socket_override`) on unix, never the real
	/// production default — that half is covered directly by
	/// `unix_host_arg_is_the_bare_default_scheme_in_production` and the
	/// `dev_quiver_home`/`dev_socket_override` tests instead.
	#[test]
	fn the_local_daemon_address_is_the_documented_one_for_this_platform() {
		let host = local_host();
		#[cfg(unix)]
		assert!(
			matches!(&host, LocalHost::Unix(p) if p.contains("quiver-dev-") && p.ends_with(".sock")),
			"unix addresses the dev-scoped override under test; got {host:?}"
		);
		// The literal, not `LOCAL_PIPE_NAME`: comparing the constant to itself
		// would pass whatever it were changed to.
		#[cfg(windows)]
		assert!(
			matches!(&host, LocalHost::Pipe(n) if n.starts_with("quiver-dev-")),
			"windows addresses the dev-scoped pipe under test; got {host:?}"
		);
	}
}
