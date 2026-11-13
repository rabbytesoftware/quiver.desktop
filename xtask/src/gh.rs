//! The GitHub CLI calls the release tasks make (`gh` is how the old scripts talked to GitHub, too).

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub const CORE_REPO: &str = "rabbytesoftware/quiver.core";
pub const DESKTOP_REPO: &str = "rabbytesoftware/quiver.desktop";

fn gh(args: &[&str]) -> Result<String> {
    let out = Command::new("gh")
        .args(args)
        .output()
        .context("gh not found: install the GitHub CLI (https://cli.github.com)")?;
    if !out.status.success() {
        bail!(
            "gh {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The newest 100 release tags of a repository.
pub fn release_tags(repo: &str) -> Result<Vec<String>> {
    let json = gh(&[
        "release", "list", "--repo", repo, "--json", "tagName", "-L", "100",
    ])?;
    let releases: Vec<serde_json::Value> =
        serde_json::from_str(&json).context("gh release list output")?;
    Ok(releases
        .iter()
        .filter_map(|r| r["tagName"].as_str().map(str::to_string))
        .collect())
}

pub fn release_notes(
    repo: &str,
    tag: &str,
) -> Result<String> {
    gh(&[
        "release", "view", tag, "--repo", repo, "--json", "body", "--jq", ".body",
    ])
}

/// Downloads the single asset `pattern` of release `tag` into `dir`, replacing a file already there.
pub fn download_asset(
    repo: &str,
    tag: &str,
    pattern: &str,
    dir: &Path,
) -> Result<()> {
    let dir = dir.to_str().context("non-utf8 download directory")?;
    gh(&[
        "release",
        "download",
        tag,
        "--repo",
        repo,
        "--pattern",
        pattern,
        "--dir",
        dir,
        "--clobber",
    ])?;
    Ok(())
}
