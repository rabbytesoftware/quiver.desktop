//! Everything between Quiver Desktop and a quiver.core daemon, with no UI and
//! no Tauri: this crate is what the application and `quiver-state` talk to.
//!
//! * [`transport`]: one request-shaped [`Transport`] over a unix socket, a
//!   Windows named pipe or https with a bearer token.
//! * [`local`]: spawn, supervise and stop the sidecar daemon this app owns
//!   ([`LocalConfig`], [`LocalConnection`]); it never kills a daemon it did not
//!   start.
//! * [`remote`]: a daemon somebody else runs ([`RemoteConnection`]), pairing
//!   codes ([`redeem_pairing_code`]) and the reachability probe.
//! * [`manager`]: [`ConnectionManager`] keeps local plus saved remotes, the
//!   active one, and switches between them. Persistence and secrets sit behind
//!   [`stores::SettingsStore`] / [`stores::SecretStore`]; events behind
//!   [`EventSink`].
//! * [`proxy`], [`bridge`], [`console`]: the `quiver://` request proxy, the
//!   WebSocket bridge and streamed console execution, all over the active
//!   transport.
//! * [`self_announce`]: registers quiver.desktop in the LOCAL daemon's catalog.
//!   [`should_announce_self`] is the policy that keeps remote connections out.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod bridge;
pub mod console;
mod error;
pub mod local;
pub mod manager;
pub mod proxy;
pub mod remote;
pub mod self_announce;
pub mod stores;
pub mod transport;
pub mod types;

pub use error::{Error, Result};
pub use local::{LocalConfig, LocalConnection, LocalHost};
pub use manager::{ConnectionManager, ManagerDeps};
pub use remote::{check_remote_health, probe_health, redeem_pairing_code, RemoteConnection};
pub use self_announce::{AnnouncedRows, SelfAnnounce, SelfAnnounceConfig};
pub use transport::Transport;
pub use types::{
    should_announce_self, ConnectionConfig, ConnectionKind, ConnectionsState, CoreStatus,
    EventSink, QuiverConnection,
};

/// Serialises every test in this crate that opens a socket.
///
/// `bridge`'s descriptor-leak regression test counts this process's open file
/// descriptors, which is process-wide state: a listener stood up by any other
/// test lands in the same count — in its baseline as well as in its final read.
/// That is not hypothetical. With the guard scoped to `bridge` alone, a
/// deliberately reintroduced 19-descriptor leak went UNDETECTED in 4 of 20
/// parallel runs, because the baseline was taken while a dozen of the transport
/// tests' sockets were open and the leak fitted inside that noise. So every test
/// that opens a socket takes this.
///
/// It lives in the crate root because that is the one module every test module
/// can name. `tokio::sync::Mutex` because the holders are all `async`, and
/// because it cannot be poisoned by a test that fails while holding it.
#[cfg(test)]
pub(crate) static FD_TESTS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
