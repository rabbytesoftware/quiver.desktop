//! `cargo xtask package [macos|windows|linux]`: assembles the per-OS release bundle in `target/bundle/`.
//!
//! Inputs: a `cargo build --release` of `quiver-app` WITHOUT feature `e2e-bridge`, and the sidecar from
//! `cargo xtask fetch-sidecar`. Flags: `--version`, `--target <triple>`, `--bin-dir`, `--out`; for macOS
//! `--universal` merges `target/aarch64-apple-darwin/release` and `target/x86_64-apple-darwin/release`.
//! The app binary is checked with `check-release-artifact` before they are bundled.

mod command;
mod deb;
mod inputs;
mod linux;
mod macos;
mod windows;

use crate::args::Args;
use anyhow::{bail, Result};
use inputs::Inputs;
use std::path::PathBuf;

const VALUED: &[&str] = &["version", "target", "bin-dir", "out"];

fn host_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(windows) {
        "windows"
    } else {
        "linux"
    }
}

pub fn run(raw: Vec<String>) -> Result<()> {
    let args = Args::parse(raw, VALUED)?;
    let os = args.positional.first().map_or(host_os(), String::as_str);
    let version = args.flag("version");
    let out = args.flag("out");
    let outputs: Vec<PathBuf> = match os {
        "macos" if args.has("universal") => {
            let arm = Inputs::resolve(version, Some("aarch64-apple-darwin"), None, out)?;
            let intel = Inputs::resolve(version, Some("x86_64-apple-darwin"), None, out)?;
            macos::package(&arm, Some(&intel))?
        }
        "macos" => macos::package(
            &Inputs::resolve(version, args.flag("target"), args.flag("bin-dir"), out)?,
            None,
        )?,
        "windows" => windows::package(&Inputs::resolve(
            version,
            args.flag("target"),
            args.flag("bin-dir"),
            out,
        )?)?,
        "linux" => linux::package(&Inputs::resolve(
            version,
            args.flag("target"),
            args.flag("bin-dir"),
            out,
        )?)?,
        other => bail!("unknown os `{other}` (macos, windows, linux)"),
    };
    for output in &outputs {
        println!("bundle: {}", output.display());
    }
    Ok(())
}
