//! The box's stand-in for github.com (docker/e2e/fixtures/upstream.py) keeps everything it serves under one state
//! directory. A scenario publishes a release by writing there, as docker/e2e/scenarios/lib.sh does for the shell
//! scenarios: the daemon under test still reaches it over HTTPS by the real hostname and still verifies what it
//! downloads.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::env::{optional, required};

pub struct Upstream {
    state: PathBuf,
    core_manifest: Option<PathBuf>,
    core_assets: Option<PathBuf>,
    core_asset: Option<String>,
    log: Option<PathBuf>,
}

const CORE_REPO: &str = "rabbytesoftware/quiver.core";

impl Upstream {
    pub fn load() -> Self {
        Self {
            state: PathBuf::from(required("QUIVER_E2E_UPSTREAM_STATE")),
            core_manifest: optional("QUIVER_E2E_CORE_MANIFEST").map(PathBuf::from),
            core_assets: optional("QUIVER_E2E_CORE_ASSETS").map(PathBuf::from),
            core_asset: optional("QUIVER_E2E_CORE_ASSET"),
            log: optional("QUIVER_E2E_UPSTREAM_LOG").map(PathBuf::from),
        }
    }

    /// The quiver.core versions the box built, oldest first (`QUIVER_E2E_CORE_TAGS`).
    pub fn core_tags() -> Vec<String> {
        required("QUIVER_E2E_CORE_TAGS")
            .split_whitespace()
            .map(String::from)
            .collect()
    }

    fn bare(&self, repo: &str) -> PathBuf {
        self.state.join("git").join(format!("{repo}.git"))
    }

    /// Tags the repository's HEAD, which is what quiver.core's version check reads through ls-remote.
    fn tag(&self, repo: &str, name: &str) {
        let status = Command::new("git")
            .arg("--git-dir")
            .arg(self.bare(repo))
            .args(["tag", "-f", name, "HEAD"])
            .output()
            .expect("run git");
        assert!(
            status.status.success(),
            "git tag {repo} {name}: {}",
            String::from_utf8_lossy(&status.stderr)
        );
    }

    /// Publishes quiver.core `tag`: the tag, the manifest at it, the binary and its `checksums.txt`, and the
    /// "latest release" pointer. Returns the published asset.
    pub fn publish_core_release(&self, tag: &str) -> PathBuf {
        let asset_name = self.core_asset.clone().expect("QUIVER_E2E_CORE_ASSET");
        let raw = self.state.join("raw").join(CORE_REPO).join(tag);
        std::fs::create_dir_all(&raw).expect("raw dir");
        std::fs::copy(
            self.core_manifest
                .as_ref()
                .expect("QUIVER_E2E_CORE_MANIFEST"),
            raw.join("ARROW.md"),
        )
        .expect("copy manifest");

        let release = self.state.join("releases").join(CORE_REPO).join(tag);
        let _ = std::fs::remove_dir_all(&release);
        std::fs::create_dir_all(&release).expect("release dir");
        let asset = release.join(&asset_name);
        let built = self
            .core_assets
            .as_ref()
            .expect("QUIVER_E2E_CORE_ASSETS")
            .join(tag)
            .join(&asset_name);
        std::fs::copy(&built, &asset).unwrap_or_else(|e| panic!("copy {}: {e}", built.display()));
        let sum = sha256(&asset);
        std::fs::write(
            release.join("checksums.txt"),
            format!("{sum}  ./{asset_name}\n"),
        )
        .expect("checksums");
        std::fs::write(
            self.state.join("releases").join(CORE_REPO).join("LATEST"),
            format!("{tag}\n"),
        )
        .expect("LATEST");
        self.tag(CORE_REPO, tag);
        asset
    }

    /// A new tag of a small fixture repository, serving the manifest it already serves at `develop`, so a row that
    /// follows the repository's `stable` channel sees a newer release ahead of what it has.
    pub fn publish_repo_tag(&self, repo: &str, tag: &str) {
        let from = self
            .state
            .join("raw")
            .join(repo)
            .join("develop")
            .join("ARROW.md");
        let to = self.state.join("raw").join(repo).join(tag);
        std::fs::create_dir_all(&to).expect("raw tag dir");
        std::fs::copy(from, to.join("ARROW.md")).expect("copy manifest");
        self.tag(repo, tag);
    }

    /// How many times the stand-in was asked for `request_path` (the request log is one JSON object per line).
    pub fn hits(&self, request_path: &str) -> usize {
        let Some(log) = &self.log else { return 0 };
        std::fs::read_to_string(log)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|v| v["path"] == request_path)
            .count()
    }
}

fn sha256(file: &Path) -> String {
    let out = Command::new("sha256sum")
        .arg(file)
        .output()
        .expect("sha256sum");
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}
