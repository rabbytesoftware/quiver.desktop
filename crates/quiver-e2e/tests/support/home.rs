//! One wiped home per test, laid out the way the app and quiver.core agree on it.
//!
//! HOME, not QUIVER_HOME, is what moves: the app dials `$HOME/.quiver/quiver.sock` while core resolves its own
//! default under QUIVER_HOME, and they agree only while QUIVER_HOME is left at `$HOME/.quiver`.

use std::path::{Path, PathBuf};

use super::env::{optional, Env};

pub struct Home {
    pub root: PathBuf,
}

impl Home {
    /// `<QUIVER_E2E_TMP>/<name>`, deleted and recreated: the "clean QUIVER_HOME" every test starts from.
    pub fn fresh(env: &Env, name: &str) -> Self {
        let root = env.tmp.join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".quiver")).expect("create the test home");
        let home = Self { root };
        // The version check is throttled to one remote call per arrow per hour, which no scenario that publishes
        // a release a minute in can live with.
        if let Some(ttl) = optional("QUIVER_E2E_VERSION_CHECK_TTL") {
            std::fs::write(
                home.quiver().join("config.yaml"),
                format!("config:\n  arrows:\n    version_check_ttl: {ttl}\n"),
            )
            .expect("write config.yaml");
        }
        home
    }

    /// Makes the app list Quiver's own components (quiver.core, quiver.desktop) in its sidebar and grids.
    ///
    /// They are hidden by default and the only switch for it is Settings > Developer, which exists in debug builds
    /// only, while the box runs the release profile (the production address of the daemon). So the preference the
    /// switch would save (`quiver.catalogVisibility` in `<data dir>/state.json`) is written before the app starts.
    pub fn show_own_components(&self) {
        let data = self.root.join(".local/share/com.rabbytesoftware.quiver");
        std::fs::create_dir_all(&data).expect("create the app data dir");
        std::fs::write(
            data.join("state.json"),
            r#"{"quiver.catalogVisibility":"true"}"#,
        )
        .expect("seed the preference");
    }

    pub fn quiver(&self) -> PathBuf {
        self.root.join(".quiver")
    }

    pub fn socket(&self) -> PathBuf {
        self.quiver().join("quiver.sock")
    }

    /// quiver.core's stable self-install path (`paths.self`), where the running binary promotes itself on boot.
    pub fn self_binary(&self) -> PathBuf {
        self.quiver().join("self").join("quiver")
    }

    /// The daemon's own log file, one JSON record per line.
    pub fn daemon_log(&self) -> PathBuf {
        self.quiver().join("logs").join("Quiver.log")
    }

    /// The environment that points an app (or a CLI) at this home.
    pub fn env(&self) -> Vec<(String, String)> {
        vec![
            ("HOME".into(), self.root.display().to_string()),
            ("QUIVER_HOME".into(), self.quiver().display().to_string()),
        ]
    }
}

impl AsRef<Path> for Home {
    fn as_ref(&self) -> &Path {
        &self.root
    }
}
