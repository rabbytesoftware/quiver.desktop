//! `checksums <dir>`: writes `checksums.txt` (sha256sum format) for the release assets in a directory.
//!
//! The Rust updater (`quiver-release`) finds a manifest called `checksums.txt` when the release page shows
//! no per-asset digest, and reads `<hex>  <name>` lines.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

pub const MANIFEST: &str = "checksums.txt";
const ASSET_SUFFIXES: &[&str] = &[".appimage", ".deb", ".dmg", "-setup.exe", ".tar.gz"];

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn is_asset(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ASSET_SUFFIXES.iter().any(|s| lower.ends_with(s))
}

pub fn run(args: Vec<String>) -> Result<()> {
    let Some(dir) = args.first() else {
        bail!("usage: cargo xtask checksums <dir>")
    };
    let mut names: Vec<String> = fs::read_dir(dir)?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| is_asset(n))
        .collect();
    names.sort();
    if names.is_empty() {
        bail!("no release assets in {dir}");
    }
    let mut manifest = String::new();
    for name in &names {
        let bytes =
            fs::read(Path::new(dir).join(name)).with_context(|| format!("reading {name}"))?;
        let _ = writeln!(manifest, "{}  {name}", sha256_hex(&bytes));
    }
    fs::write(Path::new(dir).join(MANIFEST), &manifest)?;
    print!("{manifest}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_the_known_empty_digest() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn only_release_bundles_are_listed() {
        assert!(is_asset("Quiver_0.1.0_amd64.AppImage") && is_asset("Quiver_0.1.0_x64-setup.exe"));
        assert!(!is_asset("checksums.txt") && !is_asset("notes.md"));
    }

    #[test]
    fn the_manifest_uses_the_sha256sum_format() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Quiver_1_universal.dmg"), b"").unwrap();
        run(vec![tmp.path().to_str().unwrap().into()]).unwrap();
        let text = fs::read_to_string(tmp.path().join(MANIFEST)).unwrap();
        assert_eq!(text, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  Quiver_1_universal.dmg\n");
    }
}
