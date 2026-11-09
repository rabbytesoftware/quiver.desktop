//! Dependency-direction check: nothing below `quiver-ui` may depend on `gpui`, and no new crate may
//! depend on `tauri`. Reads `cargo metadata` so it also catches transitive declarations in manifests.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::process::Command;

/// Crates that must stay free of any GPUI dependency.
const NO_GPUI: &[&str] =
    &["quiver-core-client", "quiver-state", "quiver-release", "quiver-arrow-host", "quiver-platform", "quiver-theme"];
const FORBIDDEN: &[&str] = &["gpui", "gpui-component", "tauri"];
/// The internal edges each no-GPUI crate may have.
const ALLOWED_INTERNAL: &[(&str, &[&str])] = &[
    ("quiver-theme", &[]),
    ("quiver-platform", &[]),
    ("quiver-core-client", &[]),
    ("quiver-arrow-host", &["quiver-core-client"]),
    ("quiver-release", &["quiver-core-client", "quiver-platform"]),
    ("quiver-state", &["quiver-core-client", "quiver-release", "quiver-arrow-host", "quiver-platform"]),
];

pub fn check_dependency_direction() -> Result<()> {
    let out = Command::new("cargo").args(["metadata", "--format-version", "1", "--no-deps"]).output()?;
    if !out.status.success() {
        bail!("cargo metadata failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let meta: Value = serde_json::from_slice(&out.stdout)?;
    let packages = meta["packages"].as_array().context("no packages in metadata")?;
    let mut violations = Vec::new();
    for pkg in packages {
        let name = pkg["name"].as_str().unwrap_or_default();
        if !name.starts_with("quiver-") && name != "xtask" {
            continue;
        }
        let deps: Vec<&str> = pkg["dependencies"]
            .as_array()
            .map(|d| d.iter().filter_map(|x| x["name"].as_str()).collect())
            .unwrap_or_default();
        for dep in &deps {
            if FORBIDDEN.contains(dep) && NO_GPUI.contains(&name) {
                violations.push(format!("{name} depends on `{dep}` (only quiver-ui and quiver-app may use GPUI)"));
            }
            if *dep == "tauri" || dep.starts_with("tauri-") {
                violations.push(format!("{name} depends on `{dep}` (Tauri is gone)"));
            }
            if let Some((_, allowed)) = ALLOWED_INTERNAL.iter().find(|(n, _)| *n == name) {
                if dep.starts_with("quiver-") && !allowed.contains(dep) {
                    violations.push(format!("{name} -> {dep} points the wrong way (allowed: {allowed:?})"));
                }
            }
        }
    }
    if violations.is_empty() {
        println!("check-deps: ok ({} crates)", packages.len());
        Ok(())
    } else {
        bail!("dependency direction violations:\n  {}", violations.join("\n  "))
    }
}
