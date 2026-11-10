//! Keeps the [`EntityCache`] in step with the daemon's catalog: a seed (`GET /v0/arrow`) and live
//! frames from the `/v0/arrow` stream, applied strictly in arrival order by one worker.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use futures_util::future::BoxFuture;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use super::entity_cache::EntityCache;
use super::schemas::ArrowCatalogRecord;
use crate::domain::{parse_arrow_origin, parse_inference_confidence, ArrowOrigin};
use crate::transport::ws_manager::{is_reconnect_sentinel, Subscription, WsManager};

const ARROW_ENDPOINT: &str = "/v0/arrow";

pub type ChangeFn = Arc<dyn Fn() + Send + Sync>;
pub type SeedErrorFn = Arc<dyn Fn(&crate::Error) + Send + Sync>;
pub type UpsertFn = Arc<dyn Fn(&str) + Send + Sync>;

pub type SeedFn =
    Arc<dyn Fn() -> BoxFuture<'static, crate::Result<Vec<ArrowCatalogRecord>>> + Send + Sync>;

pub struct SubscribeArrowStreamOptions {
    pub connection_id: String,
    pub seed: SeedFn,
    pub on_change: Option<ChangeFn>,
    pub on_seed_error: Option<SeedErrorFn>,
    /// Called with the namespace of an upsert frame that carries no version -- a row just
    /// registered, or one an update or adoption advanced in place. quiver.core's frames never carry
    /// the resolved ref, so only a re-read of the catalog can supply it.
    pub on_unversioned_upsert: Option<UpsertFn>,
}

enum Cmd {
    Seed {
        generation: u64,
        done: oneshot::Sender<()>,
    },
    Frame(Value),
    Stop,
}

#[derive(Default)]
struct Reseed {
    pending: bool,
    again: bool,
}

#[derive(Default)]
struct Asked {
    /// Namespaces an unversioned frame asked a re-read for.
    asked_for: HashSet<String>,
    /// Those a re-read then showed are not library rows (dependency-only arrows, which the seed
    /// never lists): asking again for those would re-read forever.
    unlisted: HashSet<String>,
}

struct Inner {
    opts: SubscribeArrowStreamOptions,
    cache: Arc<EntityCache>,
    disposed: AtomicBool,
    seed_generation: AtomicU64,
    reseed: Mutex<Reseed>,
    asked: Mutex<Asked>,
    tx: mpsc::UnboundedSender<Cmd>,
    /// The runtime the stream was opened on. `reseed` is called from the UI thread, which is not a
    /// runtime thread, so it must not reach for the ambient one.
    runtime: tokio::runtime::Handle,
}

/// Dropping it disposes the stream; `reseed` re-reads the whole catalog the way a reconnect does,
/// coalescing a burst of requests into at most one more read.
pub struct ArrowStream {
    inner: Arc<Inner>,
    subscription: Mutex<Option<Subscription>>,
}

pub fn subscribe_arrow_stream(
    ws: &WsManager,
    cache: Arc<EntityCache>,
    opts: SubscribeArrowStreamOptions,
) -> ArrowStream {
    let (tx, rx) = mpsc::unbounded_channel();
    let inner = Arc::new(Inner {
        opts,
        cache,
        disposed: AtomicBool::new(false),
        seed_generation: AtomicU64::new(0),
        reseed: Mutex::default(),
        asked: Mutex::default(),
        tx,
        runtime: tokio::runtime::Handle::current(),
    });
    tokio::spawn(worker(inner.clone(), rx));
    drop(inner.run_seed());
    let for_frames = inner.clone();
    let subscription = ws.subscribe(ARROW_ENDPOINT, move |data| for_frames.on_data(data));
    ArrowStream {
        inner,
        subscription: Mutex::new(Some(subscription)),
    }
}

/// A handle that can ask for a re-read without keeping the stream alive.
#[derive(Clone)]
pub struct Reseeder(Weak<Inner>);

impl Reseeder {
    pub fn reseed(&self) {
        if let Some(inner) = self.0.upgrade() {
            inner.request_reseed();
        }
    }
}

impl ArrowStream {
    pub fn reseeder(&self) -> Reseeder {
        Reseeder(Arc::downgrade(&self.inner))
    }

    pub fn reseed(&self) {
        self.inner.request_reseed();
    }

    pub fn dispose(&self) {
        self.inner.disposed.store(true, Ordering::SeqCst);
        let _ = self.inner.tx.send(Cmd::Stop);
        self.subscription
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
    }
}

impl Drop for ArrowStream {
    fn drop(&mut self) {
        self.dispose();
    }
}

fn text(frame: &Value, key: &str) -> Option<String> {
    frame.get(key).and_then(Value::as_str).map(str::to_string)
}

impl Inner {
    fn is_disposed(&self) -> bool {
        self.disposed.load(Ordering::SeqCst)
    }

    fn on_data(self: &Arc<Self>, data: &Value) {
        if self.is_disposed() {
            return;
        }
        if is_reconnect_sentinel(data) {
            drop(self.run_seed());
            return;
        }
        if data.get("namespace").and_then(Value::as_str).is_some() {
            let _ = self.tx.send(Cmd::Frame(data.clone()));
        }
    }

    fn run_seed(&self) -> oneshot::Receiver<()> {
        let generation = self.seed_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (done, wait) = oneshot::channel();
        let _ = self.tx.send(Cmd::Seed { generation, done });
        wait
    }

