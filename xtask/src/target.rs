//! The platform a task builds for: the Rust target triple and the names that follow from it.

use anyhow::{bail, Context, Result};
use std::process::Command;

/// The host triple, from `rustc -vV`.
pub fn host_triple() -> Result<String> {
    let out = Command::new("rustc")
        .arg("-vV")
        .output()
        .context("running rustc -vV")?;
    let text = String::from_utf8_lossy(&out.stdout);
    match text.lines().find_map(|l| l.strip_prefix("host: ")) {
        Some(host) => Ok(host.trim().to_string()),
        None => bail!("rustc -vV printed no host line"),
    }
}

pub fn exe_suffix(triple: &str) -> &'static str {
    if triple.contains("windows") {
        ".exe"
    } else {
        ""
    }
}

/// The quiver.core release asset that runs on `triple`.
pub fn core_asset_name(triple: &str) -> String {
    match triple {
        "aarch64-apple-darwin" => "quiver-darwin-arm64".into(),
        "x86_64-apple-darwin" => "quiver-darwin-amd64".into(),
        "x86_64-unknown-linux-gnu" => "quiver-linux-amd64".into(),
        "aarch64-unknown-linux-gnu" => "quiver-linux-arm64".into(),
        "x86_64-pc-windows-msvc" => "quiver-windows-amd64.exe".into(),
        other => format!("quiver-{other}"),
    }
}

/// Name of the Debian architecture / AppImage arch label for a triple.
pub fn deb_arch(triple: &str) -> &'static str {
    if triple.starts_with("aarch64") {
        "arm64"
    } else {
        "amd64"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_assets_follow_the_release_naming() {
        assert_eq!(
            core_asset_name("aarch64-apple-darwin"),
            "quiver-darwin-arm64"
        );
        assert_eq!(
            core_asset_name("x86_64-pc-windows-msvc"),
            "quiver-windows-amd64.exe"
        );
        assert_eq!(
            core_asset_name("riscv64-unknown-linux-gnu"),
            "quiver-riscv64-unknown-linux-gnu"
        );
    }

    #[test]
    fn only_windows_triples_carry_an_exe_suffix() {
        assert_eq!(exe_suffix("aarch64-pc-windows-msvc"), ".exe");
        assert_eq!(exe_suffix("x86_64-apple-darwin"), "");
    }
}
