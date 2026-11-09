//! The quality gate (`make pr-checks` for the native workspace).

use anyhow::{bail, Result};
use std::process::Command;

const STEPS: &[(&str, &[&str])] = &[
    ("fmt", &["cargo", "fmt", "--all", "--check"]),
    (
        "clippy",
        &["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    ),
    ("test", &["cargo", "test", "--workspace"]),
    ("deny", &["cargo", "deny", "check"]),
    ("audit", &["cargo", "audit"]),
    ("machete", &["cargo", "machete"]),
    ("check-deps", &["cargo", "run", "-q", "-p", "xtask", "--", "check-deps"]),
    ("check-layout", &["cargo", "run", "-q", "-p", "xtask", "--", "check-layout"]),
];

pub fn run() -> Result<()> {
    let mut failed = Vec::new();
    for (name, cmd) in STEPS {
        println!("==> {name}");
        let status = Command::new(cmd[0]).args(&cmd[1..]).status()?;
        if !status.success() {
            failed.push(*name);
        }
    }
    if failed.is_empty() {
        println!("gate: all green");
        Ok(())
    } else {
        bail!("gate failed: {}", failed.join(", "))
    }
}
