//! The on-disk-capable entity cache behind the sidebar catalog (`src/lib/persistence`). IndexedDB
//! became an in-memory map with a pluggable snapshot; everything best-effort, as before.

pub mod entity_cache;
pub mod entity_stream;
pub mod key_value;
pub mod schemas;
pub mod version;

pub use entity_cache::EntityCache;
pub use key_value::{InMemoryKeyValueStore, KeyValueStore};
pub use schemas::ArrowCatalogRecord;

#[cfg(test)]
mod entity_stream_tests;
