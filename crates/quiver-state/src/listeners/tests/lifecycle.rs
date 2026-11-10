//! Listener lifecycle: restarts, refreshes, cache wipes and teardown.

use super::*;

#[tokio::test]
async fn starting_clears_the_projection_and_closes_both_streams() {
    let rig = rig();
    let _l = rig.ready().await;
    let arrow = rig.api.wait_stream("/v0/arrow", 0).await;
    let runtime = rig.api.wait_stream("/v0/runtime", 0).await;
    rig.backend.emit_status(ConnectionStatus::Starting);
    rig.wait_status(ConnectionStatus::Starting).await;
    assert!(rig.arrows.entries().is_empty());
    wait_dropped(&arrow).await;
    wait_dropped(&runtime).await;
}

#[tokio::test]
async fn a_ready_superseded_by_starting_while_connections_were_in_flight_is_dropped() {
    let rig = rig();
    let _l = rig.setup(None, true).await;
    rig.backend.emit_status(ConnectionStatus::Starting);
    rig.wait_status(ConnectionStatus::Starting).await;
    let parked = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let (p, r) = (parked.clone(), release.clone());
    rig.backend.set_get_connections(move || {
        let (p, r) = (p.clone(), r.clone());
        async move {
            p.notify_one();
            r.notified().await;
            Ok(ConnectionsSnapshot {
                connections: vec![local_connection()],
                active_id: "local".into(),
            })
        }
    });
    rig.backend.emit_status(ConnectionStatus::Ready);
    parked.notified().await;
    rig.backend.emit_status(ConnectionStatus::Starting);
    rig.wait_status(ConnectionStatus::Starting).await;
    release.notify_waiters();
    release.notify_one();
    for _ in 0..30 {
        tokio::task::yield_now().await;
    }
    assert_eq!(rig.status.status(), ConnectionStatus::Starting);
    assert_eq!(
        rig.api.open_count("/v0/runtime"),
        1,
        "only the boot adoption's stream was ever opened"
    );
}

#[tokio::test]
async fn a_disconnected_status_only_updates_the_status() {
    let rig = rig();
    let _l = rig.ready().await;
    rig.backend.emit_status(ConnectionStatus::Disconnected);
    rig.wait_status(ConnectionStatus::Disconnected).await;
    assert_eq!(rig.arrows.entries().len(), 1);
}

#[tokio::test]
async fn re_reads_open_details_and_collections_when_core_refreshes_a_manifest() {
    let rig = rig();
    let _l = rig.ready().await;
    let refreshed = rig
        .api
        .wait_stream("/v0/arrow?user_installed=false", 0)
        .await;
    refreshed.send_json(json!({"event": "upserted", "namespace": "other@1"}));
    let mut rx = rig.queries.subscribe();
    while rig.queries.epoch(&collection_key("c")) == 0 {
        rx.changed().await.unwrap();
    }
    assert_eq!(rig.queries.epoch(&arrow_detail_key("x@1")), 1);
}

#[tokio::test]
async fn does_not_subscribe_to_manifest_refreshes_without_a_query_client() {
    let rig = rig();
    let _l = rig.setup(None, false).await;
    rig.api.wait_stream("/v0/runtime", 0).await;
    assert_eq!(rig.api.open_count("/v0/arrow?user_installed=false"), 0);
}

#[tokio::test]
async fn re_reads_open_details_when_core_announces_a_row_it_changed() {
    let rig = rig();
    let _l = rig.ready().await;
    let arrow = rig.api.wait_stream("/v0/arrow", 0).await;
    arrow.send_json(json!({"event": "upserted", "namespace": "a@v1", "name": "A"}));
    let mut rx = rig.queries.subscribe();
    while rig.queries.epoch(&arrow_detail_key("a@v1")) == 0 {
        rx.changed().await.unwrap();
    }
    assert!(rig.api.calls_to(Method::Get, LIST) >= 1);
}

#[tokio::test]
async fn a_failed_seed_marks_the_catalog_errored() {
    let rig = rig();
    rig.api.route_error(Method::Get, LIST, 500, "boom");
    let _l = rig.setup(None, true).await;
    rig.wait_arrows(|a| a.catalog() == crate::store::CatalogStatus::Error)
        .await;
}

#[tokio::test]
async fn wipes_the_cache_when_the_stored_version_differs_before_anything_starts() {
    let rig = rig();
    rig.kv.set(CACHE_VERSION_KEY, "ancient").unwrap();
    rig.cache
        .upsert(crate::persistence::entity_cache::tests::rec(
            "local", "stale@1",
        ));
    rig.api.set_reachable(false);
    let _l = rig.setup(None, true).await;
    assert!(rig.cache.arrows_for("local").is_empty());
    assert_eq!(
        rig.kv.get(CACHE_VERSION_KEY).unwrap().as_deref(),
        Some(QUIVER_CACHE_VERSION)
    );
}

#[tokio::test]
async fn dropping_the_listeners_closes_the_streams() {
    let rig = rig();
    let listeners = rig.ready().await;
    let runtime = rig.api.wait_stream("/v0/runtime", 0).await;
    drop(listeners);
    wait_dropped(&runtime).await;
}
