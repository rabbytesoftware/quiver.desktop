//! Wires the daemon's streams into the stores (`src/lib/core-store/listeners`): when core says
//! `ready` the catalog and runtime streams start against the active connection; on `starting`
//! everything is torn down and reset. Generations make sure a superseded start never lands.

mod announce;
mod announced_rows;
mod streams;

pub use announce::{active_connection_kind, should_announce, SelfAnnounceHook};
pub use announced_rows::AnnouncedRows;
pub use streams::{setup_listeners, ListenerDeps, Listeners};

#[cfg(test)]
mod tests;
