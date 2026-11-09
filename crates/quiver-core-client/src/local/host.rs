//! Where the local quiver.core daemon lives on this platform.

use std::sync::Arc;

use crate::transport::Transport;

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
/// Anchored to `manifest_dir` (this checkout's own directory, passed in by the
/// app from a compile-time anchor) rather than the process's runtime cwd: `make dev-bundle` launches its `.app` via `open`, which does
/// not reliably inherit the terminal's cwd, so only a compile-time anchor is
/// right for every dev target, not just `tauri dev`. `is_debug_build` is
/// true for a development build and false for a shipped one, so a shipped
/// production build is never scoped away from a user's real installed arrows
/// and config.
pub(super) fn dev_quiver_home(
    manifest_dir: &str,
    is_debug_build: bool,
) -> Option<std::path::PathBuf> {
    is_debug_build.then(|| std::path::Path::new(manifest_dir).join(".quiver"))
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
fn default_socket_path(dev_checkout: Option<&str>) -> String {
    if let Some(path) = dev_checkout.and_then(|dir| dev_socket_override(dir, true)) {
        return path.to_string_lossy().into_owned();
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    format!("{}/.quiver/quiver.sock", home)
}

#[cfg(unix)]
pub(super) fn local_host(dev_checkout: Option<&str>) -> LocalHost {
    LocalHost::Unix(default_socket_path(dev_checkout))
}

#[cfg(windows)]
pub(super) fn local_host(dev_checkout: Option<&str>) -> LocalHost {
    let name = dev_checkout
        .and_then(|dir| dev_pipe_name(dir, true))
        .unwrap_or_else(|| LOCAL_PIPE_NAME.to_string());
    LocalHost::Pipe(name)
}

#[cfg(unix)]
pub(super) fn transport_for(host: &LocalHost) -> Arc<dyn Transport> {
    match host {
        LocalHost::Unix(path) => Arc::new(crate::transport::unix::UnixTransport::new(path.clone())),
    }
}

#[cfg(windows)]
pub(super) fn transport_for(host: &LocalHost) -> Arc<dyn Transport> {
    match host {
        LocalHost::Pipe(name) => Arc::new(crate::transport::pipe::PipeTransport::new(name)),
    }
}

#[cfg(test)]
mod tests;
