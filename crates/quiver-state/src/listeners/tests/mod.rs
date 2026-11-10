use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::{mpsc, Notify};

use super::streams::{setup_listeners, ListenerDeps, Listeners};
use super::SelfAnnounceHook;
use crate::connection::{ConnectionsSnapshot, FakeBackend};
use crate::core_api::{FakeCoreApi, FakeStreamHandle, Method};
use crate::domain::{
    local_connection, ArrowState, ConnectionConfig, ConnectionKind, ConnectionStatus, Outcome,
};
use crate::persistence::version::{CACHE_VERSION_KEY, QUIVER_CACHE_VERSION};
use crate::persistence::{EntityCache, InMemoryKeyValueStore, KeyValueStore};
use crate::queries::keys::{arrow_detail_key, collection_key};
use crate::queries::QueryClient;
use crate::store::{ArrowStore, StatusStore};
use crate::transport::ws_manager::WsManager;

const LIST: &str = "/v0/arrow?user_installed=true";

struct Hook {
    announced: mpsc::UnboundedSender<()>,
    /// When set, the announce parks until released: it must never hold up the streams.
    park: Option<Arc<Notify>>,
}

#[async_trait]
impl SelfAnnounceHook for Hook {
    async fn announce_self(&self) {
        let _ = self.announced.send(());
        if let Some(park) = &self.park {
            park.notified().await;
        }
    }
}

struct Rig {
    api: FakeCoreApi,
    backend: Arc<FakeBackend>,
    cache: Arc<EntityCache>,
    kv: Arc<InMemoryKeyValueStore>,
    arrows: ArrowStore,
    status: StatusStore,
    queries: QueryClient,
    announced: mpsc::UnboundedReceiver<()>,
    announced_tx: mpsc::UnboundedSender<()>,
}

fn rig() -> Rig {
    let api = FakeCoreApi::new();
    api.route(
        Method::Get,
        LIST,
        json!([{"namespace": "a", "name": "A", "description": "", "tags": null,
                "versions": [{"ref": "v1", "resolved_ref": "1", "state": "ready"}]}]),
    );
    let (announced_tx, announced) = mpsc::unbounded_channel();
    let kv = Arc::new(InMemoryKeyValueStore::new());
    kv.set(CACHE_VERSION_KEY, QUIVER_CACHE_VERSION).unwrap();
    Rig {
        api,
        backend: Arc::new(FakeBackend::new()),
        cache: Arc::new(EntityCache::in_memory()),
        kv,
        arrows: ArrowStore::default(),
        status: StatusStore::default(),
        queries: QueryClient::new(),
        announced,
        announced_tx,
    }
}

impl Rig {
    async fn setup(&self, park: Option<Arc<Notify>>, with_queries: bool) -> Listeners {
        setup_listeners(ListenerDeps {
            api: Arc::new(self.api.clone()),
            ws: WsManager::new(Arc::new(self.api.clone())),
            backend: self.backend.clone(),
            cache: self.cache.clone(),
            key_value: self.kv.clone(),
            arrows: self.arrows.clone(),
            status: self.status.clone(),
            queries: with_queries.then(|| self.queries.clone()),
            announce: Arc::new(Hook {
                announced: self.announced_tx.clone(),
                park,
            }),
        })
        .await
    }

    async fn wait_status(&self, wanted: ConnectionStatus) {
        let mut rx = self.status.store().subscribe();
        while self.status.status() != wanted {
            rx.changed().await.unwrap();
        }
    }

    async fn wait_arrows(&self, what: impl Fn(&ArrowStore) -> bool) {
        let mut rx = self.arrows.store().subscribe();
        while !what(&self.arrows) {
            rx.changed().await.unwrap();
        }
    }

    /// Boots with the core not up yet, so the one start comes from the `ready` event alone.
    async fn ready(&self) -> Listeners {
        self.api.set_reachable(false);
        let listeners = self.setup(None, true).await;
        self.backend.emit_status(ConnectionStatus::Ready);
        self.wait_status(ConnectionStatus::Ready).await;
        self.wait_arrows(|a| a.entry("a@v1").is_some()).await;
        listeners
    }
}

async fn wait_dropped(handle: &FakeStreamHandle) {
    while !handle.is_dropped() {
        tokio::task::yield_now().await;
    }
}

fn runtime_frame(
    state: &str,
    active_run: serde_json::Value,
    last_return: serde_json::Value,
) -> serde_json::Value {
    json!({"namespace": "a@v1", "state": state, "active_run": active_run, "last_return": last_return})
}

