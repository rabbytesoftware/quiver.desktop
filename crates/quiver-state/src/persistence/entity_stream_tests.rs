use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::sync::{mpsc, Semaphore};

use super::entity_cache::tests::rec;
use super::entity_stream::{
    subscribe_arrow_stream, ArrowStream, SeedFn, SubscribeArrowStreamOptions,
};
use super::{ArrowCatalogRecord, EntityCache};
use crate::core_api::{FakeCoreApi, FakeStreamHandle};
use crate::domain::{ArrowOrigin, InferenceConfidence};
use crate::transport::ws_manager::{reconnect_sentinel, WsManager};
use crate::Error;

struct Rig {
    api: FakeCoreApi,
    ws: WsManager,
    cache: Arc<EntityCache>,
    changed: mpsc::UnboundedReceiver<()>,
    changed_tx: mpsc::UnboundedSender<()>,
}

fn rig() -> Rig {
    let api = FakeCoreApi::new();
    let (changed_tx, changed) = mpsc::unbounded_channel();
    Rig {
        ws: WsManager::new(Arc::new(api.clone())),
        api,
        cache: Arc::new(EntityCache::in_memory()),
        changed,
        changed_tx,
    }
}

fn seed_of(records: Vec<ArrowCatalogRecord>) -> SeedFn {
    Arc::new(move || {
        let records = records.clone();
        Box::pin(async move { Ok(records) })
    })
}

impl Rig {
    fn opts(&self, seed: SeedFn) -> SubscribeArrowStreamOptions {
        let tx = self.changed_tx.clone();
        SubscribeArrowStreamOptions {
            connection_id: "local".into(),
            seed,
            on_change: Some(Arc::new(move || {
                let _ = tx.send(());
            })),
            on_seed_error: None,
            on_unversioned_upsert: None,
        }
    }

    fn subscribe(&self, seed: SeedFn) -> ArrowStream {
        subscribe_arrow_stream(&self.ws, self.cache.clone(), self.opts(seed))
    }

    /// Waits for the initial seed to land and the live socket to be up.
    async fn ready(&mut self, stream_seed: SeedFn) -> (ArrowStream, FakeStreamHandle) {
        let stream = self.subscribe(stream_seed);
        self.changed.recv().await.unwrap();
        let handle = self.api.wait_stream("/v0/arrow", 0).await;
        (stream, handle)
    }
}

#[tokio::test]
async fn seeds_the_cache_before_any_frame_arrives() {
    let mut rig = rig();
    let _s = rig.subscribe(seed_of(vec![rec("local", "a@1")]));
    rig.changed.recv().await.unwrap();
    assert_eq!(rig.cache.arrows_for("local"), vec![rec("local", "a@1")]);
}

#[tokio::test]
async fn reports_a_failed_seed_and_stays_silent_after_disposal() {
    let mut rig = rig();
    let (err_tx, mut err_rx) = mpsc::unbounded_channel();
    let mut opts = rig.opts(Arc::new(|| {
        Box::pin(async { Err(Error::Backend("core unreachable".into())) })
    }));
    let tx = err_tx.clone();
    opts.on_seed_error = Some(Arc::new(move |e| drop(tx.send(e.to_string()))));
    let _s = subscribe_arrow_stream(&rig.ws, rig.cache.clone(), opts);
    assert!(err_rx.recv().await.unwrap().contains("core unreachable"));
    assert!(rig.changed.try_recv().is_err());

    let mut late = rig.opts(Arc::new(|| {
        Box::pin(async { Err(Error::Backend("too late".into())) })
    }));
    late.on_seed_error = Some(Arc::new(move |e| drop(err_tx.send(e.to_string()))));
    let s = subscribe_arrow_stream(&rig.ws, rig.cache.clone(), late);
    s.dispose();
    drop(s);
    tokio::task::yield_now().await;
    assert!(err_rx.try_recv().is_err());
}

