//! Announcing quiver.desktop to the LOCAL quiver.core, so the catalog row says
//! what is actually running.
//!
//! Ported 1:1 from the TypeScript `announceSelf` (`self-announce.ts`). The
//! connection manager calls the [`SelfAnnounce`] hook only for the local sidecar
//! connection (see [`should_announce_self`](crate::should_announce_self)); this
//! module is the registration logic itself.

mod api;
mod flow;
mod namespace;

#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::transport::Transport;

pub use flow::{announce_self, AnnounceOutcome};

/// The namespace quiver.desktop is published under.
pub const QUIVER_DESKTOP_NAMESPACE: &str = "github.com/rabbytesoftware/quiver.desktop";

/// What this build is, as the release pipeline stamped it.
#[derive(Debug, Clone, Default)]
pub struct SelfAnnounceConfig {
    /// The release tag this binary was built from (`stable-26.5.0`); `None` for
    /// a local or PR build. An empty string counts as none.
    pub build_tag: Option<String>,
    /// The channel the release pipeline published this build on (`stable`,
    /// `beta`, `hotfix`, `nightly-rolling`); empty for a local or PR build.
    pub build_channel: String,
}

/// The quiver.desktop identities this app's own announces registered on the
/// local daemon -- the only rows an announce may later forget. A row the user
/// added (a pin, another channel) is never on it, and core cannot tell the two
/// apart: both are user-installed.
pub trait AnnouncedRows: Send + Sync {
    fn list(&self) -> Vec<String>;
    fn add(
        &self,
        identity: &str,
    );
    fn drop_identity(
        &self,
        identity: &str,
    );
}

/// An [`AnnouncedRows`] that remembers nothing across runs.
#[derive(Default)]
pub struct MemoryAnnouncedRows {
    identities: Mutex<Vec<String>>,
}

impl MemoryAnnouncedRows {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(identities: &[&str]) -> Self {
        Self {
            identities: Mutex::new(identities.iter().map(|s| s.to_string()).collect()),
        }
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.list()
    }
}

impl AnnouncedRows for MemoryAnnouncedRows {
    fn list(&self) -> Vec<String> {
        self.identities
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn add(
        &self,
        identity: &str,
    ) {
        let mut rows = self.identities.lock().unwrap_or_else(|e| e.into_inner());
        if !rows.iter().any(|known| known == identity) {
            rows.push(identity.to_string());
        }
    }

    fn drop_identity(
        &self,
        identity: &str,
    ) {
        self.identities
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|known| known != identity);
    }
}

/// The hook the connection manager calls when the LOCAL connection becomes
/// ready. It is never called for a remote one.
#[async_trait]
// async_trait expands to a must_use return type, which clippy flags against any #[must_use] on the trait.
#[allow(clippy::double_must_use)]
pub trait SelfAnnounce: Send + Sync + 'static {
    /// Fire-and-forget-with-logging: a daemon too old for any of this, an
    /// unreachable manifest host, or a transient error must never fail or block
    /// the connection this rides on, so nothing is returned.
    async fn announce(
        &self,
        transport: Arc<dyn Transport>,
    );
}

/// The real hook: [`announce_self`] over the local connection's transport.
pub struct LocalSelfAnnounce {
    config: SelfAnnounceConfig,
    rows: Option<Arc<dyn AnnouncedRows>>,
    on_adopted: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl LocalSelfAnnounce {
    /// `rows` is the record for the local connection (`None` when it cannot be
    /// kept: nothing is then recorded or forgotten). `on_adopted` runs after a
    /// successful adopt so the host can re-read the catalog.
    pub fn new(
        config: SelfAnnounceConfig,
        rows: Option<Arc<dyn AnnouncedRows>>,
        on_adopted: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Self {
        Self {
            config,
            rows,
            on_adopted,
        }
    }
}

#[async_trait]
impl SelfAnnounce for LocalSelfAnnounce {
    async fn announce(
        &self,
        transport: Arc<dyn Transport>,
    ) {
        let outcome = announce_self(transport.as_ref(), &self.config, self.rows.as_deref()).await;
        if outcome.adopted {
            if let Some(on_adopted) = &self.on_adopted {
                on_adopted();
            }
        }
    }
}
