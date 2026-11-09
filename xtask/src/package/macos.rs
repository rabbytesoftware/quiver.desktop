//! macOS: `Quiver.app` (optionally universal) inside a `.dmg`, signed and notarized when the Apple secrets
//! are present (the logic of the old `scripts/build-macos-dmg.sh`, minus the Tauri bundler).
//!
//! Signing is opt-in through the environment, exactly as before:
//! * `APPLE_SIGNING_IDENTITY` ("Developer ID Application: Name (TEAMID)") turns signing + hardened runtime on.
//!   The certificate must already be in a keychain (the CI workflow imports `APPLE_CERTIFICATE`).
//! * `APPLE_ID` / `APPLE_PASSWORD` (app-specific) / `APPLE_TEAM_ID` turn notarization + stapling on.
//!
//! Without an identity the bundle is ad-hoc signed (Apple silicon refuses unsigned code).

use super::command::{run, run_with_paths};
use super::inputs::{copy_pieces, Inputs, Piece, IDENTIFIER, PRODUCT};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const INFO_PLIST: &str = include_str!("../../../installer/macos/Info.plist.in");
const ICON: &str = "installer/icons/icon.icns";
const ENTITLEMENTS: &str = "installer/macos/Entitlements.plist";

pub fn info_plist(version: &str) -> String {
    INFO_PLIST.replace("{{VERSION}}", version)
}

/// Builds the .dmg. With `universal`, `inputs` is the arm64 build and `x86_64` the Intel one; each binary
/// is merged with `lipo`.
pub fn package(inputs: &Inputs, x86_64: Option<&Inputs>) -> Result<Vec<PathBuf>> {
    let stage = inputs.out.join("macos-stage");
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    let app = stage.join(format!("{PRODUCT}.app"));
    let macos_dir = app.join("Contents/MacOS");
    let pieces = inputs.pieces()?;
    match x86_64 {
        None => copy_pieces(&pieces, &macos_dir)?,
        Some(intel) => lipo_pieces(&pieces, &intel.pieces()?, &macos_dir)?,
    }
    let resources = app.join("Contents/Resources");
    fs::create_dir_all(&resources)?;
    fs::copy(ICON, resources.join("icon.icns")).with_context(|| format!("copying {ICON}"))?;
    fs::write(app.join("Contents/Info.plist"), info_plist(&inputs.version))?;

    let identity = std::env::var("APPLE_SIGNING_IDENTITY")
        .ok()
        .filter(|v| !v.is_empty());
    sign_app(&app, identity.as_deref())?;

    link_applications(&stage)?;
    let dmg = inputs.out.join(format!(
        "{PRODUCT}_{}_{}.dmg",
        inputs.version,
        arch_label(x86_64.is_some(), &inputs.triple)
    ));
    if dmg.exists() {
        fs::remove_file(&dmg)?;
    }
    run_with_paths(
        "hdiutil",
        &[
            "create",
            "-volname",
            PRODUCT,
            "-ov",
            "-format",
            "UDZO",
            "-srcfolder",
        ],
        &[&stage, &dmg],
    )?;
    if let Some(identity) = identity.as_deref() {
        run_with_paths(
            "codesign",
            &["--force", "--timestamp", "--sign", identity],
            &[&dmg],
        )?;
        notarize(&dmg)?;
    }
    Ok(vec![dmg])
}

fn arch_label(universal: bool, triple: &str) -> &str {
    if universal {
        "universal"
    } else if triple.starts_with("aarch64") {
        "aarch64"
    } else {
        "x64"
    }
}

fn lipo_pieces(arm: &[Piece], intel: &[Piece], dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    for (a, i) in arm.iter().zip(intel) {
        let out = dir.join(&a.installed_as);
        run_with_paths(
            "lipo",
            &["-create", "-output"],
            &[&out, &a.source, &i.source],
        )?;
    }
    Ok(())
}

/// Nested code first, then the bundle (codesign --deep is deprecated for this).
fn sign_app(app: &Path, identity: Option<&str>) -> Result<()> {
    let macos_dir = app.join("Contents/MacOS");
    let mut binaries: Vec<PathBuf> = fs::read_dir(&macos_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    binaries.sort();
    match identity {
        Some(identity) => {
            for binary in &binaries {
                run_with_paths(
                    "codesign",
                    &[
                        "--force",
                        "--timestamp",
                        "--options",
                        "runtime",
                        "--entitlements",
                        ENTITLEMENTS,
                        "--sign",
                        identity,
                    ],
                    &[binary],
                )?;
            }
            run_with_paths(
                "codesign",
                &[
                    "--force",
                    "--timestamp",
                    "--options",
                    "runtime",
                    "--entitlements",
                    ENTITLEMENTS,
                    "--sign",
                    identity,
                ],
                &[app],
            )?;
            run_with_paths("codesign", &["--verify", "--strict", "--verbose=2"], &[app])
        }
        None => {
            println!("no APPLE_SIGNING_IDENTITY: ad-hoc signing ({IDENTIFIER})");
            for binary in &binaries {
                run_with_paths("codesign", &["--force", "--sign", "-"], &[binary])?;
            }
            run_with_paths("codesign", &["--force", "--sign", "-"], &[app])
        }
    }
}

fn notarize(dmg: &Path) -> Result<()> {
    let (id, password, team) = (env("APPLE_ID"), env("APPLE_PASSWORD"), env("APPLE_TEAM_ID"));
    let (Some(id), Some(password), Some(team)) = (id, password, team) else {
        println!("APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID not all set: signed but NOT notarized");
        return Ok(());
    };
    run_with_paths(
        "xcrun",
        &[
            "notarytool",
            "submit",
            "--wait",
            "--apple-id",
            &id,
            "--password",
            &password,
            "--team-id",
            &team,
        ],
        &[dmg],
    )?;
    run_with_paths("xcrun", &["stapler", "staple"], &[dmg])?;
    run(
        "xcrun",
        &[
            "stapler",
            "validate",
            dmg.to_str().context("non-utf8 path")?,
        ],
    )
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

#[cfg(unix)]
fn link_applications(stage: &Path) -> Result<()> {
    std::os::unix::fs::symlink("/Applications", stage.join("Applications"))?;
    Ok(())
}

#[cfg(not(unix))]
fn link_applications(_stage: &Path) -> Result<()> {
    anyhow::bail!("macOS bundles can only be built on macOS")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plist_carries_the_bundle_identity_and_version() {
        let plist = info_plist("1.2.3");
        assert!(plist.contains("<string>1.2.3</string>"));
        assert!(plist.contains("<key>CFBundleExecutable</key><string>quiverdesktop</string>"));
        assert!(plist.contains("<string>com.rabbytesoftware.quiver</string>"));
        assert!(!plist.contains("{{VERSION}}"));
    }

    #[test]
    fn the_dmg_is_labelled_by_architecture() {
        assert_eq!(arch_label(true, "aarch64-apple-darwin"), "universal");
        assert_eq!(arch_label(false, "aarch64-apple-darwin"), "aarch64");
        assert_eq!(arch_label(false, "x86_64-apple-darwin"), "x64");
    }
}
