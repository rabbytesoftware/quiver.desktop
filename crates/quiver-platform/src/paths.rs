//! Where the app keeps things, per OS.
//!
//! The locations are the ones Tauri's path resolver produced for the bundle
//! identifier in `tauri.conf.json`, so a user upgrading from the Tauri build
//! finds their existing settings. The environment is a parameter ([`PathEnv`])
//! so every OS's answer is testable on any host.

use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// `identifier` in `src-tauri/tauri.conf.json`. Changing it orphans every
/// existing user's settings.
pub const APP_IDENTIFIER: &str = "com.rabbytesoftware.quiver";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}

/// The slice of the process environment path resolution reads.
#[derive(Debug, Clone)]
pub struct PathEnv {
    pub os: Os,
    /// `$HOME` (Unix) / the profile directory (Windows, unused there).
    pub home: Option<PathBuf>,
    pub xdg_data_home: Option<PathBuf>,
    pub xdg_cache_home: Option<PathBuf>,
    /// `%APPDATA%` (Roaming).
    pub appdata: Option<PathBuf>,
    /// `%LOCALAPPDATA%`.
    pub local_appdata: Option<PathBuf>,
    /// `%USERNAME%`.
    pub username: Option<String>,
    /// `QUIVER_HOME`, quiver.core's own override of its home directory.
    pub quiver_home: Option<PathBuf>,
}

fn var_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

impl PathEnv {
    pub fn from_process() -> Self {
        Self {
            os: Os::current(),
            home: var_path("HOME").or_else(|| var_path("USERPROFILE")),
            xdg_data_home: var_path("XDG_DATA_HOME"),
            xdg_cache_home: var_path("XDG_CACHE_HOME"),
            appdata: var_path("APPDATA"),
            local_appdata: var_path("LOCALAPPDATA"),
            username: std::env::var("USERNAME").ok(),
            quiver_home: var_path("QUIVER_HOME"),
        }
    }
}

/// Every directory the app uses, resolved once.
#[derive(Debug, Clone)]
pub struct AppPaths {
    data: PathBuf,
    cache: PathBuf,
    log: PathBuf,
    quiver_home: PathBuf,
}

impl AppPaths {
    /// Resolves against this process's environment.
    pub fn resolve() -> Result<Self> {
        Self::from_env(&PathEnv::from_process())
    }

    pub fn from_env(env: &PathEnv) -> Result<Self> {
        let home = env.home.as_deref();
        let need_home = || home.ok_or(Error::NoHome);
        let (data, cache, log) = match env.os {
            Os::Linux => {
                let share = match &env.xdg_data_home {
                    Some(p) => p.clone(),
                    None => need_home()?.join(".local/share"),
                };
                let cache = match &env.xdg_cache_home {
                    Some(p) => p.clone(),
                    None => need_home()?.join(".cache"),
                };
                (
                    share.join(APP_IDENTIFIER),
                    cache.join(APP_IDENTIFIER),
                    share.join(APP_IDENTIFIER).join("logs"),
                )
            }
            Os::MacOs => {
                let lib = need_home()?.join("Library");
                (
                    lib.join("Application Support").join(APP_IDENTIFIER),
                    lib.join("Caches").join(APP_IDENTIFIER),
                    lib.join("Logs").join(APP_IDENTIFIER),
                )
            }
            Os::Windows => {
                let roaming = env.appdata.clone().ok_or(Error::NoHome)?;
                let local = env.local_appdata.clone().ok_or(Error::NoHome)?;
                (
                    roaming.join(APP_IDENTIFIER),
                    local.join(APP_IDENTIFIER),
                    local.join(APP_IDENTIFIER).join("logs"),
                )
            }
        };
        Ok(Self {
            data,
            cache,
            log,
            quiver_home: quiver_home(env),
        })
    }

    /// Settings files live here (`connections.json`, `device.json`).
    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache
    }

    pub fn log_dir(&self) -> &Path {
        &self.log
    }

    /// quiver.core's home (`QUIVER_HOME`, else the platform default). Not the
    /// app's own data: it belongs to the daemon.
    pub fn quiver_home(&self) -> &Path {
        &self.quiver_home
    }

    /// Where a self-installed quiver.core binary lives.
    pub fn core_binary_dir(&self) -> PathBuf {
        self.quiver_home.join("self")
    }

    /// The self-installed daemon binary, when one exists.
    pub fn core_binary(
        &self,
        windows: bool,
    ) -> Option<PathBuf> {
        core_binary_in(&self.quiver_home, windows)
    }
}

