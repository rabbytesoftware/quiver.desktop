use std::sync::atomic::{AtomicUsize, Ordering};

use std::sync::Arc;

use super::arrows::*;
use crate::domain::{ActiveRun, ArrowOrigin, InferenceConfidence, LastReturn, Outcome};
use crate::domain::{ArrowState, RuntimeUpdate};
use crate::persistence::entity_cache::tests::rec;
use crate::persistence::ArrowCatalogRecord;

fn record(ns: &str) -> ArrowCatalogRecord {
    rec("local", ns)
}

fn update(
    ns: &str,
    state: ArrowState,
) -> RuntimeUpdate {
    RuntimeUpdate {
        namespace: ns.into(),
        state,
        active_run: None,
        last_return: None,
    }
}

fn state_of(
    store: &ArrowStore,
    ns: &str,
) -> Option<ArrowState> {
    store.entry(ns).map(|e| e.state)
}

fn run(method: &str) -> ActiveRun {
    ActiveRun {
        method: method.into(),
        pid: None,
        variables: Default::default(),
        steps: vec![],
        surface: None,
    }
}

fn success(method: &str) -> LastReturn {
    LastReturn {
        method: method.into(),
        outcome: Outcome::Success,
    }
}

#[test]
fn refresh_goes_through_the_installed_hook_and_is_a_noop_before_one() {
    let store = ArrowStore::default();
    store.refresh_catalog();
    let n = Arc::new(AtomicUsize::new(0));
    let counter = n.clone();
    store.set_catalog_refresh(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    store.refresh_catalog();
    assert_eq!(n.load(Ordering::SeqCst), 1);
}

#[test]
fn projects_catalog_records_with_neutral_runtime_state() {
    let store = ArrowStore::default();
    store.set_catalog(&[record("a@1")]);
    let e = store.entry("a@1").unwrap();
    assert_eq!(e.state, ArrowState::Absent);
    assert!(e.active_run.is_none());
}

#[test]
fn preserves_the_overlay_for_arrows_still_present_across_a_reseed() {
    let store = ArrowStore::default();
    let rec = record("a@1");
    store.set_catalog(std::slice::from_ref(&rec));
    store.apply_runtime_update(RuntimeUpdate {
        active_run: Some(run("execute")),
        last_return: Some(success("install")),
        ..update("a@1", ArrowState::Running)
    });
    store.set_catalog(&[rec]);
    let e = store.entry("a@1").unwrap();
    assert_eq!(e.state, ArrowState::Running);
    assert_eq!(e.active_run.unwrap().method, "execute");
    assert_eq!(e.last_return.unwrap().outcome, Outcome::Success);
}

#[test]
fn a_namespace_does_not_inherit_its_overlay_after_leaving_and_rejoining() {
    let store = ArrowStore::default();
    let rec = record("a@1");
    store.set_catalog(std::slice::from_ref(&rec));
    store.apply_runtime_update(update("a@1", ArrowState::Running));
    store.set_catalog(&[record("b@1")]);
    store.set_catalog(&[rec, record("b@1")]);
    assert_eq!(state_of(&store, "a@1"), Some(ArrowState::Absent));
}

#[test]
fn carries_origin_and_confidence_and_leaves_old_records_without_one() {
    let store = ArrowStore::default();
    store.set_catalog(&[ArrowCatalogRecord {
        origin: Some(ArrowOrigin::Inferred),
        confidence: Some(InferenceConfidence::High),
        ..record("a@1")
    }]);
    let e = store.entry("a@1").unwrap();
    assert_eq!(
        (e.origin, e.confidence),
        (Some(ArrowOrigin::Inferred), Some(InferenceConfidence::High))
    );
    store.set_catalog(&[record("b@1")]);
    assert_eq!(store.entry("b@1").unwrap().origin, None);
}

#[test]
fn overlays_runtime_state_without_touching_catalog_fields() {
    let store = ArrowStore::default();
    store.set_catalog(&[ArrowCatalogRecord {
        name: "nice name".into(),
        icon: Some("i".into()),
        ..record("a@1")
    }]);
    store.apply_runtime_update(update("a@1", ArrowState::Running));
    let e = store.entry("a@1").unwrap();
    assert_eq!(
        (e.state, e.name.as_str(), e.icon.as_deref()),
        (ArrowState::Running, "nice name", Some("i"))
    );
}

#[test]
fn preserves_the_last_outcome_when_a_later_frame_omits_it_including_across_set_catalog() {
    let store = ArrowStore::default();
    let rec = record("a@1");
    store.set_catalog(std::slice::from_ref(&rec));
    store.apply_runtime_update(RuntimeUpdate {
        last_return: Some(success("install")),
        ..update("a@1", ArrowState::Ready)
    });
    store.apply_runtime_update(update("a@1", ArrowState::Running));
    assert_eq!(
        store.entry("a@1").unwrap().last_return.unwrap().outcome,
        Outcome::Success
    );
    store.set_catalog(&[rec]);
    assert_eq!(
        store.entry("a@1").unwrap().last_return.unwrap().outcome,
        Outcome::Success
    );
}

#[test]
fn catalog_status_walks_loading_ready_error_and_back() {
    let store = ArrowStore::default();
    assert_eq!(
        store.catalog(),
        CatalogStatus::Loading,
        "a cold rail never claims the library is empty"
    );
    store.set_catalog(&[]);
    assert_eq!(store.catalog(), CatalogStatus::Ready);
    store.reset();
    assert_eq!(store.catalog(), CatalogStatus::Loading);
    store.set_catalog_error();
    assert_eq!(store.catalog(), CatalogStatus::Error);
    store.set_catalog(&[]);
    assert_eq!(
        store.catalog(),
        CatalogStatus::Ready,
        "a later successful seed clears the error"
    );
}

#[test]
fn buffers_an_update_that_arrives_before_its_catalog_entry() {
    let store = ArrowStore::default();
    store.apply_runtime_update(update("a@1", ArrowState::Ready));
    assert!(store.entries().is_empty());
    store.set_catalog(&[record("a@1")]);
    assert_eq!(state_of(&store, "a@1"), Some(ArrowState::Ready));
}

#[test]
fn buffers_the_full_overlay_not_just_the_state() {
    let store = ArrowStore::default();
    store.apply_runtime_update(RuntimeUpdate {
        active_run: Some(run("execute")),
        last_return: Some(success("install")),
        ..update("a@1", ArrowState::Running)
    });
    store.set_catalog(&[record("a@1")]);
    let e = store.entry("a@1").unwrap();
    assert_eq!(e.active_run.unwrap().method, "execute");
    assert_eq!(e.last_return.unwrap().method, "install");
}

#[test]
fn a_drained_buffered_update_counts_as_live_so_a_same_batch_seed_cannot_clobber_it() {
    let store = ArrowStore::default();
    store.apply_runtime_update(update("a@1", ArrowState::Running));
    store.set_catalog(&[record("a@1")]);
    store.seed_initial_state(update("a@1", ArrowState::Ready));
    assert_eq!(state_of(&store, "a@1"), Some(ArrowState::Running));
}

#[test]
fn a_buffered_update_does_not_leak_past_reset() {
    let store = ArrowStore::default();
    store.apply_runtime_update(update("a@1", ArrowState::Running));
    store.reset();
    store.set_catalog(&[record("a@1")]);
    assert_eq!(state_of(&store, "a@1"), Some(ArrowState::Absent));
}

#[test]
fn caps_the_pending_buffer_dropping_the_oldest() {
    let store = ArrowStore::default();
    for i in 0..300 {
        store.apply_runtime_update(update(&format!("dep@{i}"), ArrowState::Ready));
    }
    store.set_catalog(&[record("dep@299")]);
    assert_eq!(
        state_of(&store, "dep@299"),
        Some(ArrowState::Ready),
        "the newest survives"
    );
    store.set_catalog(&[record("dep@299"), record("dep@0")]);
    assert_eq!(
        state_of(&store, "dep@0"),
        Some(ArrowState::Absent),
        "the oldest was evicted"
    );
}

#[test]
fn ignores_a_runtime_update_for_an_unknown_arrow() {
    let store = ArrowStore::default();
    store.apply_runtime_update(update("ghost@1", ArrowState::Running));
    assert!(store.entries().is_empty());
}

#[test]
fn seed_paints_when_no_overlay_exists_and_yields_to_a_live_frame() {
    let store = ArrowStore::default();
    store.set_catalog(&[record("a@1")]);
    store.seed_initial_state(update("a@1", ArrowState::Running));
    assert_eq!(state_of(&store, "a@1"), Some(ArrowState::Running));
    store.apply_runtime_update(update("a@1", ArrowState::Stopping));
    store.seed_initial_state(update("a@1", ArrowState::Ready));
    assert_eq!(state_of(&store, "a@1"), Some(ArrowState::Stopping));
}

#[test]
fn ignores_a_seed_for_an_unknown_arrow_and_lets_a_newer_seed_overwrite_an_older_one() {
    let store = ArrowStore::default();
    store.seed_initial_state(update("ghost@1", ArrowState::Running));
    assert!(store.entries().is_empty());
    store.set_catalog(&[record("a@1")]);
    store.seed_initial_state(update("a@1", ArrowState::Ready));
    store.seed_initial_state(update("a@1", ArrowState::Running));
    assert_eq!(state_of(&store, "a@1"), Some(ArrowState::Running));
}

#[test]
fn a_seeded_outcome_survives_a_later_seed_and_a_reseed() {
    let store = ArrowStore::default();
    let rec = record("a@1");
    store.set_catalog(std::slice::from_ref(&rec));
    store.seed_initial_state(RuntimeUpdate {
        last_return: Some(success("install")),
        ..update("a@1", ArrowState::Ready)
    });
    store.seed_initial_state(update("a@1", ArrowState::Running));
    assert_eq!(
        store.entry("a@1").unwrap().last_return.unwrap().outcome,
        Outcome::Success
    );
    store.set_catalog(&[rec]);
    assert_eq!(
        store.entry("a@1").unwrap().last_return.unwrap().outcome,
        Outcome::Success
    );
}

#[test]
fn reset_clears_the_projection_and_keeps_catalog_order() {
    let store = ArrowStore::default();
    store.set_catalog(&[record("b@1"), record("a@1")]);
    assert_eq!(
        store
            .entries()
            .iter()
            .map(|e| e.namespace.as_str())
            .collect::<Vec<_>>(),
        vec!["b@1", "a@1"]
    );
    store.reset();
    assert!(store.entries().is_empty());
}
