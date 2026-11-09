//! `verify-update --from <old bundle> --to <new bundle>`: the file-level half of "update from the previous
//! release", runnable without a display (Linux AppImage or the `.tar.gz` fallback).
//!
//! It proves, for two real `cargo xtask package linux` outputs: both unpack to the expected layout
//! (`quiverdesktop`, `quiver-arrow-window`, sidecar `quiver`), neither contains the e2e bridge, the two
//! builds actually differ, and the `ARROW.md` Linux update step (replace `Quiver.AppImage` in the XDG data
//! dir, keep it executable) turns the old install into byte-for-byte the new one. It does NOT start the app
//! or drive the in-app update; that is the box scenario (docs/release-checklist.md).

use crate::args::Args;
use crate::checksums::sha256_hex;
use crate::package_layout::EXPECTED_FILES;
use crate::release_artifact::check_files;
use anyhow::{bail, ensure, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Unpacks `bundle` into `dest` and returns the directory holding `usr/bin`.
fn unpack(bundle: &Path, dest: &Path) -> Result<PathBuf> {
    fs::create_dir_all(dest)?;
    let name = bundle
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let abs = fs::canonicalize(bundle).with_context(|| format!("bundle {}", bundle.display()))?;
    let status = if name.ends_with(".appimage") {
        crate::sidecar::make_executable(&abs)?;
        Command::new(&abs)
            .arg("--appimage-extract")
            .current_dir(dest)
            .status()?
    } else if name.ends_with(".tar.gz") {
        Command::new("tar")
            .arg("-xzf")
            .arg(&abs)
            .current_dir(dest)
            .status()?
    } else {
        bail!(
            "{}: only .AppImage and .tar.gz bundles can be unpacked here",
            bundle.display()
        );
    };
    ensure!(status.success(), "unpacking {} failed", bundle.display());
    ["squashfs-root", "Quiver.AppDir"]
        .iter()
        .map(|d| dest.join(d))
        .find(|d| d.is_dir())
        .with_context(|| format!("{} unpacked to no known directory", bundle.display()))
}

fn inspect(bundle: &Path, work: &Path, label: &str) -> Result<String> {
    let root = unpack(bundle, &work.join(label))?;
    for file in EXPECTED_FILES {
        ensure!(
            root.join("usr/bin").join(file).is_file(),
            "{label}: usr/bin/{file} is missing"
        );
    }
    let app = root.join("usr/bin/quiverdesktop");
    check_files(&[
        app.as_path(),
        root.join("usr/bin/quiver-arrow-window").as_path(),
    ])?;
    Ok(sha256_hex(&fs::read(&app)?))
}

/// The `ARROW.md` update step: place the new asset over `Quiver.AppImage` and make it executable.
fn replace_install(data_home: &Path, from: &Path, to: &Path) -> Result<()> {
    let dir = data_home.join("Quiver");
    fs::create_dir_all(&dir)?;
    let target = dir.join("Quiver.AppImage");
    fs::copy(from, &target)?;
    ensure!(
        fs::read(&target)? == fs::read(from)?,
        "the old bundle was not installed intact"
    );
    let staged = dir.join("Quiver.AppImage.new");
    fs::copy(to, &staged)?;
    crate::sidecar::make_executable(&staged)?;
    fs::rename(&staged, &target)?;
    ensure!(
        fs::read(&target)? == fs::read(to)?,
        "the update did not leave the new bundle in place"
    );
    Ok(())
}

pub fn run(raw: Vec<String>) -> Result<()> {
    let args = Args::parse(raw, &["from", "to", "workdir"])?;
    let (Some(from), Some(to)) = (args.flag("from"), args.flag("to")) else {
        bail!("usage: cargo xtask verify-update --from <old bundle> --to <new bundle> [--workdir dir]");
    };
    let work = PathBuf::from(args.flag("workdir").unwrap_or("target/verify-update"));
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    let (old_app, new_app) = (
        inspect(Path::new(from), &work, "from")?,
        inspect(Path::new(to), &work, "to")?,
    );
    ensure!(
        old_app != new_app,
        "both bundles contain the same quiverdesktop: not an update"
    );
    println!(
        "layout ok, no e2e bridge; quiverdesktop {} -> {}",
        &old_app[..12],
        &new_app[..12]
    );
    if from.to_ascii_lowercase().ends_with(".appimage")
        && to.to_ascii_lowercase().ends_with(".appimage")
    {
        replace_install(&work.join("data"), Path::new(from), Path::new(to))?;
        println!(
            "update placement ok ({}/data/Quiver/Quiver.AppImage is now the new build)",
            work.display()
        );
    }
    println!("verify-update: ok (file level only; run the box scenario for the in-app update)");
    Ok(())
}
