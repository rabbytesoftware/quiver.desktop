//! The reactive stores (`src/lib/core-store/store`): the arrows catalog with its runtime overlay,
//! the connection status, and the two-lane search state with its pass controller.

pub mod arrows;
pub mod search;
pub mod search_pass;
pub mod status;

pub use arrows::{ArrowStore, ArrowsState, CatalogStatus};
pub use search::{SearchJob, SearchPhase, SearchState, SearchStore};
pub use status::StatusStore;

#[cfg(test)]
mod search_pass_tests;
