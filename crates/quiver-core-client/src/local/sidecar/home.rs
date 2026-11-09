//! Where quiver.core keeps its home, and which binary lives there.

use std::path::{Path, PathBuf};

/// Looks for a self-installed quiver.core at its stable, version-independent
/// install path (`{quiver_home}/self/` — see quiver.core's Task 1.8, not the
/// per-`@ref` `namespaces/` tree ordinary arrows use). Returns `None` when
/// nothing is installed there yet, in which case the caller falls back to
/// the bundled bootstrap sidecar.
pub(super) fn resolve_binary_path(quiver_home: &Path) -> Option<PathBuf> {
    let bin_name = if cfg!(windows) {
        "quiver.exe"
    } else {
        "quiver"
    };
    let candidate = quiver_home.join("self").join(bin_name);
    candidate.exists().then_some(candidate)
}

/// Mirrors quiver.core's own default home-directory resolution exactly —
/// see quiver.core's `internal/core/metadata/resolve_home.go` and
/// `resolve_home_windows.go`, both driven by `metadata.yaml`'s
/// `paths.home`: `QUIVER_HOME` wins when set (and non-empty), otherwise the
/// platform default (`~/.quiver` on Unix, `C:\Users\{{USER}}\Documents\.quiver`
/// on Windows). This has to track that default exactly, not just plausibly
/// — it is how this app finds the same `self/` directory quiver.core's own
/// Task 1.8 installs into.
pub(super) fn quiver_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("QUIVER_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    platform_default_home()
}

/// The one home `spawn` uses both to look for a self-installed binary and
/// (via `dev_home`, at the call site) to scope the spawned daemon's data
/// directory — the same directory for both, always, which is what keeps
/// `dev_home`'s isolation guarantee whole. `Some` (a dev build) wins over
/// [`quiver_home_dir`]'s real, production resolution entirely: a dev
/// build must never fall through to a developer's real, unrelated
/// self-installed Core just because none is installed under the checkout's
/// own scoped home yet. `None` (a release build) falls back to
/// `quiver_home_dir()` unchanged.
pub(super) fn effective_home(dev_home: Option<&Path>) -> PathBuf {
    dev_home
        .map(Path::to_path_buf)
        .unwrap_or_else(quiver_home_dir)
}

/// The Unix half of `quiver_home_dir`'s platform default:
/// `$HOME/.quiver`, matching quiver.core's own `resolveHome()` (which expands
/// its `~/.quiver` template via `os.UserHomeDir()` — `$HOME` on Unix). Falling
/// back to `/tmp` when `HOME` is unset mirrors the same fallback already used
/// for the dev socket path in `local::mod::default_socket_path`, and keeps
/// this infallible rather than forcing every caller to handle an env lookup
/// that essentially never fails on a real desktop.
#[cfg(unix)]
fn platform_default_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".quiver")
}

/// The Windows half of `quiver_home_dir`'s platform default.
/// `metadata.yaml`'s `paths.home.windows` documents the literal template
/// `C:\Users\{{USER}}\Documents\.quiver`; quiver.core's own
/// `resolve_home_windows.go` fills `{{USER}}` from `os/user.Current()`
/// (falling back to `"unknown"` if that fails) rather than deriving the whole
/// path from `%USERPROFILE%`, so this mirrors that exactly — including the
/// same `"unknown"` fallback — instead of reconstructing a differently-shaped
/// but plausible-looking path.
#[cfg(windows)]
fn platform_default_home() -> PathBuf {
    let user = std::env::var("USERNAME").unwrap_or_else(|_| "unknown".into());
    PathBuf::from(format!(r"C:\Users\{user}\Documents\.quiver"))
}

#[cfg(test)]
mod tests;
