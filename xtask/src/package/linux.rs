//! Linux: `Quiver_<version>_<arch>.AppImage` (the asset the updater selects and `ARROW.md` installs to
//! `~/.local/share/Quiver/Quiver.AppImage`), a `.tar.gz` of the same AppDir when `appimagetool` is not
//! available, and a `.deb`.
//!
//! The AppImage is NOT self-contained: it carries the three binaries but relies on the host's system
//! libraries (X11/Wayland, Vulkan, libxkbcommon, and WebKitGTK for arrow apps), as the `.deb` declares.
//! The Tauri build bundled libraries with linuxdeploy; porting that is not done.

use super::command::{exists_on_path, run, run_with_paths};
use super::deb;
use super::inputs::{copy_pieces, Inputs, INSTALLED_APP, PRODUCT};
use crate::target::deb_arch;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

const DESKTOP_ENTRY: &str = include_str!("../../../installer/linux/quiver.desktop");
const ICON: &str = "installer/icons/icon.png";

pub fn appimage_name(version: &str, arch: &str) -> String {
    format!("{PRODUCT}_{version}_{arch}.AppImage")
}

fn app_run() -> String {
    format!("#!/bin/sh\nHERE=\"$(dirname \"$(readlink -f \"$0\")\")\"\nexec \"$HERE/usr/bin/{INSTALLED_APP}\" \"$@\"\n")
}

pub fn package(inputs: &Inputs) -> Result<Vec<PathBuf>> {
    let pieces = inputs.pieces()?;
    let arch = deb_arch(&inputs.triple);
    let appdir = inputs.out.join("linux-stage/Quiver.AppDir");
    if appdir.exists() {
        fs::remove_dir_all(&appdir)?;
    }
    copy_pieces(&pieces, &appdir.join("usr/bin"))?;
    fs::write(appdir.join("quiver.desktop"), DESKTOP_ENTRY)?;
    fs::copy(ICON, appdir.join("quiver.png")).with_context(|| format!("copying {ICON}"))?;
    let apprun = appdir.join("AppRun");
    fs::write(&apprun, app_run())?;
    crate::sidecar::make_executable(&apprun)?;

    let mut outputs = Vec::new();
    let image = inputs.out.join(appimage_name(&inputs.version, arch));
    let tool = std::env::var("APPIMAGETOOL").unwrap_or_else(|_| "appimagetool".into());
    if exists_on_path(&tool) {
        // The tool is itself an AppImage; extract-and-run avoids needing FUSE (CI, containers).
        std::env::set_var("APPIMAGE_EXTRACT_AND_RUN", "1");
        let machine = if arch == "arm64" { "aarch64" } else { "x86_64" };
        std::env::set_var("ARCH", machine);
        run_with_paths(&tool, &[], &[&appdir, &image])?;
        outputs.push(image);
    } else {
        println!("appimagetool not found (set APPIMAGETOOL): writing a .tar.gz of the AppDir instead; NO .AppImage, so this output cannot be used for the updater");
        let tarball = inputs
            .out
            .join(format!("{PRODUCT}_{}_{arch}.tar.gz", inputs.version));
        let parent = appdir.parent().context("appdir has no parent")?;
        let (tar_arg, parent_arg) = (
            tarball.to_str().context("non-utf8 path")?,
            parent.to_str().context("non-utf8 path")?,
        );
        run("tar", &["-czf", tar_arg, "-C", parent_arg, "Quiver.AppDir"])?;
        outputs.push(tarball);
    }
    if exists_on_path("dpkg-deb") {
        outputs.push(deb::build(inputs, &pieces, arch)?);
    } else {
        println!("dpkg-deb not found: skipping the .deb");
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_appimage_name_ends_in_the_suffix_the_updater_selects() {
        assert_eq!(
            appimage_name("0.1.0", "amd64"),
            "Quiver_0.1.0_amd64.AppImage"
        );
    }

    #[test]
    fn apprun_executes_the_installed_app_name() {
        assert!(app_run().contains("usr/bin/quiverdesktop"));
    }
}
