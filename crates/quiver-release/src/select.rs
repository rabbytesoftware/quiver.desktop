//! Picking the bundle to install out of a release's asset list.

use crate::asset::ReleaseAsset;
use crate::{Error, Result};

/// The filename suffix identifying this platform's bundle.
///
/// These are `ARROW.md`'s own three choices, and `install.sh`'s and
/// `install.ps1`'s: the AppImage on Linux (the `.deb` published beside it
/// needs root and a Debian-family distro), the `.dmg` on macOS, and the NSIS
/// `*-setup.exe` on Windows (the `.msi` published beside it installs
/// per-machine and would need elevation). Diverging here would mean the app
/// updating itself into a different place than it installed itself.
fn platform_suffix(os: &str) -> Option<&'static str> {
    match os {
        "linux" => Some(".appimage"),
        "macos" => Some(".dmg"),
        "windows" => Some("-setup.exe"),
        _ => None,
    }
}

/// Every spelling of an architecture that appears in a bundle filename.
///
/// Tauri names the Linux bundle with the Debian architecture (`amd64`) and the
/// macOS one with the Rust triple's (`x86_64`); `x64` and `aarch64` are what
/// other tooling emits. An unrecognised architecture returns an empty slice,
/// which [`select_asset`] reads as "do not narrow" -- so a release carrying
/// exactly one asset of the right kind still resolves, exactly as `install.sh`
/// treats its `unknown`.
fn arch_aliases(arch: &str) -> &'static [&'static str] {
    match arch {
        "x86_64" => &["amd64", "x86_64", "x64"],
        "aarch64" => &["arm64", "aarch64"],
        _ => &[],
    }
}

fn names_arch(
    name: &str,
    aliases: &[&str],
) -> bool {
    let lower = name.to_ascii_lowercase();
    aliases.iter().any(|alias| lower.contains(alias))
}

