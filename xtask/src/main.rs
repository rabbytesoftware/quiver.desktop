//! Repo tasks, run as `cargo xtask <task>` (alias in `.cargo/config.toml`).

mod gate;
mod layout;
mod structure;

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("check-deps") => structure::check_dependency_direction(),
        Some("check-layout") => layout::check_file_sizes_and_names(),
        Some("gate") => gate::run(),
        Some(other) => bail!("unknown task `{other}`; try: gate, check-deps, check-layout"),
        None => bail!("usage: cargo xtask <gate|check-deps|check-layout>"),
    }
}
