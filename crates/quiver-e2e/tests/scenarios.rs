#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The end-to-end scenarios: each drives the REAL app (built with the e2e bridge) against a real quiver.core.
//! Run them in the box (`make test-e2e-box`); without `QUIVER_E2E_APP_BINARY` every scenario skips.

mod support;

#[path = "scenarios/arrow_apps.rs"]
mod arrow_apps;
#[path = "scenarios/arrow_page.rs"]
mod arrow_page;
#[path = "scenarios/auto_register.rs"]
mod auto_register;
#[path = "scenarios/bootstrap.rs"]
mod bootstrap;
#[path = "scenarios/console.rs"]
mod console;
#[path = "scenarios/core_settings.rs"]
mod core_settings;
#[path = "scenarios/generic_outdated_badge.rs"]
mod generic_outdated_badge;
#[path = "scenarios/glyphs.rs"]
mod glyphs;
#[path = "scenarios/modals.rs"]
mod modals;
#[path = "scenarios/qol.rs"]
mod qol;
#[path = "scenarios/row_menu.rs"]
mod row_menu;
#[path = "scenarios/self_update_while_running.rs"]
mod self_update_while_running;
#[path = "scenarios/update_core.rs"]
mod update_core;