/// Picks the asset to install out of a release's asset list.
///
/// Selection is by extension, never by filename: quiver.desktop's release
/// filenames carry a product version that does not track the git tag, so no
/// name can be predicted from the tag being installed.
///
/// Architecture narrows the candidates only when it leaves at least one --
/// a release publishing a single universal bundle (which is what the macOS
/// `.dmg` is) must not be narrowed out of existence.
pub fn select_asset<'a>(
    assets: &'a [ReleaseAsset],
    os: &str,
    arch: &str,
) -> Result<&'a ReleaseAsset> {
    let Some(suffix) = platform_suffix(os) else {
        return Err(Error::UnsupportedPlatform(format!(
            "no Quiver bundle is published for {os}"
        )));
    };

    let candidates: Vec<&ReleaseAsset> = assets
        .iter()
        .filter(|asset| asset.name.to_ascii_lowercase().ends_with(suffix))
        .collect();

    let Some(first) = candidates.first().copied() else {
        return Err(Error::NoAsset(format!(
            "this release publishes no *{suffix} asset for {os}/{arch}"
        )));
    };

    let aliases = arch_aliases(arch);
    let narrowed = candidates
        .iter()
        .copied()
        .find(|asset| names_arch(&asset.name, aliases));

    Ok(narrowed.unwrap_or(first))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::kind::{NO_ASSET, UNSUPPORTED_PLATFORM};

    fn asset(name: &str) -> ReleaseAsset {
        ReleaseAsset {
            name: name.to_string(),
            browser_download_url: format!(
                "https://github.com/rabbytesoftware/quiver.desktop/releases/download/stable-1.0/{name}"
            ),
            digest: None,
        }
    }

    /// The whole reason selection is by extension: these are the names a
    /// real `tauri build` publishes, all carrying the same static product
    /// version, none of them naming the tag.
    fn full_release() -> Vec<ReleaseAsset> {
        vec![
            asset("quiver-desktop_0.1.0_amd64.AppImage"),
            asset("quiver-desktop_0.1.0_amd64.deb"),
            asset("Quiver_0.1.0_aarch64.dmg"),
            asset("Quiver_0.1.0_x64_en-US.msi"),
            asset("Quiver_0.1.0_x64-setup.exe"),
        ]
    }

    #[test]
    fn linux_takes_the_appimage_and_never_the_deb() {
        let assets = full_release();
        let chosen = select_asset(&assets, "linux", "x86_64").unwrap();
        assert_eq!(chosen.name, "quiver-desktop_0.1.0_amd64.AppImage");
    }

    #[test]
    fn macos_takes_the_dmg() {
        let assets = full_release();
        let chosen = select_asset(&assets, "macos", "aarch64").unwrap();
        assert_eq!(chosen.name, "Quiver_0.1.0_aarch64.dmg");
    }

    /// The `.msi` is published beside the NSIS bundle and installs
    /// per-machine, so it would need elevation. `ARROW.md` and `install.ps1`
    /// both take the `-setup.exe`; so must this.
    #[test]
    fn windows_takes_the_nsis_setup_and_never_the_msi() {
        let assets = full_release();
        let chosen = select_asset(&assets, "windows", "x86_64").unwrap();
        assert_eq!(chosen.name, "Quiver_0.1.0_x64-setup.exe");
    }

    #[test]
    fn architecture_decides_between_two_bundles_of_the_same_kind() {
        let assets = vec![
            asset("Quiver_0.1.0_x64.dmg"),
            asset("Quiver_0.1.0_aarch64.dmg"),
        ];
        assert_eq!(
            select_asset(&assets, "macos", "aarch64").unwrap().name,
            "Quiver_0.1.0_aarch64.dmg"
        );
        assert_eq!(
            select_asset(&assets, "macos", "x86_64").unwrap().name,
            "Quiver_0.1.0_x64.dmg"
        );
    }

    /// A universal bundle names no architecture at all. Narrowing must not
    /// remove the only candidate.
    #[test]
    fn a_release_with_one_unnamed_bundle_still_resolves() {
        let assets = vec![asset("Quiver_0.1.0_universal.dmg")];
        assert_eq!(
            select_asset(&assets, "macos", "aarch64").unwrap().name,
            "Quiver_0.1.0_universal.dmg"
        );
    }

    /// `install.sh` prints `unknown` for a machine name it does not know and
    /// carries on without narrowing. Same rule here.
    #[test]
    fn an_unknown_architecture_does_not_narrow() {
        let assets = vec![asset("quiver-desktop_0.1.0_amd64.AppImage")];
        assert_eq!(
            select_asset(&assets, "linux", "powerpc64").unwrap().name,
            "quiver-desktop_0.1.0_amd64.AppImage"
        );
    }

    #[test]
    fn a_release_with_nothing_for_this_platform_is_an_error() {
        let assets = vec![asset("quiver-desktop_0.1.0_amd64.AppImage")];
        let err = select_asset(&assets, "macos", "aarch64").unwrap_err();
        assert_eq!(err.kind(), NO_ASSET);
    }

    #[test]
    fn an_empty_release_is_an_error_not_a_panic() {
        let err = select_asset(&[], "linux", "x86_64").unwrap_err();
        assert_eq!(err.kind(), NO_ASSET);
    }

    #[test]
    fn a_platform_with_no_bundle_is_named_as_such() {
        let assets = full_release();
        let err = select_asset(&assets, "freebsd", "x86_64").unwrap_err();
        assert_eq!(err.kind(), UNSUPPORTED_PLATFORM);
    }

    /// GitHub preserves upload order, and the case of an extension is the
    /// bundler's choice (`.AppImage`), so matching must be case-insensitive
    /// without the match itself reordering anything.
    #[test]
    fn extension_matching_ignores_case() {
        let assets = vec![asset("Quiver_0.1.0_amd64.appimage")];
        assert_eq!(
            select_asset(&assets, "linux", "x86_64").unwrap().name,
            "Quiver_0.1.0_amd64.appimage"
        );
    }
}