#[tokio::test]
async fn ready_starts_the_streams_seeds_the_catalog_and_the_initial_runtime_state() {
    let rig = rig();
    let _l = rig.ready().await;
    let entry = rig.arrows.entry("a@v1").unwrap();
    assert_eq!(
        (entry.version.as_str(), entry.state),
        ("1", ArrowState::Ready),
        "seeded from versions[].state"
    );
    assert_eq!(rig.api.calls_to(Method::Get, LIST), 1);
    rig.api.wait_stream("/v0/runtime", 0).await;
    rig.api.wait_stream("/v0/arrow", 0).await;
}

#[tokio::test]
async fn does_not_report_ready_before_the_runtime_stream_is_subscribed() {
    let rig = rig();
    rig.api.set_reachable(false);
    let _l = rig.setup(None, true).await;
    assert_eq!(rig.status.status(), ConnectionStatus::Starting);
    rig.backend.emit_status(ConnectionStatus::Ready);
    rig.wait_status(ConnectionStatus::Ready).await;
    // By the time ready is visible the subscription exists, so the socket is (being) opened.
    rig.api.wait_stream("/v0/runtime", 0).await;
}

#[tokio::test]
async fn adopts_an_already_running_core_at_boot_and_announces_once() {
    let mut rig = rig();
    let _l = rig.setup(None, true).await;
    assert_eq!(
        rig.status.status(),
        ConnectionStatus::Ready,
        "setup returns once the adoption settled"
    );
    rig.api.wait_stream("/v0/runtime", 0).await;
    rig.announced.recv().await.unwrap();
    assert!(rig.announced.try_recv().is_err());
}

#[tokio::test]
async fn leaves_the_boot_to_the_event_when_the_core_is_not_reachable_yet() {
    let mut rig = rig();
    rig.api.set_reachable(false);
    let _l = rig.setup(None, true).await;
    assert_eq!(rig.status.status(), ConnectionStatus::Starting);
    assert_eq!(rig.api.open_count("/v0/runtime"), 0);
    assert!(rig.announced.try_recv().is_err());
}

#[tokio::test]
async fn adopts_the_connection_the_backend_reports_active() {
    let rig = rig();
    let remote = ConnectionConfig {
        id: "remote-1".into(),
        name: "R".into(),
        kind: ConnectionKind::Remote,
        url: Some("http://h:7070".into()),
        api_version: "v0".into(),
    };
    rig.backend.set_connections(ConnectionsSnapshot {
        connections: vec![local_connection(), remote],
        active_id: "remote-1".into(),
    });
    let _l = rig.setup(None, true).await;
    rig.wait_arrows(|a| a.entry("a@v1").is_some()).await;
    assert_eq!(rig.cache.arrows_for("remote-1").len(), 1);
    assert!(rig.cache.arrows_for("local").is_empty());
}

#[tokio::test]
async fn announces_on_a_local_connection_but_never_on_a_remote_one() {
    let mut rig = rig();
    let _l = rig.setup(None, true).await;
    rig.announced.recv().await.unwrap(); // the boot adoption, on the local connection

    let remote = ConnectionConfig {
        id: "r1".into(),
        name: "R".into(),
        kind: ConnectionKind::Remote,
        url: Some("http://h:7070".into()),
        api_version: "v0".into(),
    };
    rig.backend.set_connections(ConnectionsSnapshot {
        connections: vec![local_connection(), remote],
        active_id: "r1".into(),
    });
    rig.backend.emit_status(ConnectionStatus::Starting);
    rig.backend.emit_status(ConnectionStatus::Ready);
    rig.wait_status(ConnectionStatus::Ready).await;
    rig.api.wait_stream("/v0/runtime", 1).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    assert!(
        rig.announced.try_recv().is_err(),
        "a remote daemon is never announced to"
    );
}

#[tokio::test]
async fn announces_again_on_every_fresh_ready_and_not_while_starting_or_disconnected() {
    let mut rig = rig();
    let _l = rig.ready().await;
    rig.announced.recv().await.unwrap();
    rig.backend.emit_status(ConnectionStatus::Starting);
    rig.backend.emit_status(ConnectionStatus::Disconnected);
    rig.wait_status(ConnectionStatus::Disconnected).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    assert!(
        rig.announced.try_recv().is_err(),
        "neither starting nor disconnected announce"
    );
    rig.backend.emit_status(ConnectionStatus::Ready);
    rig.announced.recv().await.unwrap();
}

#[tokio::test]
async fn a_slow_announce_never_holds_up_the_streams_or_ready() {
    let rig = rig();
    let park = Arc::new(Notify::new());
    let _l = rig.setup(Some(park.clone()), true).await;
    rig.wait_status(ConnectionStatus::Ready).await;
    rig.api.wait_stream("/v0/runtime", 0).await;
    park.notify_waiters();
}

