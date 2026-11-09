//! Which daemon connections exist and which one is active (`src/lib/connection`), plus the
//! [`Backend`] seam to the native shell that owns them.

mod backend;
#[cfg(any(test, feature = "testing"))]
mod fake;
mod listeners;
mod store;

pub use backend::{Backend, BackendPlatform, ConnectionsSnapshot};
#[cfg(any(test, feature = "testing"))]
pub use fake::FakeBackend;
pub use listeners::{setup_connection_listeners, ConnectionListeners};
pub use store::{ConnectionState, ConnectionStore};
