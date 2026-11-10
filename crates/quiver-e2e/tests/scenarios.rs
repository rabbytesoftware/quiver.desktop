#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The end-to-end scenarios: each drives the REAL app (built with the e2e bridge) against a real quiver.core.
//! Run them in the box (`make test-e2e-box`); without `QUIVER_E2E_APP_BINARY` every scenario skips.

mod support;

#[path = "scenarios/auto_register.rs"]
mod auto_register;
#[path = "scenarios/bootstrap.rs"]
mod bootstrap;
#[path = "scenarios/console.rs"]
mod console;
#[path = "scenarios/generic_outdated_badge.rs"]
mod generic_outdated_badge;
#[path = "scenarios/qol.rs"]
mod qol;
#[path = "scenarios/self_update_while_running.rs"]
mod self_update_while_running;
