//! Building every service and tearing them down.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use quiver_arrow_host::ArrowHosts;
use quiver_core_client::bridge::WsBridgeManager;
use quiver_core_client::console::ConsoleExecManager;
use quiver_core_client::self_announce::LocalSelfAnnounce;
use quiver_core_client::stores::SecretStore;
use quiver_core_client::{
    ConnectionManager, LocalConfig, ManagerDeps, SelfAnnounce, SelfAnnounceConfig,
};
use quiver_platform::{fdlimit, platform_key, AppPaths};
use quiver_state::connection::{
    setup_connection_listeners, Backend, ConnectionListeners, ConnectionStore,
};
use quiver_state::core_api::CoreApi;
use quiver_state::listeners::{setup_listeners, ListenerDeps, Listeners};
use quiver_state::persistence::{EntityCache, KeyValueStore};
use quiver_state::queries::QueryClient;
use quiver_state::store::{ArrowStore, StatusStore};
use quiver_state::transport::ws_manager::WsManager;
use tokio::runtime::{Handle, Runtime};

use super::announce::{NoAnnounce, StoredRows};
use super::arrow_host::CoreUpstream;
use super::arrow_windows::{ArrowWindows, HelperLauncher, ProcessLauncher};
use super::backend::NativeBackend;
use super::core_api::TransportCoreApi;
use super::events::ChannelEvents;
use super::release_vars::{NativeReleaseSource, ReleaseSource};
use super::secrets::KeyringSecrets;
use super::settings::{FileKeyValue, FileSettings};
use super::{Error, Result};

/// File (under the app data dir) behind `quiver-state`'s string store: the cache version marker,
/// the cached catalog and the self-announced rows.
const STATE_FILE: &str = "state.json";

/// Everything that differs between the real app and a test.
pub struct ServicesConfig {
    /// Settings files (`connections.json`, `device.json`, `state.json`) live here.
    pub data_dir: PathBuf,
    pub local: LocalConfig,
    pub secrets: Arc<dyn SecretStore>,
    /// `None` when the helper binary is missing: arrow apps then fail to open, nothing else.
    pub launcher: Option<Arc<dyn HelperLauncher>>,
    pub release: Arc<dyn ReleaseSource>,
    /// This machine's `"os/arch"` key.
    pub platform: String,
}

impl ServicesConfig {
    /// The real app: OS directories, the OS keyring, the daemon bundled next to this executable.
    pub fn from_environment() -> Result<Self> {
        let paths = AppPaths::resolve()?;
        Ok(Self {
            data_dir: paths.data_dir().to_path_buf(),
            local: LocalConfig::new(
                dev_checkout().as_deref(),
                LocalConfig::bundled_next_to_current_exe(),
            ),
            secrets: Arc::new(KeyringSecrets::keyring()),
            launcher: ProcessLauncher::next_to_current_exe()
                .map(|launcher| Arc::new(launcher) as Arc<dyn HelperLauncher>),
            release: Arc::new(NativeReleaseSource::new()),
            platform: platform_key(),
        })
    }
}

/// A development build points its daemon at this checkout (socket, pipe and `.quiver` home are
/// scoped to it), so it never touches the user's real installation; a shipped build is never
/// scoped. The compile-time anchor, like the Tauri app's `CARGO_MANIFEST_DIR`: a launcher that
/// does not inherit the terminal's cwd (`open` on macOS) still finds the checkout.
fn dev_checkout() -> Option<String> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .ancestors()
        .nth(2)
        .map(|root| root.to_string_lossy().into_owned())
}

/// All the long-lived services, shared with the views.
pub struct AppServices {
    pub manager: Arc<ConnectionManager>,
    pub events: Arc<ChannelEvents>,
    pub api: Arc<dyn CoreApi>,
    pub backend: Arc<dyn Backend>,
    pub ws: WsManager,
    pub arrows: ArrowStore,
    pub status: StatusStore,
    pub connections: ConnectionStore,
    pub queries: QueryClient,
    pub cache: Arc<EntityCache>,
    pub key_value: Arc<dyn KeyValueStore>,
    pub arrow_windows: ArrowWindows,
    pub release: Arc<dyn ReleaseSource>,
    pub console: Arc<ConsoleExecManager>,
    handle: Handle,
    /// Held while shutting down; `true` once it has completed. A second caller waits for the first.
    shut_down: tokio::sync::Mutex<bool>,
    /// The state listeners; dropping them closes their streams.
    listeners: Mutex<Option<(Listeners, ConnectionListeners)>>,
    /// Only when `start()` created the runtime.
    runtime: Mutex<Option<Runtime>>,
}