#[tokio::test]
async fn applies_runtime_frames_ignores_unknown_namespaces_and_the_reconnect_sentinel() {
    let rig = rig();
    let _l = rig.ready().await;
    let runtime = rig.api.wait_stream("/v0/runtime", 0).await;
    runtime.send_json(json!({"namespace": "ghost@1", "state": "running"}));
    runtime.send_json(json!({"reconnected": true}));
    runtime.send_json(json!({"not": "a runtime frame"}));
    runtime.send_json(runtime_frame("running", json!(null), json!(null)));
    rig.wait_arrows(|a| {
        a.entry("a@v1")
            .is_some_and(|e| e.state == ArrowState::Running)
    })
    .await;
    assert_eq!(
        rig.arrows.entries().len(),
        1,
        "no entry for a runtime frame naming an unknown namespace"
    );
}

/// A self-update swap replaces the daemon: the runtime stream is gone with it and never replays, so
/// "updating" (and "running" for a supervised arrow the swap detached) would stay forever. The
/// socket's reconnect re-reads the list, and that read must win over the frames seen before.
#[tokio::test]
async fn a_reconnect_replaces_the_runtime_state_the_live_frames_had_painted() {
    let rig = rig();
    let _l = rig.ready().await;
    let runtime = rig.api.wait_stream("/v0/runtime", 0).await;
    let run = json!({"method": "_update", "variables": {}, "steps": []});
    runtime.send_json(runtime_frame("updating", run, json!(null)));
    rig.wait_arrows(|a| {
        a.entry("a@v1")
            .is_some_and(|e| e.state == ArrowState::Updating)
    })
    .await;
    rig.api.route(
        Method::Get,
        LIST,
        json!([{"namespace": "a", "name": "A", "description": "", "tags": null,
                "versions": [{"ref": "v1", "resolved_ref": "2", "state": "detached"}]}]),
    );
    let arrow = rig.api.wait_stream("/v0/arrow", 0).await;
    arrow.send_json(json!({"reconnected": true}));
    rig.wait_arrows(|a| {
        a.entry("a@v1")
            .is_some_and(|e| e.state == ArrowState::Detached)
    })
    .await;
    assert!(rig.arrows.entry("a@v1").unwrap().active_run.is_none());
}

#[tokio::test]
async fn a_reseed_without_a_reconnect_keeps_the_state_live_frames_painted() {
    let rig = rig();
    let _l = rig.ready().await;
    let runtime = rig.api.wait_stream("/v0/runtime", 0).await;
    runtime.send_json(runtime_frame("running", json!(null), json!(null)));
    rig.wait_arrows(|a| {
        a.entry("a@v1")
            .is_some_and(|e| e.state == ArrowState::Running)
    })
    .await;
    rig.arrows.refresh_catalog();
    let mut rx = rig.arrows.store().subscribe();
    while rig.api.calls_to(Method::Get, LIST) < 2 {
        rx.changed().await.unwrap();
    }
    assert_eq!(rig.arrows.entry("a@v1").unwrap().state, ArrowState::Running);
}

#[tokio::test]
async fn a_successful_update_run_ending_re_reads_the_catalog() {
    let rig = rig();
    let _l = rig.ready().await;
    let runtime = rig.api.wait_stream("/v0/runtime", 0).await;
    runtime.send_json(runtime_frame(
        "updating",
        json!({"method": "_update", "variables": {}, "steps": []}),
        json!(null),
    ));
    rig.wait_arrows(|a| a.entry("a@v1").is_some_and(|e| e.active_run.is_some()))
        .await;
    let before = rig.arrows.store().revision();
    runtime.send_json(runtime_frame(
        "ready",
        json!(null),
        json!({"method": "_update", "outcome": "success"}),
    ));
    let mut rx = rig.arrows.store().subscribe();
    while rig.api.calls_to(Method::Get, LIST) < 2 {
        rx.changed().await.unwrap();
    }
    assert!(rig.arrows.store().revision() > before);
    assert_eq!(
        rig.arrows
            .entry("a@v1")
            .unwrap()
            .last_return
            .unwrap()
            .outcome,
        Outcome::Success
    );
}

#[test]
fn only_a_successful_update_outcome_counts() {
    use crate::domain::LastReturn;
    let lr = |method: &str, outcome| {
        Some(LastReturn {
            method: method.into(),
            outcome,
        })
    };
    assert!(super::streams::is_successful_update(
        lr("update", Outcome::Success).as_ref()
    ));
    assert!(super::streams::is_successful_update(
        lr("_update", Outcome::Success).as_ref()
    ));
    assert!(!super::streams::is_successful_update(
        lr("update", Outcome::Failed).as_ref()
    ));
    assert!(!super::streams::is_successful_update(
        lr("update", Outcome::Cancelled).as_ref()
    ));
    assert!(!super::streams::is_successful_update(
        lr("install", Outcome::Success).as_ref()
    ));
    assert!(!super::streams::is_successful_update(None));
}

mod lifecycle;
