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
/// The Icon Composer source of the macOS 26 adaptive icon: the light look, the dark look (a yellow arrow) and
/// the tinted one. Compiled by `actool` into `Assets.car`; without it macOS draws its own dark version of
/// `icon.icns` (a black arrow).
const ADAPTIVE_ICON_SOURCE: &str = "installer/icons/quiver.icon";
/// The name `actool` gives the icon in `Assets.car`. It must equal the `.icon` bundle's stem and the
/// `CFBundleIconName` in `Info.plist.in`, or macOS ignores the car and falls back to the plain `.icns`.
const ADAPTIVE_ICON_NAME: &str = "quiver";
/// Set (to anything) to build without the adaptive icon, on a machine whose Xcode cannot compile it.
const PLAIN_ICON_ENV: &str = "QUIVER_PLAIN_MAC_ICON";
const ENTITLEMENTS: &str = "installer/macos/Entitlements.plist";

pub fn info_plist(version: &str) -> String {
    INFO_PLIST.replace("{{VERSION}}", version)
}

/// Builds the .dmg. With `universal`, `inputs` is the arm64 build and `x86_64` the Intel one; each binary
/// is merged with `lipo`.
pub fn package(
    inputs: &Inputs,
    x86_64: Option<&Inputs>,
) -> Result<Vec<PathBuf>> {
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
    compile_adaptive_icon(&inputs.out, &resources)?;
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

/// Compiles `quiver.icon` into `Resources/Assets.car` (the old build did this in `icons-compiler/`).
///
/// A failure fails the build: shipping only the plain `.icns` looks right on macOS 15 and wrong on 26 (the
/// system invents a dark version with a black arrow), so a release must not do it silently. Needs Xcode
/// 26.4 or newer (26.3's `actool` crashes on this file); `QUIVER_PLAIN_MAC_ICON=1` opts out for a local build.
fn compile_adaptive_icon(
    out: &Path,
    resources: &Path,
) -> Result<()> {
    if std::env::var_os(PLAIN_ICON_ENV).is_some_and(|v| !v.is_empty()) {
        println!("warning: {PLAIN_ICON_ENV} is set: the app ships without its adaptive (dark/tinted) icon");
        return Ok(());
    }
    let work = out.join("actool");
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    fs::create_dir_all(&work)?;
    let compiled = work.to_str().context("non-utf8 path")?;
    let partial = work.join("partial.plist");
    let partial = partial.to_str().context("non-utf8 path")?;
    run(
        "xcrun",
        &[
            "actool",
            ADAPTIVE_ICON_SOURCE,
            "--compile",
            compiled,
            "--output-partial-info-plist",
            partial,
            "--app-icon",
            ADAPTIVE_ICON_NAME,
            "--include-all-app-icons",
            "--enable-on-demand-resources",
            "NO",
            "--development-region",
            "en",
            "--target-device",
            "mac",
            "--minimum-deployment-target",
            "11.0",
            "--platform",
            "macosx",
        ],
    )
    .with_context(|| {
        format!(
            "compiling {ADAPTIVE_ICON_SOURCE} needs Xcode 26.4+ (set {PLAIN_ICON_ENV}=1 to build without it)"
        )
    })?;
    let car = work.join("Assets.car");
    anyhow::ensure!(
        car.is_file(),
        "actool wrote no Assets.car in {}",
        work.display()
    );
    fs::copy(&car, resources.join("Assets.car")).context("copying Assets.car")?;
    Ok(())
}

fn arch_label(
    universal: bool,
    triple: &str,
) -> &str {
    if universal {
        "universal"
    } else if triple.starts_with("aarch64") {
        "aarch64"
    } else {
        "x64"
    }
}

fn lipo_pieces(
    arm: &[Piece],
    intel: &[Piece],
    dir: &Path,
) -> Result<()> {
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
fn sign_app(
    app: &Path,
    identity: Option<&str>,
) -> Result<()> {
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

    /// The dark icon was lost once in the port (the old build compiled `quiver.icon`; the new one did not, and
    /// macOS then drew a black-arrow dark icon). These keep the three pieces tied together without a Mac.
    #[test]
    fn the_plist_names_the_adaptive_icon_the_source_provides() {
        let plist = info_plist("1.2.3");
        assert!(plist.contains(&format!(
            "<key>CFBundleIconName</key><string>{ADAPTIVE_ICON_NAME}</string>"
        )));
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(ADAPTIVE_ICON_SOURCE);
        assert_eq!(
            source.file_stem().and_then(|s| s.to_str()),
            Some(ADAPTIVE_ICON_NAME),
            "the .icon bundle's name is the icon's name in Assets.car"
        );
        let json = fs::read_to_string(source.join("icon.json")).unwrap();
        assert!(
            json.contains("\"appearance\" : \"dark\""),
            "the source must define a dark look (the yellow arrow)"
        );
    }

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
