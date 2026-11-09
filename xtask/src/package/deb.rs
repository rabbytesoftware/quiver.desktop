//! The `.deb`: the three binaries in `/usr/bin` (the layout the Tauri .deb had, and what the `ARROW.md`
//! Linux detect command expects: `quiverdesktop` on PATH), a menu entry and an icon.

use super::command::run;
use super::inputs::{copy_pieces, Inputs, Piece};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

const DESKTOP_ENTRY: &str = include_str!("../../../installer/linux/quiver.desktop");
const ICON: &str = "installer/icons/icon.png";
/// Runtime libraries the GPUI app and the WebKitGTK helper load.
const DEPENDS: &str = "libwebkit2gtk-4.1-0, libgtk-3-0, libxkbcommon-x11-0, libvulkan1, libfontconfig1, libssl3, libxcb1";

pub fn control(version: &str, arch: &str) -> String {
    format!(
        "Package: quiver\nVersion: {version}\nSection: devel\nPriority: optional\nArchitecture: {arch}\nDepends: {DEPENDS}\nMaintainer: Rabbyte Software\nDescription: Quiver Desktop\n The Quiver desktop application.\n"
    )
}

pub fn build(inputs: &Inputs, pieces: &[Piece], arch: &str) -> Result<PathBuf> {
    let root = inputs.out.join("linux-stage/deb");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    copy_pieces(pieces, &root.join("usr/bin"))?;
    let apps = root.join("usr/share/applications");
    fs::create_dir_all(&apps)?;
    fs::write(apps.join("quiver.desktop"), DESKTOP_ENTRY)?;
    let icons = root.join("usr/share/icons/hicolor/512x512/apps");
    fs::create_dir_all(&icons)?;
    fs::copy(ICON, icons.join("quiver.png")).with_context(|| format!("copying {ICON}"))?;
    fs::create_dir_all(root.join("DEBIAN"))?;
    fs::write(root.join("DEBIAN/control"), control(&inputs.version, arch))?;

    let deb = inputs
        .out
        .join(format!("quiver-desktop_{}_{arch}.deb", inputs.version));
    let (root_arg, deb_arg) = (
        root.to_str().context("non-utf8 path")?,
        deb.to_str().context("non-utf8 path")?,
    );
    run(
        "dpkg-deb",
        &["--root-owner-group", "--build", root_arg, deb_arg],
    )?;
    Ok(deb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_control_file_names_the_package_version_and_arch() {
        let c = control("1.2.3", "amd64");
        assert!(c.contains("Version: 1.2.3") && c.contains("Architecture: amd64"));
        assert!(c.ends_with('\n'));
    }
}
