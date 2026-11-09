//! `check-release-artifact <path>...`: fails if a built binary contains the e2e bridge.
//!
//! The bridge (`quiver-e2e`, feature `e2e-bridge` of `quiver-app`) is a remote-control surface for the UI and
//! must never ship. Its port env var and crate name are string constants that survive `strip`, so their
//! presence in a binary proves the feature was compiled in. A directory is scanned file by file.

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &["QUIVER_E2E_PORT", "quiver_e2e"];

/// The forbidden markers found in `bytes`.
pub fn markers_in(bytes: &[u8]) -> Vec<&'static str> {
    FORBIDDEN
        .iter()
        .copied()
        .filter(|needle| bytes.windows(needle.len()).any(|w| w == needle.as_bytes()))
        .collect()
}

pub fn check_files(paths: &[&Path]) -> Result<()> {
    let mut problems = Vec::new();
    for path in paths {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let found = markers_in(&bytes);
        if !found.is_empty() {
            problems.push(format!(
                "{}: contains the e2e bridge ({})",
                path.display(),
                found.join(", ")
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        bail!(
            "{}\nrelease builds must be built WITHOUT feature e2e-bridge",
            problems.join("\n")
        )
    }
}

fn collect(path: &Path, into: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            collect(&entry?.path(), into)?;
        }
    } else {
        into.push(path.to_path_buf());
    }
    Ok(())
}

pub fn run(paths: Vec<String>) -> Result<()> {
    if paths.is_empty() {
        bail!("usage: cargo xtask check-release-artifact <binary|dir>...");
    }
    let mut files = Vec::new();
    for path in &paths {
        collect(Path::new(path), &mut files)?;
    }
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    check_files(&refs)?;
    println!(
        "check-release-artifact: {} file(s) free of the e2e bridge",
        files.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binary_with_the_bridge_env_var_is_flagged() {
        assert_eq!(markers_in(b"\0\0QUIVER_E2E_PORT\0"), ["QUIVER_E2E_PORT"]);
        assert_eq!(markers_in(b"src/quiver_e2e/server.rs"), ["quiver_e2e"]);
    }

    #[test]
    fn a_clean_binary_passes() {
        assert!(markers_in(b"\x7fELF quiver_core quiver-e2e").is_empty());
    }

    #[test]
    fn check_files_names_the_offending_file() {
        let tmp = tempfile::tempdir().unwrap();
        let bad = tmp.path().join("bad");
        let good = tmp.path().join("good");
        fs::write(&bad, b"..QUIVER_E2E_PORT..").unwrap();
        fs::write(&good, b"nothing").unwrap();
        assert!(check_files(&[good.as_path()]).is_ok());
        let err = check_files(&[good.as_path(), bad.as_path()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("bad") && !err.contains("good: "), "{err}");
    }
}
