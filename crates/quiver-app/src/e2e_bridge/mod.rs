//! The debug-only control bridge (`e2e-bridge` feature).
//!
//! With the feature on, the app serves the JSON-RPC bridge from `quiver-e2e` on localhost and every view that
//! calls [`signal`] / [`frame_painted`] feeds it. With the feature off (every release build) the same
//! functions are empty inline stubs and no bridge code is compiled; CI inspects the release artifact to prove it.

#[cfg(feature = "e2e-bridge")]
mod live;
#[cfg(feature = "e2e-bridge")]
pub use live::{frame_painted, install_logger, signal, start, StateSnapshot};

#[cfg(not(feature = "e2e-bridge"))]
mod stub;
#[cfg(not(feature = "e2e-bridge"))]
pub use stub::{frame_painted, signal};
