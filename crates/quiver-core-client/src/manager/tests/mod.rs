use super::*;
use crate::bridge::FrameSink;
use crate::stores::{MemorySecretStore, MemorySettingsStore};
use crate::transport::http::HttpTransport;
use crate::types::ConnectionKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

mod announce;
mod crud;
mod switching;

/// Stands in for a connection either side of a switch. The one thing a real
/// one does that these tests do not exercise is `start` against a daemon, and
/// `switch_to` takes that half as a future precisely so it does not have to
/// be reached here. `start` reports `Ready`.
struct StubConnection {
    config: ConnectionConfig,
    torn_down: Arc<AtomicBool>,
    transport: Arc<dyn Transport>,
}

impl StubConnection {
    /// `id` is what a test reads back to tell which connection is active.
    /// The transport is real but points nowhere: `switch_to` never dials,
    /// and the tests below only need `transport()` to RESOLVE — that it
    /// resolves at all, while a switch is mid-flight, is the property
    /// finding B is about.
    fn new(
        id: &str,
        torn_down: Arc<AtomicBool>,
    ) -> Self {
        Self::of_kind(id, ConnectionKind::Local, torn_down)
    }

    fn of_kind(
        id: &str,
        kind: ConnectionKind,
        torn_down: Arc<AtomicBool>,
    ) -> Self {
        Self {
            config: ConnectionConfig {
                id: id.into(),
                name: id.into(),
                kind,
                url: None,
                api_version: "v0".into(),
            },
            torn_down,
            transport: Arc::new(HttpTransport::new("http://127.0.0.1:1", None)),
        }
    }

    fn boxed(
        id: &str,
        torn_down: Arc<AtomicBool>,
    ) -> Box<dyn QuiverConnection> {
        Box::new(Self::new(id, torn_down))
    }

    fn boxed_of_kind(
        id: &str,
        kind: ConnectionKind,
    ) -> Box<dyn QuiverConnection> {
        Box::new(Self::of_kind(id, kind, Arc::new(AtomicBool::new(false))))
    }
}

#[async_trait::async_trait]
impl QuiverConnection for StubConnection {
    async fn start(
        &self,
        _events: &dyn EventSink,
    ) -> CoreStatus {
        CoreStatus::Ready
    }

    async fn teardown(&self) {
        self.torn_down.store(true, Ordering::Relaxed);
    }

    fn transport(&self) -> Arc<dyn Transport> {
        Arc::clone(&self.transport)
    }

    fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    fn set_name(
        &mut self,
        name: String,
    ) {
        self.config.name = name;
    }
}

/// Collects what the manager tells the host.
#[derive(Default)]
struct RecordingEvents {
    changes: Mutex<Vec<ConnectionsState>>,
}

impl EventSink for RecordingEvents {
    fn emit_core_status(
        &self,
        _status: CoreStatus,
    ) {
    }

    fn emit_connection_changed(
        &self,
        state: &ConnectionsState,
    ) {
        self.changes.lock().unwrap().push(state.clone());
    }
}

/// What a test needs to look at after driving a manager.
struct Harness {
    manager: ConnectionManager,
    events: Arc<RecordingEvents>,
    settings: Arc<MemorySettingsStore>,
    secrets: Arc<MemorySecretStore>,
}

fn harness(
    active: Box<dyn QuiverConnection>,
    self_announce: Option<Arc<dyn SelfAnnounce>>,
) -> Harness {
    let events = Arc::new(RecordingEvents::default());
    let settings = Arc::new(MemorySettingsStore::new());
    let secrets = Arc::new(MemorySecretStore::new());
    let deps = ManagerDeps {
        local: LocalConfig::new(Some("/tmp/quiver-manager-tests"), None),
        settings: settings.clone(),
        secrets: secrets.clone(),
        events: events.clone(),
        bridge: Arc::new(WsBridgeManager::new()),
        self_announce,
    };
    Harness {
        manager: ConnectionManager::with_connection(active, deps),
        events,
        settings,
        secrets,
    }
}

/// A manager whose active connection is a stub, so the lock discipline can
/// be driven without a daemon. `ConnectionManager::new` would install a real
/// `LocalConnection` instead.
fn manager_with(active: Box<dyn QuiverConnection>) -> ConnectionManager {
    harness(active, None).manager
}

async fn active_id(m: &ConnectionManager) -> String {
    m.active.read().await.config().id.clone()
}

/// Records the frames a stream announced, so the test can assert on what the
/// frontend was NOT told.
struct Recorder(Arc<Mutex<Vec<String>>>);

impl FrameSink for Recorder {
    fn send(
        &self,
        frame: String,
    ) {
        self.0.lock().unwrap().push(frame)
    }
}
