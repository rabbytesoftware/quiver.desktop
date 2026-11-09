//! `fetch-sidecar`: downloads the quiver.core release binary the app ships beside it.
//!
//! Honours `core-version.local` (first non-comment line pins an exact release tag, seeded from
//! `core-version.local.example` like the Makefile did); otherwise resolves the `stable` channel named in
//! `installer/core-channels.json`. Output: `target/sidecar/<triple>/quiver[.exe]`, the file name the app
//! looks for next to its executable.

use crate::args::Args;
use crate::core_version::resolve_channel;
use crate::{gh, target};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const LOCAL_PIN: &str = "core-version.local";
const LOCAL_PIN_EXAMPLE: &str = "core-version.local.example";
const CHANNELS_FILE: &str = "installer/core-channels.json";

/// Where the sidecar for `triple` is kept.
pub fn sidecar_path(triple: &str) -> PathBuf {
    Path::new("target/sidecar")
        .join(triple)
        .join(format!("quiver{}", target::exe_suffix(triple)))
}

/// The first line of the pin file that is neither a comment nor blank.
pub fn parse_pin(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
}

fn local_pin() -> Result<Option<String>> {
    let pin = Path::new(LOCAL_PIN);
    if !pin.exists() && Path::new(LOCAL_PIN_EXAMPLE).exists() {
        fs::copy(LOCAL_PIN_EXAMPLE, pin)?;
    }
    Ok(fs::read_to_string(pin).ok().and_then(|t| parse_pin(&t)))
}

fn stable_channel() -> Result<String> {
    let text =
        fs::read_to_string(CHANNELS_FILE).with_context(|| format!("reading {CHANNELS_FILE}"))?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    match json["stable"].as_str() {
        Some(channel) => Ok(channel.to_string()),
        None => bail!("`stable` missing from {CHANNELS_FILE}"),
    }
}

/// The quiver.core tag to ship: the local pin, else the newest on the stable channel.
pub fn core_tag() -> Result<String> {
    if let Some(pin) = local_pin()? {
        println!("using locally pinned quiver.core version {pin} (from {LOCAL_PIN})");
        return Ok(pin);
    }
    let channel = stable_channel()?;
    resolve_channel(&gh::release_tags(gh::CORE_REPO)?, &channel)
}

pub fn run(raw: Vec<String>) -> Result<()> {
    let args = Args::parse(raw, &["target", "tag"])?;
    let triple = match args.flag("target") {
        Some(t) => t.to_string(),
        None => target::host_triple()?,
    };
    let tag = match args.flag("tag") {
        Some(t) => t.to_string(),
        None => core_tag()?,
    };
    let asset = target::core_asset_name(&triple);
    let dest = sidecar_path(&triple);
    let dir = dest.parent().context("sidecar path has no parent")?;
    fs::create_dir_all(dir)?;
    println!("fetching quiver.core {tag} ({asset}) for {triple}");
    gh::download_asset(gh::CORE_REPO, &tag, &asset, dir)?;
    fs::rename(dir.join(&asset), &dest)?;
    make_executable(&dest)?;
    println!("sidecar ready: {}", dest.display());
    Ok(())
}

#[cfg(unix)]
pub fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pin_is_the_first_line_that_is_not_a_comment() {
        assert_eq!(
            parse_pin("# a\n\n  stable-26.5.1  \nstable-26.6.0\n"),
            Some("stable-26.5.1".into())
        );
    }

    #[test]
    fn an_all_comment_pin_file_pins_nothing() {
        assert_eq!(parse_pin("# only\n   \n#more"), None);
    }

    #[test]
    fn the_sidecar_lives_under_target_per_triple() {
        assert_eq!(
            sidecar_path("x86_64-pc-windows-msvc"),
            Path::new("target/sidecar/x86_64-pc-windows-msvc/quiver.exe")
        );
        assert_eq!(
            sidecar_path("x86_64-unknown-linux-gnu"),
            Path::new("target/sidecar/x86_64-unknown-linux-gnu/quiver")
        );
    }
}