/// `<home>/self/quiver[.exe]`, when it exists.
pub fn core_binary_in(
    quiver_home: &Path,
    windows: bool,
) -> Option<PathBuf> {
    let name = if windows { "quiver.exe" } else { "quiver" };
    let candidate = quiver_home.join("self").join(name);
    candidate.exists().then_some(candidate)
}

/// Mirrors quiver.core's own default home-directory resolution exactly --
/// quiver.core's `internal/core/metadata/resolve_home.go` and
/// `resolve_home_windows.go`, both driven by `metadata.yaml`'s `paths.home`:
/// `QUIVER_HOME` wins when set (and non-empty), otherwise the platform default
/// (`~/.quiver` on Unix, `C:\Users\{{USER}}\Documents\.quiver` on Windows).
/// This has to track that default exactly, not just plausibly -- it is how
/// this app finds the same `self/` directory quiver.core installs into.
///
/// On Unix, `/tmp` stands in when `HOME` is unset (the same fallback the
/// socket path uses), keeping this infallible. On Windows quiver.core fills
/// `{{USER}}` from `os/user.Current()` with `"unknown"` as the fallback, and
/// so does this.
fn quiver_home(env: &PathEnv) -> PathBuf {
    if let Some(home) = &env.quiver_home {
        return home.clone();
    }
    match env.os {
        Os::Windows => {
            let user = env.username.as_deref().unwrap_or("unknown");
            PathBuf::from(format!(r"C:\Users\{user}\Documents\.quiver"))
        }
        _ => env
            .home
            .clone()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".quiver"),
    }
}

