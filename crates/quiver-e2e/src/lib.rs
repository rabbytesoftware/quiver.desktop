//! End-to-end control of the real Quiver app.
//!
//! The app (built with its `e2e-bridge` feature) serves a JSON-RPC bridge on localhost. This crate holds
//! the shared [`protocol`], a [`client::Bridge`] for scenarios, an [`app::AppProcess`] launcher and the
//! MCP server (`quiver-mcp`) that lets an agent drive the app.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod app;
pub mod client;
pub mod error;
pub mod protocol;
pub mod server;
pub mod signals;

pub use error::{Error, Result};