impl AppServices {
    /// The app's entry point: raise the descriptor limit, start a runtime and build everything.
    /// The local daemon is brought up in the background; watch `status` for it.
    pub fn start() -> Result<Self> {
        let outcome = fdlimit::raise();
        if matches!(outcome, fdlimit::Outcome::Failed(_)) {
            log::warn!("{outcome}");
        } else {
            log::info!("{outcome}");
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("quiver-runtime")
            .build()
            .map_err(|e| Error::Runtime(e.to_string()))?;
        let services = runtime.block_on(Self::build(ServicesConfig::from_environment()?));
        *services
            .runtime
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(runtime);
        Ok(services)
    }

    /// Builds the services on the current runtime and starts the active (local) connection in the
    /// background.
    pub async fn build(config: ServicesConfig) -> Self {
        let events = Arc::new(ChannelEvents::new());
        let settings = Arc::new(FileSettings::new(&config.data_dir));
        let key_value: Arc<dyn KeyValueStore> =
            Arc::new(FileKeyValue::open(config.data_dir.join(STATE_FILE)));
        let arrows = ArrowStore::default();

        // The only announcer: the manager calls it for the LOCAL connection only.
        let on_adopted = {
            let arrows = arrows.clone();
            Arc::new(move || arrows.refresh_catalog())
        };
        let build = super::release_vars::ReleaseSource::build_info(config.release.as_ref());
        let announce: Arc<dyn SelfAnnounce> = Arc::new(LocalSelfAnnounce::new(
            SelfAnnounceConfig {
                build_tag: build.tag,
                build_channel: build.channel,
            },
            Some(Arc::new(StoredRows::local(key_value.clone()))),
            Some(on_adopted),
        ));

        let manager = Arc::new(ConnectionManager::new(ManagerDeps {
            local: config.local,
            settings,
            secrets: config.secrets,
            events: events.clone(),
            bridge: Arc::new(WsBridgeManager::new()),
            self_announce: Some(announce),
        }));
        let api: Arc<dyn CoreApi> = Arc::new(TransportCoreApi::new(manager.clone()));
        let backend: Arc<dyn Backend> = Arc::new(NativeBackend::new(
            manager.clone(),
            events.clone(),
            config.platform,
        ));

        let status = StatusStore::default();
        let connections = ConnectionStore::default();
        let queries = QueryClient::new();
        let cache = Arc::new(EntityCache::with_persistence(key_value.clone()));
        let ws = WsManager::new(api.clone());

        // Subscribe before the connection starts so no status event is missed.
        let connection_listeners =
            setup_connection_listeners(backend.clone(), connections.clone()).await;
        let listeners = setup_listeners(ListenerDeps {
            api: api.clone(),
            ws: ws.clone(),
            backend: backend.clone(),
            cache: cache.clone(),
            key_value: key_value.clone(),
            arrows: arrows.clone(),
            status: status.clone(),
            queries: Some(queries.clone()),
            announce: Arc::new(NoAnnounce),
        })
        .await;
        let start = manager.clone();
        tokio::spawn(async move { start.start().await });

        let upstream = Arc::new(CoreUpstream::new(manager.clone()));
        let launcher = config
            .launcher
            .unwrap_or_else(|| Arc::new(MissingHelper) as Arc<dyn HelperLauncher>);
        let arrow_windows = ArrowWindows::new(launcher, Arc::new(ArrowHosts::new()), upstream);

        Self {
            manager,
            events,
            api,
            backend,
            ws,
            arrows,
            status,
            connections,
            queries,
            cache,
            key_value,
            arrow_windows,
            release: config.release,
            console: Arc::new(ConsoleExecManager::new()),
            handle: Handle::current(),
            shut_down: tokio::sync::Mutex::new(false),
            listeners: Mutex::new(Some((listeners, connection_listeners))),
            runtime: Mutex::new(None),
        }
    }

    /// The runtime the services run on, for views that need to spawn work.
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Closes every arrow window and stream and stops the local daemon this app started (one it
    /// merely found running is left alone). Idempotent, and a concurrent second call waits for the
    /// first. Returns when the daemon has been reaped.
    pub async fn shutdown_async(&self) {
        let mut done = self.shut_down.lock().await;
        if *done {
            return;
        }
        self.arrow_windows.shutdown();
        self.listeners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        self.console.cancel_all();
        self.manager.bridge().close_all();
        self.manager.shutdown().await;
        *done = true;
    }

    /// [`Self::shutdown_async`] for the thread that owns the window (never a runtime thread).
    /// Call it on window close and on quit.
    pub fn shutdown(&self) {
        self.handle.block_on(self.shutdown_async());
    }
}

impl Drop for AppServices {
    fn drop(&mut self) {
        let runtime = self
            .runtime
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        // Only a services object that owns its runtime can block on it; one built on someone
        // else's runtime must have been shut down by them.
        if let Some(runtime) = runtime {
            runtime.block_on(self.shutdown_async());
            runtime.shutdown_background();
        }
    }
}

/// The launcher when the helper binary is not installed.
struct MissingHelper;

impl HelperLauncher for MissingHelper {
    fn launch(
        &self,
        _: &super::arrow_windows::HelperArgs,
    ) -> Result<super::arrow_windows::HelperLink> {
        Err(Error::Helper(format!(
            "{} was not found next to the application",
            super::arrow_windows::HELPER_BINARY
        )))
    }
}
