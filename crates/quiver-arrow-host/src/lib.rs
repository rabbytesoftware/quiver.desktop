//! Arrow app rules: scheme, CSP, host allow-list, 4-app cap, script injection,
//! and the IPC between the main process and the native arrow window process.
//! No GPUI, no webview: the app implements [`ArrowUpstream`] over its daemon
//! transport and a helper process shows the pages.
//!
//! The page lives at `arrow-app://<host>/`, where the host is `<hex>.localhost`
//! (on Windows wry rewrites that to `http://arrow-app.<hex>.localhost/`, a
//! secure context). Every request is forwarded to the daemon's
//! `/v0/ui/<namespace>/...` route over the active connection, so no port is
//! opened on this machine and the bearer token never reaches the page.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod csp;
mod error;
mod handler;
mod hosts;
mod inject;
mod ipc;
mod message;
mod protocol;
mod registry;
mod upstream;
mod uri;
mod ws;

pub use csp::{harden, ARROW_CSP};
pub use error::{Error, Result};
pub use handler::handle_request;
pub use hosts::{arrow_host, host_label, ArrowHosts};
pub use inject::inject_shim;
pub use ipc::{
    decode_line, encode_line, HostToWindow, ProtocolRequest, ProtocolResponse, WindowToHost,
};
pub use message::{ArrowRequest, ArrowResponse, Headers};
pub use protocol::ProtocolService;
pub use registry::{admit, ArrowAppRegistry, Shown, MAX_OPEN_APPS};
pub use upstream::{ArrowUpstream, WsRelay};
pub use uri::{app_url, daemon_path, encode_namespace, host_of, is_safe_ws_path};
pub use ws::{ws_key, ws_target};

/// The custom URI scheme arrow interfaces are served from.
pub const SCHEME: &str = "arrow-app";

/// Path on every arrow origin answered by this app instead of the daemon.
pub const SHIM_PATH: &str = "/__arrow/shim.js";

/// The WebSocket shim, shipped inside the binary.
pub const SHIM_JS: &str = include_str!("../assets/shim.js");
