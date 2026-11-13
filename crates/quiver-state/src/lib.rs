//! App state and actions (what the Zustand store did): one module per domain. No GPUI.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod arrow_app;
pub mod arrow_details;
pub mod collection;
pub mod collections;
pub mod connection;
pub mod console;
pub mod core_api;
pub mod domain;
pub mod dto;
pub mod home;
pub mod i18n;
pub mod library;
pub mod listeners;
pub mod namespace;
pub mod observable;
pub mod persistence;
pub mod queries;
pub mod recommended;
pub mod remote;
pub mod search;
pub mod settings;
pub mod shell;
pub mod sidebar;
pub mod store;
pub mod tile;
pub mod transport;

mod error;
pub use error::{Error, Result};
