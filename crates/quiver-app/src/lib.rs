//! The Quiver Desktop application crate: the glue between the pure crates
//! (`quiver-core-client`, `quiver-state`, `quiver-arrow-host`, `quiver-platform`,
//! `quiver-release`) and the views. No views live in [`services`].
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod services;
