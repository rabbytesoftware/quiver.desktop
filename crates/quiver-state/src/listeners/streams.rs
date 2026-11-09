use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value;
use tokio::task::JoinHandle;

use super::announce::{announce_if_local, SelfAnnounceHook};
use crate::connection::Backend;
use crate::core_api::{CoreApi, CoreApiExt};
use crate::domain::{ConnectionStatus, LastReturn, Outcome, RuntimeUpdate};
use crate::dto::arrow::{to_arrow_catalog_records, to_initial_runtime_updates, ArrowListResponseItemDto};
use crate::dto::runtime::{to_runtime_update, RuntimeUpdateDto};
use crate::persistence::entity_stream::{subscribe_arrow_stream, ArrowStream, Reseeder, SubscribeArrowStreamOptions};
use crate::persistence::version::maybe_wipe_on_version_change;
use crate::persistence::{EntityCache, KeyValueStore};
use crate::queries::keys::{arrow_detail_prefix, collection_prefix};
use crate::queries::QueryClient;
use crate::store::{ArrowStore, StatusStore};
use crate::transport::ws_manager::{is_reconnect_sentinel, Subscription, WsManager};

const RUNTIME_ENDPOINT: &str = "/v0/runtime";
/// Arrows that are not in the library: core pushes one here when it refreshes a manifest it had
/// served from an expired cache, so a screen that showed the old one re-reads. The library stream
/// (`user_installed` defaulting to true) never carries these.
const REFRESHED_ENDPOINT: &str = "/v0/arrow?user_installed=false";

pub struct ListenerDeps {
    pub api: Arc<dyn CoreApi>,
    pub ws: WsManager,
    pub backend: Arc<dyn Backend>,
    pub cache: Arc<EntityCache>,
    /// Where the cache-version marker lives (`maybe_wipe_on_version_change`).
    pub key_value: Arc<dyn KeyValueStore>,
    pub arrows: ArrowStore,
    pub status: StatusStore,
    /// Absent when nothing renders arrow details or collections, so there is nothing to refresh.
    pub queries: Option<QueryClient>,
    pub announce: Arc<dyn SelfAnnounceHook>,
}

pub(super) fn is_successful_update(last_return: Option<&LastReturn>) -> bool {
    last_return.is_some_and(|r| r.outcome == Outcome::Success && r.method.strip_prefix('_').unwrap_or(&r.method) == "update")
}

/// Everything one connection's streams hold open. Dropping it closes them.
struct Streams {
    _arrow: ArrowStream,
    _runtime: Subscription,
    _refresh: Option<Subscription>,
}

#[derive(Default)]
struct Lifecycle {
    start_pending: u32,
    streams: Option<Streams>,
}

struct Inner {
    deps: ListenerDeps,
    /// Bumped on every `starting`: any start that began under an older generation must not land.
    generation: Arc<AtomicU64>,
    lifecycle: Mutex<Lifecycle>,
}

/// Dropping it stops the status loop and closes the streams.
pub struct Listeners {
    inner: Arc<Inner>,
    task: JoinHandle<()>,
}

impl Drop for Listeners {
    fn drop(&mut self) {
        self.task.abort();
        self.inner.stop_streams();
    }
}

struct PendingGuard<'a>(&'a Inner);

impl<'a> PendingGuard<'a> {
    fn new(inner: &'a Inner) -> Self {
        inner.lifecycle().start_pending += 1;
        Self(inner)
    }
}

impl Drop for PendingGuard<'_> {
    fn drop(&mut self) {
        self.0.lifecycle().start_pending -= 1;
    }
}

/// Starts listening to core status and adopts an already-running core. Returns once that adoption
/// has settled (like the TS `await setupListeners()`).
pub async fn setup_listeners(deps: ListenerDeps) -> Listeners {
    maybe_wipe_on_version_change(&deps.cache, deps.key_value.as_ref());
    let mut statuses = deps.backend.on_core_status();
    let inner = Arc::new(Inner { deps, generation: Arc::new(AtomicU64::new(0)), lifecycle: Mutex::default() });
    let looping = inner.clone();
    let task = tokio::spawn(async move {
        while let Some(status) = statuses.recv().await {
            looping.on_status(status);
        }
    });
    inner.adopt_running_core().await;
    Listeners { inner, task }
}

