//! `cargo xtask perf`: native vs old Tauri performance, measured in the perf box
//! (`scripts/perf-box.sh cargo xtask perf <task>`: Xvfb + openbox + software rendering). See docs/performance.md.
//!
//! * `build [core|native|tauri|all]`: quiver.core, the two native builds, the Tauri release binary.
//! * `startup [--runs N] [--idle S] [--demo] [variant...]`: cold starts, RSS, idle CPU and wake-ups.
//! * `scroll`: the long-list and big-README scenarios of the demo.
//! * `all`: every measurement, JSON into docker/native/evidence/perf/.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

const DIR: &str = "xtask/src/perf";
const OUT: &str = "docker/native/evidence/perf";
const VARIANTS: [&str; 3] = ["native-release", "native-bridge", "tauri"];

fn sh(script: &str, args: &[&str]) -> Result<()> {
    let status = Command::new("bash")
        .arg(format!("{DIR}/{script}"))
        .args(args)
        .status()
        .with_context(|| format!("run {script}"))?;
    if !status.success() {
        bail!("{script} failed: {status}");
    }
    Ok(())
}

fn measure(args: &[&str]) -> Result<()> {
    let status = Command::new("python3")
        .arg(format!("{DIR}/measure.py"))
        .args(args)
        .status()
        .context("run measure.py (python3 and the perf box are required)")?;
    if !status.success() {
        bail!("measure.py {args:?} failed: {status}");
    }
    Ok(())
}

fn build(what: &str) -> Result<()> {
    match what {
        "core" => sh("build-core.sh", &[]),
        "native" => sh("build-native.sh", &[]),
        "tauri" => sh("build-tauri.sh", &[]),
        "all" => {
            build("core")?;
            build("native")?;
            build("tauri")
        }
        other => bail!("unknown build target `{other}`: core, native, tauri, all"),
    }
}

fn startup(rest: &[String]) -> Result<()> {
    let flags: Vec<&str> = rest
        .iter()
        .map(String::as_str)
        .filter(|a| !VARIANTS.contains(a))
        .collect();
    let mut variants: Vec<&str> = rest
        .iter()
        .map(String::as_str)
        .filter(|a| VARIANTS.contains(a))
        .collect();
    if variants.is_empty() {
        variants = VARIANTS.to_vec();
    }
    let demo = flags.contains(&"--demo");
    for variant in variants {
        if demo && variant == "tauri" {
            continue;
        }
        let out = format!(
            "{OUT}/startup-{variant}{}.json",
            if demo { "-demo" } else { "" }
        );
        let mut args = vec!["startup", "--variant", variant, "--out", &out];
        args.extend(&flags);
        measure(&args)?;
    }
    Ok(())
}

fn scroll() -> Result<()> {
    for (scenario, count) in [("sidebar", "5000"), ("library", "2000"), ("readme", "1")] {
        let out = format!("{OUT}/scroll-{scenario}.json");
        measure(&[
            "scroll",
            "--scenario",
            scenario,
            "--count",
            count,
            "--out",
            &out,
        ])?;
    }
    Ok(())
}

pub fn run(rest: Vec<String>) -> Result<()> {
    if !Path::new(DIR).is_dir() {
        bail!("run from the repository root");
    }
    let (task, rest) = rest
        .split_first()
        .context("usage: cargo xtask perf <build|startup|scroll|all>")?;
    match task.as_str() {
        "build" => build(rest.first().map_or("all", String::as_str)),
        "startup" => startup(rest),
        "scroll" => scroll(),
        "all" => {
            startup(rest)?;
            scroll()
        }
        other => bail!("unknown perf task `{other}`"),
    }
}