    fn request_reseed(self: &Arc<Self>) {
        if self.is_disposed() {
            return;
        }
        {
            let mut state = self.reseed.lock().unwrap_or_else(PoisonError::into_inner);
            if state.pending {
                state.again = true;
                return;
            }
            state.pending = true;
        }
        let this = self.clone();
        self.runtime.spawn(async move {
            loop {
                let _ = this.run_seed().await;
                let mut state = this.reseed.lock().unwrap_or_else(PoisonError::into_inner);
                state.pending = false;
                if !state.again || this.is_disposed() {
                    state.again = false;
                    return;
                }
                state.again = false;
                state.pending = true;
            }
        });
    }

    fn changed(&self) {
        if !self.is_disposed() {
            if let Some(cb) = &self.opts.on_change {
                cb();
            }
        }
    }

    fn apply_frame(&self, frame: &Value) {
        let connection_id = &self.opts.connection_id;
        let Some(namespace) = text(frame, "namespace") else {
            return;
        };
        if frame.get("event").and_then(Value::as_str) == Some("removed") {
            self.cache.remove(connection_id, &namespace);
            return;
        }
        // quiver.core's catalog frames carry no version (the resolved ref is row state, re-read
        // from `GET /v0/arrow`), so a frame must not erase the one the last seed recorded. Each
        // still asks for that re-read: an update's advance reaches this client only as such a
        // frame, after the runtime already reported the run ended. Likewise a frame that omits
        // origin/inference (an older core) must not erase what the seed recorded; a declared arrow
        // never carries a confidence.
        let cached = self.cache.arrow(connection_id, &namespace);
        let frame_version = text(frame, "version");
        let version = frame_version
            .clone()
            .or_else(|| cached.as_ref().map(|c| c.version.clone()))
            .unwrap_or_default();
        let user_installed = frame.get("user_installed").and_then(Value::as_bool);
        if frame_version.is_none() && user_installed != Some(false) {
            let ask = {
                let mut asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
                let ask = !asked.unlisted.contains(&namespace);
                if ask {
                    asked.asked_for.insert(namespace.clone());
                }
                ask
            };
            if ask {
                if let Some(cb) = &self.opts.on_unversioned_upsert {
                    cb(&namespace);
                }
            }
        }
        let origin = match text(frame, "origin") {
            None => cached
                .as_ref()
                .and_then(|c| c.origin)
                .unwrap_or(ArrowOrigin::Declared),
            Some(wire) => parse_arrow_origin(Some(&wire)),
        };
        let confidence = if origin == ArrowOrigin::Declared {
            None
        } else {
            match frame
                .pointer("/inference/confidence")
                .and_then(Value::as_str)
            {
                None => cached.as_ref().and_then(|c| c.confidence),
                Some(wire) => parse_inference_confidence(Some(wire)),
            }
        };
        let media = |key: &str| {
            frame
                .pointer(&format!("/media/{key}"))
                .and_then(Value::as_str)
                .map(str::to_string)
        };
        self.cache.upsert(ArrowCatalogRecord {
            connection_id: connection_id.clone(),
            namespace,
            name: text(frame, "name").unwrap_or_default(),
            description: text(frame, "description").unwrap_or_default(),
            tags: frame
                .get("tags")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            icon: media("icon").or_else(|| text(frame, "icon")),
            banner: media("banner").or_else(|| text(frame, "banner")),
            version,
            last_used_at: text(frame, "last_used_at")
                .or_else(|| cached.and_then(|c| c.last_used_at)),
            origin: Some(origin),
            confidence,
        });
    }

    async fn apply_seed(&self, generation: u64) -> crate::Result<()> {
        let items = (self.opts.seed)().await?;
        if self.is_disposed() || generation != self.seed_generation.load(Ordering::SeqCst) {
            return Ok(());
        }
        let fresh: HashSet<&str> = items.iter().map(|i| i.namespace.as_str()).collect();
        {
            let mut asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
            let asked_for = std::mem::take(&mut asked.asked_for);
            for namespace in asked_for {
                if !fresh.contains(namespace.as_str()) {
                    asked.unlisted.insert(namespace);
                }
            }
            for namespace in &fresh {
                asked.unlisted.remove(*namespace);
            }
        }
        let connection_id = &self.opts.connection_id;
        for cached in self.cache.arrows_for(connection_id) {
            if !fresh.contains(cached.namespace.as_str()) {
                self.cache.remove(connection_id, &cached.namespace);
            }
        }
        for item in items.iter().cloned() {
            self.cache.upsert(item);
        }
        Ok(())
    }
}

async fn worker(inner: Arc<Inner>, mut rx: mpsc::UnboundedReceiver<Cmd>) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Cmd::Stop => return,
            Cmd::Frame(frame) => {
                if !inner.is_disposed() {
                    inner.apply_frame(&frame);
                    inner.changed();
                }
            }
            Cmd::Seed { generation, done } => {
                // A failed seed must NOT poison the queue: it is absorbed here so later frames and
                // reseeds still run. apply_seed errors before mutating the cache, so a failed
                // reseed leaves the cache intact and a later reseed retries.
                match inner.apply_seed(generation).await {
                    Ok(()) => inner.changed(),
                    Err(err) => {
                        log::error!("entity-stream: seed failed for {ARROW_ENDPOINT}: {err}");
                        if !inner.is_disposed() {
                            if let Some(cb) = &inner.opts.on_seed_error {
                                cb(&err);
                            }
                        }
                    }
                }
                let _ = done.send(());
            }
        }
    }
}