impl Inner {
    fn lifecycle(&self) -> std::sync::MutexGuard<'_, Lifecycle> {
        self.lifecycle.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    fn stop_streams(&self) {
        let streams = self.lifecycle().streams.take();
        self.deps.arrows.set_catalog_refresh(|| {});
        drop(streams);
    }

    fn spawn_announce(&self) {
        tokio::spawn(announce_if_local(self.deps.backend.clone(), self.deps.announce.clone()));
    }

    /// The synchronous half of a status event, so events are handled in the order they arrive;
    /// whatever has to wait is spawned.
    fn on_status(self: &Arc<Self>, status: ConnectionStatus) {
        if status == ConnectionStatus::Ready {
            // Captured before begin_streams's own awaits, not read afterwards: it must see the
            // generation this ready belongs to, even if a later `starting` (which bumps it) lands
            // before it finishes. The announce is fire-and-forget on purpose -- it never blocks
            // (or is blocked by) the primary catalog load begin_streams performs.
            let generation = self.generation();
            self.spawn_announce();
            let this = self.clone();
            tokio::spawn(async move {
                if let Err(err) = this.begin_streams(generation).await {
                    log::error!("core-store: could not start the streams: {err}");
                    return;
                }
                this.set_ready_if_current(generation);
            });
            return;
        }
        self.deps.status.set_status(status);
        if status == ConnectionStatus::Starting {
            self.generation.fetch_add(1, Ordering::SeqCst);
            self.stop_streams();
            self.deps.arrows.reset();
        }
    }

    /// The store must not claim ready before the streams exist: the runtime subscription
    /// begin_streams just opened is the ONLY channel that confirms an action's progress, and the
    /// daemon does not replay a broadcast fired before a subscriber existed. Reporting ready any
    /// earlier lets a UI gated on this status act before anything is listening for the result, and
    /// that action's outcome is then lost until a full reload re-seeds from a REST GET. Guarded on
    /// generation for the same reason begin_streams itself is: a `starting` that superseded this
    /// ready while it awaited must not resurrect it as ready afterwards.
    fn set_ready_if_current(&self, generation: u64) {
        if self.generation() == generation {
            self.deps.status.set_status(ConnectionStatus::Ready);
        }
    }

    async fn begin_streams(self: &Arc<Self>, generation: u64) -> crate::Result<()> {
        let _pending = PendingGuard::new(self);
        let snapshot = self.deps.backend.get_connections().await?;
        if self.generation() != generation {
            return Ok(());
        }
        self.start_streams(&snapshot.active_id);
        Ok(())
    }

    async fn adopt_running_core(self: &Arc<Self>) {
        let generation = self.generation();
        if !self.deps.api.is_reachable().await {
            return;
        }
        {
            let lifecycle = self.lifecycle();
            if self.generation() != generation || lifecycle.start_pending > 0 || lifecycle.streams.is_some() {
                return;
            }
        }
        self.spawn_announce();
        if let Err(err) = self.begin_streams(generation).await {
            log::error!("core-store: failed to adopt an already-running core: {err}");
        }
        // Only once the runtime stream this generation wanted is actually subscribed.
        self.set_ready_if_current(generation);
    }

