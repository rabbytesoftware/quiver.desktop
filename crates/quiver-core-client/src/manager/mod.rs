//! The set of connections: local plus saved remotes, which one is active, and
//! how to switch between them without wedging the app.

mod merge;
mod persist;
mod switch;

use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

use crate::bridge::WsBridgeManager;
use crate::local::{self, LocalConfig, LocalConnection};
use crate::remote::{redeem_pairing_code, RemoteConnection};
use crate::self_announce::SelfAnnounce;
use crate::stores::{SecretStore, SettingsStore};
use crate::transport::http::HttpTransport;
use crate::transport::Transport;
use crate::types::{
    should_announce_self, ConnectionConfig, ConnectionsState, CoreStatus, EventSink,
    QuiverConnection,
};
use crate::{Error, Result};

use merge::merge_connections;
use persist::{device_id, load_remote_configs, save_remote_configs};
use switch::retire_streams_and_teardown;

/// The label a paired device shows up under in `quiver auth devices list` on
/// the core side -- one label for every remote this app is ever pointed at,
/// since a device id already identifies the pairing, not the connection's
/// own (purely local, user-chosen) display name.
const DEVICE_LABEL: &str = "Quiver Desktop";

/// Everything the manager needs from the application around it.
pub struct ManagerDeps {
    /// How the local daemon is reached and where its binary comes from.
    pub local: LocalConfig,
    /// Saved remote connections and the device id.
    pub settings: Arc<dyn SettingsStore>,
    /// Bearer tokens of saved remote connections.
    pub secrets: Arc<dyn SecretStore>,
    /// Receives core status and connection-changed events.
    pub events: Arc<dyn EventSink>,
    /// The WebSocket legs a switch must retire.
    pub bridge: Arc<WsBridgeManager>,
    /// Registers quiver.desktop in the Core catalog. Called for the LOCAL
    /// connection only, each time it becomes ready; never for a remote one.
    pub self_announce: Option<Arc<dyn SelfAnnounce>>,
}

pub struct ConnectionManager {
    active: RwLock<Box<dyn QuiverConnection>>,
    /// Serialises switches.
    ///
    /// This used to be the write half of `active`: `switch_connection` took it
    /// before starting the new connection and released it after installing, so
    /// no two switches could interleave. That is exactly what wedged the app —
    /// `start()` dials a peer, a peer that accepts and then stalls holds the
    /// write lock indefinitely, and tokio's `RwLock` is fair, so every later
    /// READER (i.e. every `quiver://` request and every `ws_open`) queued behind
    /// the waiting writer and never came back.
    ///
    /// So the serialisation moved here, to a lock no reader ever touches. A
    /// stalled `start()` now blocks nothing but other switches, which is the
    /// one thing it is supposed to block.
    switching: Mutex<()>,
    deps: ManagerDeps,
}

impl ConnectionManager {
    pub fn new(deps: ManagerDeps) -> Self {
        Self::with_active(deps.local.clone(), deps)
    }

    fn with_active(
        local: LocalConfig,
        deps: ManagerDeps,
    ) -> Self {
        Self::with_connection(Box::new(LocalConnection::new(local)), deps)
    }

    fn with_connection(
        active: Box<dyn QuiverConnection>,
        deps: ManagerDeps,
    ) -> Self {
        Self {
            active: RwLock::new(active),
            switching: Mutex::new(()),
            deps,
        }
    }

    /// The bridge the manager retires on a switch; the app opens its streams on it.
    pub fn bridge(&self) -> &Arc<WsBridgeManager> {
        &self.deps.bridge
    }

    /// Bring the active connection up. Resolves once it is ready or has failed;
    /// the self-announce (local only) runs in the background after that.
    pub async fn start(&self) {
        self.start_active().await;
    }

    /// [`Self::start`], returning the background self-announce when one was begun.
    async fn start_active(&self) -> Option<JoinHandle<()>> {
        // Takes `switching` even though it changes nothing: this is a long
        // await (the local sidecar's health wait runs to five seconds) taken
        // under the READ lock, and the invariant that keeps tokio's fair
        // `RwLock` harmless is that no writer is ever *waiting* on it while a
        // reader holds it. Every writer goes through `switching` first, so a
        // switch or a rename that lands mid-startup parks here instead of on
        // `active.write()`, where it would stall every request in the app.
        let _switching = self.switching.lock().await;
        let active = self.active.read().await;
        let status = active.start(self.deps.events.as_ref()).await;
        self.announce_on_ready(&**active, status)
    }

