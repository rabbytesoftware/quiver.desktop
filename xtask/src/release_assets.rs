//! `check-release-assets <dir> [--arrow ARROW.md]`: the assembled release must be installable by the
//! updater: one asset per platform suffix `quiver-release` selects on (`.AppImage`, `.dmg`, `-setup.exe`), a
//! `checksums.txt` that covers every bundle, and an `ARROW.md` that still takes the two variables the app
//! supplies (`QUIVER_RELEASE_ASSET_URL`, `QUIVER_RELEASE_CHECKSUM`).

use crate::args::Args;
use crate::checksums::MANIFEST;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

const PLATFORM_SUFFIXES: &[(&str, &str)] = &[
    ("linux", ".appimage"),
    ("macos", ".dmg"),
    ("windows", "-setup.exe"),
];
const ARROW_VARIABLES: &[&str] = &["QUIVER_RELEASE_ASSET_URL", "QUIVER_RELEASE_CHECKSUM"];

/// Problems found in a release made of `names` (file names) with the given checksum manifest and ARROW.md.
pub fn problems(
    names: &[String],
    manifest: Option<&str>,
    arrow: Option<&str>,
) -> Vec<String> {
    let mut found = Vec::new();
    for (os, suffix) in PLATFORM_SUFFIXES {
        let count = names
            .iter()
            .filter(|n| n.to_ascii_lowercase().ends_with(suffix))
            .count();
        if count == 0 {
            found.push(format!("no *{suffix} asset for {os}"));
        }
    }
    match manifest {
        None => found.push(format!("no {MANIFEST}")),
        Some(text) => {
            for name in names.iter().filter(|n| {
                PLATFORM_SUFFIXES
                    .iter()
                    .any(|(_, s)| n.to_ascii_lowercase().ends_with(s))
            }) {
                let listed = text.lines().any(|l| {
                    l.split_whitespace()
                        .last()
                        .is_some_and(|f| f.trim_start_matches("./") == name)
                });
                if !listed {
                    found.push(format!("{MANIFEST} does not list {name}"));
                }
            }
        }
    }
    match arrow {
        None => found.push("ARROW.md not found".into()),
        Some(text) => {
            for var in ARROW_VARIABLES {
                if !text.contains(var) {
                    found.push(format!("ARROW.md no longer declares {var}"));
                }
            }
        }
    }
    found
}

pub fn run(raw: Vec<String>) -> Result<()> {
    let args = Args::parse(raw, &["arrow"])?;
    let dir = args
        .positional
        .first()
        .context("usage: cargo xtask check-release-assets <dir> [--arrow ARROW.md]")?;
    let names: Vec<String> = fs::read_dir(dir)?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    let manifest = fs::read_to_string(Path::new(dir).join(MANIFEST)).ok();
    let arrow = fs::read_to_string(args.flag("arrow").unwrap_or("ARROW.md")).ok();
    let found = problems(&names, manifest.as_deref(), arrow.as_deref());
    if !found.is_empty() {
        bail!("release is not installable:\n  {}", found.join("\n  "));
    }
    println!("check-release-assets: ok ({} files)", names.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    const FULL: &[&str] = &[
        "Quiver_1.0.0_amd64.AppImage",
        "Quiver_1.0.0_universal.dmg",
        "Quiver_1.0.0_x64-setup.exe",
        "checksums.txt",
    ];
    const ARROW: &str = "QUIVER_RELEASE_ASSET_URL QUIVER_RELEASE_CHECKSUM";

    fn manifest() -> String {
        FULL.iter()
            .take(3)
            .map(|n| format!("{}  {n}\n", "0".repeat(64)))
            .collect()
    }

    #[test]
    fn a_complete_release_has_no_problems() {
        assert!(problems(&names(FULL), Some(&manifest()), Some(ARROW)).is_empty());
    }

    #[test]
    fn a_missing_platform_and_manifest_entry_are_reported() {
        let partial = names(&["Quiver_1.0.0_amd64.AppImage", "Quiver_1.0.0_universal.dmg"]);
        let p = problems(
            &partial,
            Some("00  Quiver_1.0.0_amd64.AppImage\n"),
            Some(ARROW),
        );
        assert!(p.iter().any(|m| m.contains("-setup.exe")), "{p:?}");
        assert!(
            p.iter()
                .any(|m| m.contains("does not list Quiver_1.0.0_universal.dmg")),
            "{p:?}"
        );
    }

    #[test]
    fn an_arrow_without_the_app_supplied_variables_is_reported() {
        let p = problems(&names(FULL), Some(&manifest()), Some("nothing"));
        assert_eq!(p.len(), 2, "{p:?}");
    }
}