#[tokio::test]
async fn prunes_rows_the_seed_no_longer_lists_but_never_another_connections() {
    let mut rig = rig();
    rig.cache.upsert(rec("remote-1", "z@1"));
    let (s1, _h) = rig
        .ready(seed_of(vec![rec("local", "a@1"), rec("local", "b@1")]))
        .await;
    drop(s1);
    let mut rig2 = Rig {
        changed: rig.changed,
        ..rig
    };
    let _s2 = rig2.subscribe(seed_of(vec![rec("local", "a@1")]));
    rig2.changed.recv().await.unwrap();
    let names: Vec<_> = rig2
        .cache
        .arrows_for("local")
        .into_iter()
        .map(|r| r.namespace)
        .collect();
    assert_eq!(names, vec!["a@1"]);
    assert_eq!(rig2.cache.arrows_for("remote-1").len(), 1);
}

#[tokio::test]
async fn merges_a_live_upsert_frame_with_defaults() {
    let mut rig = rig();
    let (_s, h) = rig.ready(seed_of(vec![])).await;
    h.send_json(json!({
        "event": "upserted", "namespace": "new@1", "name": "New Arrow", "description": "a description",
        "tags": ["a", "b"], "icon": "icon.png", "banner": "banner.png", "version": "2"
    }));
    rig.changed.recv().await.unwrap();
    assert_eq!(
        rig.cache.arrows_for("local"),
        vec![ArrowCatalogRecord {
            connection_id: "local".into(),
            namespace: "new@1".into(),
            name: "New Arrow".into(),
            description: "a description".into(),
            tags: vec!["a".into(), "b".into()],
            icon: Some("icon.png".into()),
            banner: Some("banner.png".into()),
            version: "2".into(),
            last_used_at: None,
            origin: Some(ArrowOrigin::Declared),
            confidence: None,
        }]
    );
}

#[tokio::test]
async fn carries_origin_confidence_and_nested_media_preferring_nested() {
    let mut rig = rig();
    let (_s, h) = rig.ready(seed_of(vec![])).await;
    h.send_json(json!({
        "event": "upserted", "namespace": "made@1", "name": "Made", "origin": "inferred",
        "inference": {"generator": "fletcher/1", "confidence": "medium"},
        "icon": "flat.png", "media": {"icon": "nested.png", "banner": "nested-banner.png"}
    }));
    rig.changed.recv().await.unwrap();
    let stored = &rig.cache.arrows_for("local")[0];
    assert_eq!(stored.origin, Some(ArrowOrigin::Inferred));
    assert_eq!(stored.confidence, Some(InferenceConfidence::Medium));
    assert_eq!(stored.icon.as_deref(), Some("nested.png"));
    assert_eq!(stored.banner.as_deref(), Some("nested-banner.png"));
}

#[tokio::test]
async fn keeps_cached_version_origin_confidence_and_last_used_when_a_frame_omits_them() {
    let mut rig = rig();
    let seeded = ArrowCatalogRecord {
        version: "v1.2.0".into(),
        origin: Some(ArrowOrigin::Inferred),
        confidence: Some(InferenceConfidence::High),
        last_used_at: Some("2026-01-01T00:00:00Z".into()),
        ..rec("local", "a@1")
    };
    let (_s, h) = rig.ready(seed_of(vec![seeded])).await;
    h.send_json(json!({"event": "upserted", "namespace": "a@1", "name": "A"}));
    rig.changed.recv().await.unwrap();
    let stored = &rig.cache.arrows_for("local")[0];
    assert_eq!(stored.version, "v1.2.0");
    assert_eq!(stored.origin, Some(ArrowOrigin::Inferred));
    assert_eq!(stored.confidence, Some(InferenceConfidence::High));
    assert_eq!(stored.last_used_at.as_deref(), Some("2026-01-01T00:00:00Z"));

    h.send_json(json!({"event": "upserted", "namespace": "a@1", "name": "A", "last_used_at": "2026-02-02T00:00:00Z", "origin": "declared"}));
    rig.changed.recv().await.unwrap();
    let stored = &rig.cache.arrows_for("local")[0];
    assert_eq!(stored.last_used_at.as_deref(), Some("2026-02-02T00:00:00Z"));
    assert_eq!(
        (stored.origin, stored.confidence),
        (Some(ArrowOrigin::Declared), None),
        "a stale confidence is dropped"
    );
}

