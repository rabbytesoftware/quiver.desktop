//! Shared plumbing for the scenarios: environment, per-test home, the app, daemon access, the GitHub stand-in.
//! One module per concern so the next scenario (update_core, arrow_apps) reuses them as they are.

#![allow(dead_code)]

pub mod app;
pub mod arrow_windows;
pub mod arrows;
pub mod core_update;
pub mod daemon;
pub mod env;
pub mod home;
pub mod lifecycle;
pub mod procs;
pub mod sandbox;
pub mod upstream;
pub mod wait;

pub const NS_CORE: &str = "github.com/rabbytesoftware/quiver.core";
pub const NS_DESKTOP: &str = "github.com/rabbytesoftware/quiver.desktop";
