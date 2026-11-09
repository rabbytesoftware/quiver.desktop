//! The two bridges between GPUI (one UI thread) and `quiver-state` (tokio):
//! [`observe_store`] turns a store change into a re-render, [`spawn_action`] runs async work on the
//! tokio runtime and hands the result back on the UI thread.

mod observe;
mod spawn;

pub use observe::observe_store;
pub use spawn::{spawn_action, spawn_detached};