#[tokio::test]
async fn asks_for_a_reread_on_unversioned_upserts_except_non_library_rows() {
    let mut rig = rig();
    let (asked_tx, mut asked) = mpsc::unbounded_channel();
    let mut opts = rig.opts(seed_of(vec![]));
    opts.on_unversioned_upsert = Some(Arc::new(move |ns| drop(asked_tx.send(ns.to_string()))));
    let _s = subscribe_arrow_stream(&rig.ws, rig.cache.clone(), opts);
    rig.changed.recv().await.unwrap();
    let h = rig.api.wait_stream("/v0/arrow", 0).await;
    h.send_json(json!({"event":"upserted","namespace":"lib@1","name":"L"}));
    h.send_json(json!({"event":"upserted","namespace":"dep@1","name":"D","user_installed":false}));
    h.send_json(json!({"event":"upserted","namespace":"versioned@1","name":"V","version":"3"}));
    for _ in 0..3 {
        rig.changed.recv().await.unwrap();
    }
    assert_eq!(asked.recv().await.unwrap(), "lib@1");
    assert!(
        asked.try_recv().is_err(),
        "neither a non-library row nor a versioned frame asks"
    );
}

#[tokio::test]
async fn does_not_ask_again_for_a_namespace_a_reread_showed_is_not_a_library_row() {
    let mut rig = rig();
    let (asked_tx, mut asked) = mpsc::unbounded_channel();
    let mut opts = rig.opts(seed_of(vec![]));
    opts.on_unversioned_upsert = Some(Arc::new(move |ns| drop(asked_tx.send(ns.to_string()))));
    let s = subscribe_arrow_stream(&rig.ws, rig.cache.clone(), opts);
    rig.changed.recv().await.unwrap();
    let h = rig.api.wait_stream("/v0/arrow", 0).await;
    h.send_json(json!({"event":"upserted","namespace":"dep@1","name":"D"}));
    rig.changed.recv().await.unwrap();
    assert_eq!(asked.recv().await.unwrap(), "dep@1");
    s.reseed();
    rig.changed.recv().await.unwrap();
    h.send_json(json!({"event":"upserted","namespace":"dep@1","name":"D"}));
    rig.changed.recv().await.unwrap();
    assert!(asked.try_recv().is_err());
}

#[tokio::test]
async fn commits_an_upsert_then_a_delete_in_arrival_order() {
    let mut rig = rig();
    let (_s, h) = rig.ready(seed_of(vec![])).await;
    h.send_json(json!({"event":"upserted","namespace":"a@1","name":"A","version":"1"}));
    h.send_json(json!({"event":"removed","namespace":"a@1"}));
    rig.changed.recv().await.unwrap();
    rig.changed.recv().await.unwrap();
    assert!(rig.cache.arrows_for("local").is_empty());
}

#[tokio::test]
async fn reseeds_on_the_reconnect_sentinel_and_survives_a_failed_seed() {
    let mut rig = rig();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let seed: SeedFn = Arc::new(move || {
        let n = counter.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if n == 1 {
                Err(Error::Backend("down".into()))
            } else {
                Ok(vec![rec("local", &format!("seed{n}@1"))])
            }
        })
    });
    let (_s, h) = rig.ready(seed).await;
    h.send_json(reconnect_sentinel()); // second seed fails, no on_change
    h.send_json(json!({"event":"upserted","namespace":"live@1","name":"L","version":"1"}));
    rig.changed.recv().await.unwrap(); // the live frame still applies after the failed seed
    h.send_json(reconnect_sentinel());
    rig.changed.recv().await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let names: Vec<_> = rig
        .cache
        .arrows_for("local")
        .into_iter()
        .map(|r| r.namespace)
        .collect();
    assert_eq!(
        names,
        vec!["seed2@1"],
        "the last seed prunes what it does not list"
    );
}

