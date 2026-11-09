//! Layout rules: no file over 500 lines, no `utils`/`helpers`/`common`/`misc` modules.

use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

const HARD_LIMIT: usize = 500;
const BANNED_STEMS: &[&str] = &["utils", "util", "helpers", "common", "misc"];

pub fn check_file_sizes_and_names() -> Result<()> {
    let mut problems = Vec::new();
    for root in ["crates", "xtask/src"] {
        walk(Path::new(root), &mut problems)?;
    }
    if problems.is_empty() {
        println!("check-layout: ok");
        Ok(())
    } else {
        bail!("layout violations:\n  {}", problems.join("\n  "))
    }
}

fn walk(dir: &Path, problems: &mut Vec<String>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n == "target" || n == "node_modules")
            {
                continue;
            }
            walk(&path, problems)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if BANNED_STEMS.contains(&stem) {
                problems.push(format!("{}: banned module name `{stem}`", path.display()));
            }
            let lines = fs::read_to_string(&path)?.lines().count();
            if lines > HARD_LIMIT {
                problems.push(format!(
                    "{}: {lines} lines (hard limit {HARD_LIMIT})",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}