/// The local daemon's Unix socket: `$HOME/.quiver/quiver.sock` (`/tmp` when
/// `HOME` is unset). Deliberately not under `QUIVER_HOME`: it mirrors what
/// the Tauri app dialled.
pub fn default_socket_path(env: &PathEnv) -> PathBuf {
    env.home
        .clone()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".quiver/quiver.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The desktop shell ties a window to its menu entry and icon by the window's app id, which is this
    /// constant: the Linux entry, its file name and its icons must all carry it.
    #[test]
    fn the_linux_desktop_entry_and_icons_are_named_by_the_app_identifier() {
        let entry = include_str!("../../../installer/linux/com.rabbytesoftware.quiver.desktop");
        assert_eq!(APP_IDENTIFIER, "com.rabbytesoftware.quiver");
        assert!(entry.lines().any(|l| l == format!("Icon={APP_IDENTIFIER}")));
        assert!(entry
            .lines()
            .any(|l| l == format!("StartupWMClass={APP_IDENTIFIER}")));
        let largest = include_bytes!(
            "../../../installer/icons/hicolor/512x512/apps/com.rabbytesoftware.quiver.png"
        );
        assert!(largest.starts_with(b"\x89PNG"));
    }

    fn env(os: Os) -> PathEnv {
        PathEnv {
            os,
            home: Some("/home/u".into()),
            xdg_data_home: None,
            xdg_cache_home: None,
            appdata: Some(r"C:\Users\u\AppData\Roaming".into()),
            local_appdata: Some(r"C:\Users\u\AppData\Local".into()),
            username: Some("u".into()),
            quiver_home: None,
        }
    }

    #[test]
    fn the_identifier_is_the_one_the_tauri_build_shipped_under() {
        // Changing it orphans every user's data directory (the former tauri.conf.json `identifier`).
        assert_eq!(APP_IDENTIFIER, "com.rabbytesoftware.quiver");
    }

    #[test]
    fn linux_follows_xdg_with_home_fallbacks() {
        let p = AppPaths::from_env(&env(Os::Linux)).unwrap();
        assert_eq!(
            p.data_dir(),
            Path::new("/home/u/.local/share/com.rabbytesoftware.quiver")
        );
        assert_eq!(
            p.cache_dir(),
            Path::new("/home/u/.cache/com.rabbytesoftware.quiver")
        );
        assert_eq!(
            p.log_dir(),
            Path::new("/home/u/.local/share/com.rabbytesoftware.quiver/logs")
        );

        let mut e = env(Os::Linux);
        e.xdg_data_home = Some("/x/data".into());
        e.xdg_cache_home = Some("/x/cache".into());
        let p = AppPaths::from_env(&e).unwrap();
        assert_eq!(
            p.data_dir(),
            Path::new("/x/data/com.rabbytesoftware.quiver")
        );
        assert_eq!(
            p.cache_dir(),
            Path::new("/x/cache/com.rabbytesoftware.quiver")
        );
    }

    #[test]
    fn macos_uses_library_directories() {
        let p = AppPaths::from_env(&env(Os::MacOs)).unwrap();
        assert_eq!(
            p.data_dir(),
            Path::new("/home/u/Library/Application Support/com.rabbytesoftware.quiver")
        );
        assert_eq!(
            p.cache_dir(),
            Path::new("/home/u/Library/Caches/com.rabbytesoftware.quiver")
        );
        assert_eq!(
            p.log_dir(),
            Path::new("/home/u/Library/Logs/com.rabbytesoftware.quiver")
        );
    }

    #[test]
    fn windows_uses_roaming_for_data_and_local_for_cache_and_logs() {
        let p = AppPaths::from_env(&env(Os::Windows)).unwrap();
        assert_eq!(
            p.data_dir(),
            Path::new(r"C:\Users\u\AppData\Roaming").join(APP_IDENTIFIER)
        );
        assert_eq!(
            p.cache_dir(),
            Path::new(r"C:\Users\u\AppData\Local").join(APP_IDENTIFIER)
        );
        assert_eq!(
            p.log_dir(),
            Path::new(r"C:\Users\u\AppData\Local")
                .join(APP_IDENTIFIER)
                .join("logs")
        );
    }

    #[test]
    fn a_missing_home_is_an_error_where_it_is_needed() {
        let mut e = env(Os::Linux);
        e.home = None;
        assert!(matches!(AppPaths::from_env(&e), Err(Error::NoHome)));
    }

    #[test]
    fn quiver_home_prefers_the_override_then_the_platform_default() {
        let mut e = env(Os::Linux);
        e.quiver_home = Some("/custom".into());
        assert_eq!(
            AppPaths::from_env(&e).unwrap().quiver_home(),
            Path::new("/custom")
        );

        e.quiver_home = None;
        assert_eq!(
            AppPaths::from_env(&e).unwrap().quiver_home(),
            Path::new("/home/u/.quiver")
        );

        let w = AppPaths::from_env(&env(Os::Windows)).unwrap();
        assert_eq!(w.quiver_home(), Path::new(r"C:\Users\u\Documents\.quiver"));
    }

    #[test]
    fn quiver_home_falls_back_further_when_the_platform_env_is_also_unset() {
        let mut e = env(Os::Linux);
        e.home = None;
        e.xdg_data_home = Some("/d".into());
        e.xdg_cache_home = Some("/c".into());
        assert_eq!(
            AppPaths::from_env(&e).unwrap().quiver_home(),
            Path::new("/tmp/.quiver")
        );

        let mut w = env(Os::Windows);
        w.username = None;
        assert_eq!(
            AppPaths::from_env(&w).unwrap().quiver_home(),
            Path::new(r"C:\Users\unknown\Documents\.quiver")
        );
    }

    #[test]
    fn the_core_binary_is_found_only_when_installed() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(core_binary_in(dir.path(), false), None);
        std::fs::create_dir(dir.path().join("self")).unwrap();
        std::fs::write(dir.path().join("self/quiver"), b"").unwrap();
        assert_eq!(
            core_binary_in(dir.path(), false),
            Some(dir.path().join("self/quiver"))
        );
        assert_eq!(core_binary_in(dir.path(), true), None);
    }

    #[test]
    fn the_socket_lives_under_home() {
        assert_eq!(
            default_socket_path(&env(Os::Linux)),
            Path::new("/home/u/.quiver/quiver.sock")
        );
    }
}