#[tokio::test]
async fn coalesces_a_burst_of_reread_requests_into_one_more_seed() {
    let mut rig = rig();
    let gate = Arc::new(Semaphore::new(1));
    let calls = Arc::new(AtomicUsize::new(0));
    let (g, c) = (gate.clone(), calls.clone());
    let seed: SeedFn = Arc::new(move || {
        let (g, c) = (g.clone(), c.clone());
        Box::pin(async move {
            g.acquire().await.unwrap().forget();
            c.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        })
    });
    let s = rig.subscribe(seed);
    rig.changed.recv().await.unwrap(); // initial seed through
    s.reseed(); // in flight, blocked on the gate
    for _ in 0..5 {
        s.reseed();
    }
    gate.add_permits(10);
    rig.changed.recv().await.unwrap();
    rig.changed.recv().await.unwrap();
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "initial + the in-flight one + exactly one queued"
    );
}

#[tokio::test]
async fn a_seed_superseded_by_a_newer_one_writes_nothing() {
    let mut rig = rig();
    let first_gate = Arc::new(Semaphore::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let (g, c) = (first_gate.clone(), calls.clone());
    let seed: SeedFn = Arc::new(move || {
        let n = c.fetch_add(1, Ordering::SeqCst);
        let g = g.clone();
        Box::pin(async move {
            if n == 0 {
                g.acquire().await.unwrap().forget();
                Ok(vec![rec("local", "stale@1")])
            } else {
                Ok(vec![rec("local", "fresh@1")])
            }
        })
    });
    let s = rig.subscribe(seed);
    let _h = rig.api.wait_stream("/v0/arrow", 0).await;
    // Enqueue a newer seed while the first is parked: it bumps the generation.
    s.reseed();
    first_gate.add_permits(1);
    rig.changed.recv().await.unwrap();
    rig.changed.recv().await.unwrap();
    let names: Vec<_> = rig
        .cache
        .arrows_for("local")
        .into_iter()
        .map(|r| r.namespace)
        .collect();
    assert_eq!(names, vec!["fresh@1"]);
}

#[tokio::test]
async fn ignores_invalid_frames_and_stops_after_dispose() {
    let mut rig = rig();
    let (s, h) = rig.ready(seed_of(vec![])).await;
    h.send_json(Value::Null);
    h.send_json(json!({"event": "upserted"}));
    h.send_json(json!({"event":"upserted","namespace":"ok@1","name":"Ok","version":"1"}));
    rig.changed.recv().await.unwrap();
    assert_eq!(rig.cache.arrows_for("local").len(), 1);
    s.dispose();
    h.send_json(json!({"event":"upserted","namespace":"late@1","name":"L","version":"1"}));
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert_eq!(rig.cache.arrows_for("local").len(), 1);
    assert!(rig.changed.try_recv().is_err());
}

/// The UI asks for a re-read from its own thread, which is not a runtime thread: the stream must
/// spawn on the runtime it was opened on, not on the ambient one (which does not exist there).
#[tokio::test]
async fn a_reseed_requested_from_a_thread_outside_the_runtime_still_runs() {
    let mut rig = rig();
    let reads = Arc::new(AtomicUsize::new(0));
    let counted = reads.clone();
    let seed: SeedFn = Arc::new(move || {
        counted.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(vec![]) })
    });
    let (stream, _live) = rig.ready(seed).await;
    let reseeder = stream.reseeder();
    let stream = std::thread::spawn(move || {
        stream.reseed();
        reseeder.reseed();
        stream
    })
    .join()
    .unwrap();
    rig.changed.recv().await.unwrap();
    assert!(reads.load(Ordering::SeqCst) >= 2);
    drop(stream);
}
