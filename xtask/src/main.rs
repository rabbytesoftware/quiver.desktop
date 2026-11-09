//! Repo tasks, run as `cargo xtask <task>` (alias in `.cargo/config.toml`).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod args;
mod checksums;
mod core_version;
mod gate;
mod gh;
mod layout;
mod package;
mod package_layout;
mod release_artifact;
mod release_assets;
mod sidecar;
mod structure;
mod target;
mod verify_update;

use anyhow::{bail, Context, Result};

const TASKS: &str = "gate, check-deps, check-layout, resolve-core-version <channel>, extract-core-version <tag>, \
fetch-sidecar, package [macos|windows|linux], check-release-artifact <path>..., check-release-assets <dir>, checksums <dir>, verify-update";

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let task = args.next();
    let rest: Vec<String> = args.collect();
    match task.as_deref() {
        Some("check-deps") => structure::check_dependency_direction(),
        Some("check-layout") => layout::check_file_sizes_and_names(),
        Some("gate") => gate::run(),
        Some("resolve-core-version") => {
            let channel = rest
                .first()
                .context("usage: cargo xtask resolve-core-version <channel>")?;
            println!(
                "{}",
                core_version::resolve_channel(&gh::release_tags(gh::CORE_REPO)?, channel)?
            );
            Ok(())
        }
        Some("extract-core-version") => {
            let tag = rest
                .first()
                .context("usage: cargo xtask extract-core-version <desktop-release-tag>")?;
            println!(
                "{}",
                core_version::extract_core_version(&gh::release_notes(gh::DESKTOP_REPO, tag)?)?
            );
            Ok(())
        }
        Some("fetch-sidecar") => sidecar::run(rest),
        Some("package") => package::run(rest),
        Some("check-release-artifact") => release_artifact::run(rest),
        Some("check-release-assets") => release_assets::run(rest),
        Some("checksums") => checksums::run(rest),
        Some("verify-update") => verify_update::run(rest),
        Some(other) => bail!("unknown task `{other}`; try: {TASKS}"),
        None => bail!("usage: cargo xtask <{TASKS}>"),
    }
}
