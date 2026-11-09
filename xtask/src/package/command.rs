//! Running the external tools packaging needs (`hdiutil`, `makensis`, `dpkg-deb`, ...).

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub fn run(program: &str, args: &[&str]) -> Result<()> {
    println!("+ {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("{program} not found or not runnable"))?;
    if !status.success() {
        bail!("{program} exited with {status}");
    }
    Ok(())
}

/// Like [`run`] for paths: they are passed as UTF-8 or the task fails.
pub fn run_with_paths(program: &str, args: &[&str], paths: &[&Path]) -> Result<()> {
    let mut all: Vec<&str> = args.to_vec();
    for path in paths {
        all.push(path.to_str().context("non-utf8 path")?);
    }
    run(program, &all)
}

pub fn exists_on_path(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success() || !o.stdout.is_empty() || !o.stderr.is_empty())
}
