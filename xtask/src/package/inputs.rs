//! What a bundle is made of and where the pieces come from.
//!
//! The installed names are deliberate and match what the shipped `ARROW.md` lifecycles, the quiver-core-client
//! sidecar lookup (`quiver` next to the running executable) and the Windows/Linux detect commands already
//! expect: the GPUI app is installed as `quiverdesktop[.exe]` and the daemon sidecar as `quiver[.exe]`, side
//! by side in one directory (arrow apps' webviews live inside the app, there is no helper binary).

use crate::release_artifact;
use crate::target::{exe_suffix, host_triple};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub const PRODUCT: &str = "Quiver";
pub const IDENTIFIER: &str = "com.rabbytesoftware.quiver";
pub const INSTALLED_APP: &str = "quiverdesktop";
pub const INSTALLED_SIDECAR: &str = "quiver";
/// The cargo binary names the app may be built under (`quiverdesktop` after the bin rename, `quiver` before).
const CARGO_APP_NAMES: &[&str] = &["quiverdesktop", "quiver"];

#[derive(Debug, Clone)]
pub struct Inputs {
    pub version: String,
    pub triple: String,
    pub bin_dir: PathBuf,
    pub sidecar: PathBuf,
    pub out: PathBuf,
}

/// One binary and the name it is installed under.
#[derive(Debug, PartialEq, Eq)]
pub struct Piece {
    pub source: PathBuf,
    pub installed_as: String,
}

impl Inputs {
    pub fn resolve(
        version: Option<&str>,
        triple: Option<&str>,
        bin_dir: Option<&str>,
        out: Option<&str>,
    ) -> Result<Self> {
        // `cargo build --release` writes target/release; with an explicit `--target` it writes
        // target/<triple>/release.
        let default_bin_dir = match triple {
            Some(t) => PathBuf::from("target").join(t).join("release"),
            None => PathBuf::from("target/release"),
        };
        let triple = match triple {
            Some(t) => t.to_string(),
            None => host_triple()?,
        };
        let bin_dir = bin_dir.map_or(default_bin_dir, PathBuf::from);
        Ok(Self {
            version: version.unwrap_or("0.1.0").to_string(),
            sidecar: crate::sidecar::sidecar_path(&triple),
            triple,
            bin_dir,
            out: PathBuf::from(out.unwrap_or("target/bundle")),
        })
    }

    /// The two binaries, checked to exist.
    pub fn pieces(&self) -> Result<Vec<Piece>> {
        let suffix = exe_suffix(&self.triple);
        let app = find_app(&self.bin_dir, suffix)?;
        let pieces = vec![
            Piece {
                source: app,
                installed_as: format!("{INSTALLED_APP}{suffix}"),
            },
            Piece {
                source: self.sidecar.clone(),
                installed_as: format!("{INSTALLED_SIDECAR}{suffix}"),
            },
        ];
        for piece in &pieces {
            if !piece.source.is_file() {
                bail!(
                    "missing {} (build it / run `cargo xtask fetch-sidecar`)",
                    piece.source.display()
                );
            }
        }
        // The bundles are compressed images the scan cannot see into, so the raw binaries are checked here.
        release_artifact::check_files(&[&pieces[0].source])?;
        Ok(pieces)
    }
}

fn find_app(
    bin_dir: &Path,
    suffix: &str,
) -> Result<PathBuf> {
    CARGO_APP_NAMES
        .iter()
        .map(|name| bin_dir.join(format!("{name}{suffix}")))
        .find(|p| p.is_file())
        .with_context(|| {
            format!(
                "no app binary ({}) in {}",
                CARGO_APP_NAMES.join(" / "),
                bin_dir.display()
            )
        })
}

/// Copies every piece into `dir` under its installed name.
pub fn copy_pieces(
    pieces: &[Piece],
    dir: &Path,
) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    for piece in pieces {
        let dest = dir.join(&piece.installed_as);
        std::fs::copy(&piece.source, &dest)
            .with_context(|| format!("copying {} to {}", piece.source.display(), dest.display()))?;
        crate::sidecar::make_executable(&dest)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn inputs(dir: &Path) -> Inputs {
        Inputs {
            version: "1.0.0".into(),
            triple: "x86_64-unknown-linux-gnu".into(),
            bin_dir: dir.join("release"),
            sidecar: dir.join("core"),
            out: dir.join("out"),
        }
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    #[test]
    fn the_app_is_installed_as_quiverdesktop_and_the_sidecar_as_quiver() {
        let tmp = tempfile::tempdir().unwrap();
        let i = inputs(tmp.path());
        touch(&i.bin_dir.join("quiver"));
        touch(&i.sidecar);
        let names: Vec<_> = i
            .pieces()
            .unwrap()
            .into_iter()
            .map(|p| p.installed_as)
            .collect();
        assert_eq!(names, ["quiverdesktop", "quiver"]);
    }

    #[test]
    fn a_missing_piece_is_reported_by_path() {
        let tmp = tempfile::tempdir().unwrap();
        let i = inputs(tmp.path());
        touch(&i.bin_dir.join("quiverdesktop"));
        let err = i.pieces().unwrap_err().to_string();
        assert!(err.contains("core"), "{err}");
    }

    #[test]
    fn copy_pieces_uses_the_installed_names() {
        let tmp = tempfile::tempdir().unwrap();
        let i = inputs(tmp.path());
        touch(&i.bin_dir.join("quiverdesktop"));
        touch(&i.sidecar);
        let dest = tmp.path().join("stage");
        copy_pieces(&i.pieces().unwrap(), &dest).unwrap();
        assert!(dest.join("quiver").is_file() && dest.join("quiverdesktop").is_file());
    }
}