    fn start_streams(self: &Arc<Self>, connection_id: &str) {
        self.stop_streams();
        let deps = &self.deps;
        let stream_generation = self.generation();
        let pending_initial: Arc<Mutex<Vec<RuntimeUpdate>>> = Arc::default();
        let reseeder: Arc<Mutex<Option<Reseeder>>> = Arc::default();

        let seed = {
            let (api, pending, connection_id) = (deps.api.clone(), pending_initial.clone(), connection_id.to_string());
            Arc::new(move || {
                let (api, pending, connection_id) = (api.clone(), pending.clone(), connection_id.clone());
                Box::pin(async move {
                    let items = api.get::<Vec<ArrowListResponseItemDto>>("/v0/arrow?user_installed=true").await?;
                    *pending.lock().unwrap_or_else(PoisonError::into_inner) = to_initial_runtime_updates(&items);
                    Ok(to_arrow_catalog_records(&items, &connection_id))
                }) as futures_util::future::BoxFuture<'static, _>
            })
        };
        let on_change = {
            let (cache, arrows, generation, pending, connection_id) =
                (deps.cache.clone(), deps.arrows.clone(), self.generation.clone(), pending_initial.clone(), connection_id.to_string());
            Arc::new(move || {
                let records = cache.arrows_for(&connection_id);
                if generation.load(Ordering::SeqCst) != stream_generation {
                    return;
                }
                arrows.set_catalog(&records);
                let batch = std::mem::take(&mut *pending.lock().unwrap_or_else(PoisonError::into_inner));
                let visible: std::collections::HashSet<&str> = records.iter().map(|r| r.namespace.as_str()).collect();
                for update in batch {
                    if visible.contains(update.namespace.as_str()) {
                        arrows.seed_initial_state(update);
                    }
                }
            })
        };
        let on_seed_error = {
            let (arrows, generation) = (deps.arrows.clone(), self.generation.clone());
            Arc::new(move |_: &crate::Error| {
                if generation.load(Ordering::SeqCst) == stream_generation {
                    arrows.set_catalog_error();
                }
            })
        };
        let on_unversioned_upsert = {
            let (reseeder, queries) = (reseeder.clone(), deps.queries.clone());
            Arc::new(move |_: &str| {
                if let Some(r) = reseeder.lock().unwrap_or_else(PoisonError::into_inner).as_ref() {
                    r.reseed();
                }
                // An update's commit runs after its run has ended, so the detail the run-end
                // refetch read can still carry the old `available`. This frame is core announcing
                // the commit; no runtime frame follows it, so only a re-read here can drop the
                // update button.
                if let Some(queries) = &queries {
                    queries.invalidate(&arrow_detail_prefix());
                }
            })
        };
        let arrow_stream = subscribe_arrow_stream(
            &deps.ws,
            deps.cache.clone(),
            SubscribeArrowStreamOptions {
                connection_id: connection_id.to_string(),
                seed,
                on_change: Some(on_change),
                on_seed_error: Some(on_seed_error),
                on_unversioned_upsert: Some(on_unversioned_upsert),
            },
        );
        let handle = arrow_stream.reseeder();
        *reseeder.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle.clone());
        deps.arrows.set_catalog_refresh(move || handle.reseed());

        // A reconnect may have dropped frames, so the sentinel re-reads too.
        let refresh = deps.queries.clone().map(|queries| {
            deps.ws.subscribe(REFRESHED_ENDPOINT, move |_| {
                queries.invalidate(&arrow_detail_prefix());
                queries.invalidate(&collection_prefix());
            })
        });

        let arrows = deps.arrows.clone();
        let runtime = deps.ws.subscribe(RUNTIME_ENDPOINT, move |data| on_runtime_frame(&arrows, data));
        self.lifecycle().streams = Some(Streams { _arrow: arrow_stream, _runtime: runtime, _refresh: refresh });
    }
}

fn on_runtime_frame(arrows: &ArrowStore, data: &Value) {
    if is_reconnect_sentinel(data) {
        return;
    }
    let update = match serde_json::from_value::<RuntimeUpdateDto>(data.clone()) {
        Ok(dto) => to_runtime_update(dto),
        Err(err) => {
            log::warn!("core-store: ignoring a malformed runtime frame: {err}");
            return;
        }
    };
    let was_running = arrows.entry(&update.namespace).is_some_and(|e| e.active_run.is_some());
    let finished_update = was_running && update.active_run.is_none() && is_successful_update(update.last_return.as_ref());
    arrows.apply_runtime_update(update);
    // A successful update moves the row's resolved ref, which only the catalog read carries;
    // whatever page is open, the sidebar follows.
    if finished_update {
        arrows.refresh_catalog();
    }
}
