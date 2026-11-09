//! Windows: the NSIS per-user installer `Quiver_<version>_x64-setup.exe` (the `*-setup.exe` the updater's
//! asset selection picks), compiled with `makensis` from `installer/windows/quiver.nsi`.
//!
//! The installer embeds the Visual C++ redistributable (`installer/windows/windows-installer-hooks.nsh`):
//! `QUIVER_VCREDIST_PATH` must name a `vc_redist.x64.exe` or the compile fails by design.
//! Authenticode signing is not done here (the old Tauri build did not sign either).

use super::command::run_with_paths;
use super::inputs::{copy_pieces, Inputs, PRODUCT};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;

const ICON: &str = "installer/icons/icon.ico";
const SCRIPT: &str = "installer/windows/quiver.nsi";

/// `1.2.3` -> `1.2.3` (NSIS wants a numeric `major.minor.patch`; the build tag is not part of it).
pub fn nsis_version(version: &str) -> Result<String> {
    let numeric: Vec<&str> = version.split('.').collect();
    if numeric.len() != 3
        || numeric
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        bail!("`{version}` is not a numeric major.minor.patch version");
    }
    Ok(version.to_string())
}

pub fn installer_name(version: &str) -> String {
    format!("{PRODUCT}_{version}_x64-setup.exe")
}

pub fn package(inputs: &Inputs) -> Result<Vec<PathBuf>> {
    let version = nsis_version(&inputs.version)?;
    let vcredist = std::env::var("QUIVER_VCREDIST_PATH").unwrap_or_default();
    if !PathBuf::from(&vcredist).is_file() {
        bail!("QUIVER_VCREDIST_PATH must point at vc_redist.x64.exe (got `{vcredist}`)");
    }
    let stage = inputs.out.join("windows-stage");
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    copy_pieces(&inputs.pieces()?, &stage)?;
    fs::copy(ICON, stage.join("icon.ico")).with_context(|| format!("copying {ICON}"))?;

    fs::create_dir_all(&inputs.out)?;
    let out = std::env::current_dir()?
        .join(&inputs.out)
        .join(installer_name(&version));
    let stage_abs = std::env::current_dir()?.join(&stage);
    let define_version = format!("/DVERSION={version}");
    let define_stage = format!("/DSTAGE={}", stage_abs.display());
    let define_out = format!("/DOUT={}", out.display());
    run_with_paths(
        "makensis",
        &["/V2", &define_version, &define_stage, &define_out],
        &[&PathBuf::from(SCRIPT)],
    )?;
    Ok(vec![out])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_installer_name_ends_in_the_suffix_the_updater_selects() {
        assert!(installer_name("0.1.0").ends_with("-setup.exe"));
        assert_eq!(installer_name("0.1.0"), "Quiver_0.1.0_x64-setup.exe");
    }

    #[test]
    fn nsis_needs_a_numeric_triple() {
        assert!(nsis_version("1.2.3").is_ok());
        assert!(nsis_version("stable-1.0").is_err());
        assert!(nsis_version("1.2").is_err());
    }
}
