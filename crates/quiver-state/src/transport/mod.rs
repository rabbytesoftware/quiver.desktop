//! Transport semantics (the decisions in `src/lib/transport`): the daemon's response envelope with
//! its proxy-failure retry, and the shared WebSocket manager with its reconnect sentinel. The
//! bytes themselves move in `quiver-core-client`.

pub mod api;
pub mod ws_manager;
