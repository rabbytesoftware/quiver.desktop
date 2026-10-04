//! Arrow Apps: showing an arrow's web interface inside the shell.
//!
//! The page lives at `arrow-app://<host>/` (on Windows wry rewrites that to
//! `http://arrow-app.<host>/`). Every request is forwarded to the daemon's
//! `/v0/ui/<namespace>/...` route over the active connection, so no port is
//! opened on this machine and the bearer token never reaches the page.

pub mod csp;
pub mod handler;
pub mod hosts;
pub mod inject;
pub mod uri;
// pub mod ws;

/// The custom URI scheme arrow interfaces are served from.
pub const SCHEME: &str = "arrow-app";

/// Path on every arrow origin answered by this app instead of the daemon.
pub const SHIM_PATH: &str = "/__arrow/shim.js";

/// The WebSocket shim, shipped inside the binary.
pub const SHIM_JS: &str = include_str!("../../../src/arrow-shim/shim.js");