    /// Announce quiver.desktop to a daemon that just became ready -- but only the
    /// local sidecar's. A remote core belongs to somebody else: registering this
    /// desktop in its catalog is exactly the bug `should_announce_self` guards.
    fn announce_on_ready(
        &self,
        connection: &dyn QuiverConnection,
        status: CoreStatus,
    ) -> Option<JoinHandle<()>> {
        if status != CoreStatus::Ready || !should_announce_self(connection.config().kind) {
            return None;
        }
        let hook = Arc::clone(self.deps.self_announce.as_ref()?);
        let transport = connection.transport();
        Some(tokio::spawn(async move { hook.announce(transport).await }))
    }

    pub async fn get_connections(&self) -> ConnectionsState {
        let active = self.active.read().await;
        let active_id = active.config().id.clone();
        let active_config = active.config().clone();
        drop(active);

        let remotes = load_remote_configs(self.deps.settings.as_ref());
        let connections =
            merge_connections(&active_id, active_config, local::default_config(), remotes);
        ConnectionsState {
            connections,
            active_id,
        }
    }

    async fn emit_connection_changed(&self) {
        let state = self.get_connections().await;
        self.deps.events.emit_connection_changed(&state);
    }

    /// `code` is a one-time device-pairing code (see `quiver auth generate` on
    /// the core side), not a bearer token: this daemon has no auth surface to
    /// redeem it against unless it is bound to `tcp://`, and the redeemed
    /// token that comes back is what actually authenticates every request
    /// this connection ever makes.
    pub async fn add_connection(
        &self,
        name: String,
        url: String,
        code: String,
    ) -> Result<ConnectionConfig> {
        let device_id = device_id(self.deps.settings.as_ref())?;
        // Tokenless and thrown away after this one call: there is nothing to
        // authenticate this probe with, since the token it is fetching is the
        // only one that will ever exist for this pairing.
        let pairing_probe = HttpTransport::new(url.clone(), None);
        let token = redeem_pairing_code(&pairing_probe, &code, &device_id, DEVICE_LABEL).await?;

        let id = uuid::Uuid::new_v4().to_string();
        let conn = RemoteConnection::new(id.clone(), name, url, token.clone()).await?;
        let config = conn.config().clone();

        self.deps.secrets.set(&id, &token)?;

        let settings = self.deps.settings.as_ref();
        let mut configs = load_remote_configs(settings);
        configs.push(config.clone());
        save_remote_configs(settings, &configs)?;

        self.emit_connection_changed().await;
        Ok(config)
    }

    pub async fn remove_connection(
        &self,
        id: &str,
    ) -> Result<()> {
        if id == "local" {
            return Err(Error::CannotRemoveLocal);
        }
        let settings = self.deps.settings.as_ref();
        let mut configs = load_remote_configs(settings);
        configs.retain(|c| c.id != id);
        save_remote_configs(settings, &configs)?;
        let _ = self.deps.secrets.delete(id);
        self.emit_connection_changed().await;
        Ok(())
    }

    /// Make `id` the active connection: build it, start it, and only then retire
    /// the one it replaces. A switch that cannot be completed leaves the current
    /// connection and its streams exactly as they were.
    pub async fn switch_connection(
        &self,
        id: &str,
    ) -> Result<()> {
        self.switch_to(async move {
            // Build first, then start, and only then hand the connection over
            // to be installed: a switch that cannot be completed must leave the
            // current connection and its streams exactly as they were, and
            // `switch_to` never touches `active` until this future is Ok.
            let conn = self.build_connection(id).await?;
            let status = conn.start(self.deps.events.as_ref()).await;
            Ok((conn, status))
        })
        .await
        .map(|_announce| ())
    }

