//! Everything the app injects to say how the local daemon is reached and
//! where its binary comes from.

use std::path::PathBuf;

use super::host::{dev_quiver_home, local_host, LocalHost};

/// Where and how the local quiver.core runs.
#[derive(Debug, Clone)]
pub struct LocalConfig {
    pub(super) host: LocalHost,
    /// A dev build points the daemon at this checkout's own `.quiver`, never
    /// the user's real one. `None` in a shipped build.
    pub(super) dev_home: Option<PathBuf>,
    /// The seed binary shipped with the app, used until quiver.core has
    /// self-updated into its own home. Replaces Tauri's `externalBin` sidecar.
    pub(super) bundled_binary: Option<PathBuf>,
}

impl LocalConfig {
    /// `dev_checkout` is `Some(dir)` for a development build of the checkout at
    /// `dir` (socket, pipe and home are scoped to it, see `host`), and `None`
    /// for a shipped build. `bundled_binary` is the shipped seed Core, if any.
    pub fn new(dev_checkout: Option<&str>, bundled_binary: Option<PathBuf>) -> Self {
        Self {
            host: local_host(dev_checkout),
            dev_home: dev_checkout.and_then(|dir| dev_quiver_home(dir, true)),
            bundled_binary,
        }
    }

    /// Where Tauri's sidecar resolution looked: `quiver` (`quiver.exe`) next to
    /// the running executable, if it is there.
    pub fn bundled_next_to_current_exe() -> Option<PathBuf> {
        let name = if cfg!(windows) {
            "quiver.exe"
        } else {
            "quiver"
        };
        let path = std::env::current_exe().ok()?.parent()?.join(name);
        path.exists().then_some(path)
    }

    pub fn host(&self) -> &LocalHost {
        &self.host
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shipped_build_never_scopes_the_daemon_away_from_the_real_home() {
        let config = LocalConfig::new(None, None);
        assert_eq!(config.dev_home, None);
        #[cfg(unix)]
        assert_eq!(config.host.host_arg(false), "unix://");
    }

    #[test]
    fn a_dev_build_scopes_the_daemon_to_its_own_checkout() {
        let config = LocalConfig::new(Some("/work/quiver"), Some(PathBuf::from("/opt/quiver")));
        assert_eq!(config.dev_home, Some(PathBuf::from("/work/quiver/.quiver")));
        assert_eq!(config.bundled_binary, Some(PathBuf::from("/opt/quiver")));
        #[cfg(unix)]
        assert!(config.host.host_arg(true).starts_with("unix:///"));
    }
}
