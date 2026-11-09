//! Reads and writes against the daemon (`src/lib/core-store/queries` and `mutations`), without
//! TanStack: plain async functions, a [`QueryClient`] that remembers invalidations and cached
//! data (for optimistic updates), and [`Query`] to hold a result a view renders.

pub mod arrow;
pub mod client;
pub mod collection;
pub mod home;
pub mod keys;
pub mod mutations;
pub mod query;

pub use client::QueryClient;
pub use query::{Query, QueryState};