    async fn build_connection(
        &self,
        id: &str,
    ) -> Result<Box<dyn QuiverConnection>> {
        if id == "local" {
            return Ok(Box::new(LocalConnection::new(self.deps.local.clone())));
        }
        let configs = load_remote_configs(self.deps.settings.as_ref());
        let cfg = configs
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| Error::ConnectionNotFound(id.to_string()))?;
        let token = self.deps.secrets.get(id)?;
        let conn = RemoteConnection::new(
            cfg.id.clone(),
            cfg.name.clone(),
            cfg.url.clone().unwrap_or_default(),
            token,
        )
        .await?;
        Ok(Box::new(conn))
    }

    /// The lock discipline of a switch, with building and starting the new
    /// connection lifted out into `prepare`, which can then be a stand-in under
    /// test: which lock is held across what is the half that wedged the app, and
    /// all of that is here, and drivable.
    ///
    /// Three properties, in the order they matter:
    ///
    ///   1. `switching` is held for the whole switch, so two switches cannot
    ///      interleave. It replaces the write lock in that role — see the field.
    ///   2. NO lock on `active` is held across `prepare`. It dials a peer;
    ///      a peer that accepts and then stalls makes it take forever, and a
    ///      write lock held across that blocks every subsequent reader too,
    ///      because tokio's `RwLock` is fair.
    ///   3. A failing `prepare` returns before `active` is touched at all, so
    ///      the previous connection and its streams survive the failure intact.
    ///
    /// Returns the background self-announce, when the new connection is the local
    /// one and became ready.
    async fn switch_to<F>(
        &self,
        prepare: F,
    ) -> Result<Option<JoinHandle<()>>>
    where
        F: std::future::Future<Output = Result<(Box<dyn QuiverConnection>, CoreStatus)>>,
    {
        let _switching = self.switching.lock().await;
        let (new_conn, status) = prepare.await?;
        let mut guard = self.active.write().await;
        retire_streams_and_teardown(&self.deps.bridge, &**guard).await;
        *guard = new_conn;
        Ok(self.announce_on_ready(&**guard, status))
    }

    pub async fn rename_connection(
        &self,
        id: &str,
        name: String,
    ) -> Result<()> {
        if id == "local" {
            return Err(Error::CannotRenameLocal);
        }
        let settings = self.deps.settings.as_ref();
        let mut configs = load_remote_configs(settings);
        match configs.iter_mut().find(|c| c.id == id) {
            Some(cfg) => cfg.name = name.clone(),
            None => return Err(Error::ConnectionNotFound(id.to_string())),
        }
        save_remote_configs(settings, &configs)?;
        // Keep active in-memory config in sync. `switching` first, like every
        // other writer of `active` — see `start`: a writer that waits on the
        // RwLock itself blocks every reader behind it.
        let _switching = self.switching.lock().await;
        {
            let mut guard = self.active.write().await;
            if guard.config().id == id {
                guard.set_name(name);
            }
        }
        self.emit_connection_changed().await;
        Ok(())
    }

    pub async fn transport(&self) -> Arc<dyn Transport> {
        self.active.read().await.transport()
    }

    /// Tear the active connection down because the app is exiting.
    ///
    /// The one thing this reaps today is the local daemon: the manager spawns it and
    /// nothing else can kill it (see `SidecarManager::reap`), so without a call on
    /// the exit path every quit leaves a `quiver daemon` behind for the session.
    /// The app's exit handler is the caller, and it can arrive twice (a window
    /// close request and the exit itself), which is why everything underneath is
    /// idempotent.
    ///
    /// The READ lock, not `switching` and not the write lock. Two reasons, both
    /// about not hanging a quit:
    ///
    ///   * `switching` is held for the whole of a switch, and a switch to a peer
    ///     that accepts and then stalls holds it for `HEALTH_TIMEOUT`. Waiting on
    ///     it here would make quitting mid-switch wait too, for no benefit: a
    ///     switch in flight has not replaced `active` yet, so the connection this
    ///     reads is the one whose daemon needs killing either way.
    ///   * a reader is safe to take because no lock on `active` is ever held
    ///     across an unbounded await — the invariant `switch_to` documents and
    ///     `a_switch_that_never_finishes_preparing_still_leaves_the_app_readable`
    ///     enforces. The write lock is only held across
    ///     `retire_streams_and_teardown`, which now ends in a `kill()`, and that
    ///     is bounded by the OS.
    pub async fn shutdown(&self) {
        self.active.read().await.teardown().await;
    }
}

#[cfg(test)]
mod tests;
