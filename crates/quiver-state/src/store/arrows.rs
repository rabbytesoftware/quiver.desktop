use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};

use indexmap::IndexMap;

use crate::domain::{ArrowEntry, ArrowState, RuntimeUpdate};
use crate::observable::Store;
use crate::persistence::ArrowCatalogRecord;

const NEUTRAL_STATE: ArrowState = ArrowState::Absent;

/// The runtime endpoint broadcasts every arrow the daemon manages, not just this catalog's own
/// (user-installed) rows -- a dependency pulled in by an install transitions through its own states
/// on the same stream, and since a pure dependency never appears in the catalog seed (always
/// filtered to user-installed arrows), its buffered update would otherwise sit in
/// `pending_runtime` for the rest of the session. This cap keeps that bounded: once exceeded, the
/// oldest buffered entry is dropped to make room for the newest.
const MAX_PENDING_RUNTIME: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogStatus {
    Loading,
    Ready,
    Error,
}

/// What the sidebar and every list read: catalog rows keyed by `namespace@selector`, in catalog
/// order, each with the live runtime overlay applied.
pub struct ArrowsState {
    pub arrows: IndexMap<String, ArrowEntry>,
    pub catalog: CatalogStatus,
    runtime: IndexMap<String, RuntimeUpdate>,
    live_namespaces: HashSet<String>,
    /// A runtime frame for a namespace whose catalog entry hasn't landed yet (announce's own POST
    /// races its two independent WS streams -- the catalog upsert and the runtime transition it
    /// triggers arrive in either order). Kept separate from `runtime` so the pruning in
    /// `set_catalog` -- which can't tell "removed" apart from "not seeded yet" -- never discards it
    /// before `set_catalog` gets a chance to consume it. Drained (never pruned) on first sight of
    /// the namespace in `set_catalog`, or wholesale on reset.
    pending_runtime: IndexMap<String, RuntimeUpdate>,
}

impl Default for ArrowsState {
    fn default() -> Self {
        Self {
            arrows: IndexMap::new(),
            catalog: CatalogStatus::Loading,
            runtime: IndexMap::new(),
            live_namespaces: HashSet::new(),
            pending_runtime: IndexMap::new(),
        }
    }
}

type Refresh = Arc<dyn Fn() + Send + Sync>;

fn to_entry(record: &ArrowCatalogRecord, overlay: Option<&RuntimeUpdate>) -> ArrowEntry {
    ArrowEntry {
        namespace: record.namespace.clone(),
        name: record.name.clone(),
        description: record.description.clone(),
        tags: record.tags.clone(),
        icon: record.icon.clone(),
        banner: record.banner.clone(),
        version: record.version.clone(),
        state: overlay.map_or(NEUTRAL_STATE, |o| o.state),
        active_run: overlay.and_then(|o| o.active_run.clone()),
        last_return: overlay.and_then(|o| o.last_return.clone()),
        last_used_at: record.last_used_at.clone(),
        origin: record.origin,
        confidence: record.confidence,
    }
}

/// A frame that omits the outcome keeps the last known one.
fn resolve_overlay(existing: &ArrowEntry, update: &RuntimeUpdate) -> RuntimeUpdate {
    RuntimeUpdate {
        namespace: update.namespace.clone(),
        state: update.state,
        active_run: update.active_run.clone(),
        last_return: update.last_return.clone().or_else(|| existing.last_return.clone()),
    }
}

fn patch_overlay(existing: &mut ArrowEntry, overlay: &RuntimeUpdate) {
    existing.state = overlay.state;
    existing.active_run = overlay.active_run.clone();
    existing.last_return = overlay.last_return.clone();
}

#[derive(Clone)]
pub struct ArrowStore {
    store: Store<ArrowsState>,
    /// Re-reads the catalog from `GET /v0/arrow` -- the only source of each row's resolved ref,
    /// which no live frame carries. A no-op while no catalog stream is up; the listeners install
    /// the real one.
    refresh: Arc<Mutex<Refresh>>,
}

impl Default for ArrowStore {
    fn default() -> Self {
        Self { store: Store::default(), refresh: Arc::new(Mutex::new(Arc::new(|| {}))) }
    }
}

impl ArrowStore {
    pub fn store(&self) -> &Store<ArrowsState> {
        &self.store
    }

    pub fn entry(&self, namespace: &str) -> Option<ArrowEntry> {
        self.store.read(|s| s.arrows.get(namespace).cloned())
    }

    pub fn entries(&self) -> Vec<ArrowEntry> {
        self.store.read(|s| s.arrows.values().cloned().collect())
    }

    pub fn catalog(&self) -> CatalogStatus {
        self.store.read(|s| s.catalog)
    }

    pub fn set_catalog_error(&self) {
        self.store.update(|s| s.catalog = CatalogStatus::Error);
    }

    pub fn set_catalog(&self, records: &[ArrowCatalogRecord]) {
        self.store.update(|s| {
            let mut next = IndexMap::with_capacity(records.len());
            let mut still_present = HashSet::with_capacity(records.len());
            for record in records {
                still_present.insert(record.namespace.as_str());
                let mut overlay = s.runtime.get(&record.namespace).cloned();
                if overlay.is_none() && !s.arrows.contains_key(&record.namespace) {
                    if let Some(pending) = s.pending_runtime.shift_remove(&record.namespace) {
                        s.runtime.insert(record.namespace.clone(), pending.clone());
                        s.live_namespaces.insert(record.namespace.clone());
                        overlay = Some(pending);
                    }
                }
                next.insert(record.namespace.clone(), to_entry(record, overlay.as_ref()));
            }
            let gone: Vec<String> = s.runtime.keys().filter(|ns| !still_present.contains(ns.as_str())).cloned().collect();
            for namespace in gone {
                s.runtime.shift_remove(&namespace);
                s.live_namespaces.remove(&namespace);
            }
            s.arrows = next;
            s.catalog = CatalogStatus::Ready;
        });
    }

    pub fn apply_runtime_update(&self, update: RuntimeUpdate) {
        self.store.update(|s| {
            let Some(existing) = s.arrows.get(&update.namespace) else {
                s.pending_runtime.shift_remove(&update.namespace);
                if s.pending_runtime.len() >= MAX_PENDING_RUNTIME {
                    s.pending_runtime.shift_remove_index(0);
                }
                s.pending_runtime.insert(update.namespace.clone(), update);
                return;
            };
            let resolved = resolve_overlay(existing, &update);
            s.runtime.insert(update.namespace.clone(), resolved.clone());
            s.live_namespaces.insert(update.namespace.clone());
            if let Some(entry) = s.arrows.get_mut(&update.namespace) {
                patch_overlay(entry, &resolved);
            }
        });
    }

    /// Paints the initial state from the catalog list (neither stream pushes on connect), unless a
    /// live runtime frame already established the overlay.
    pub fn seed_initial_state(&self, update: RuntimeUpdate) {
        self.store.update(|s| {
            let Some(existing) = s.arrows.get(&update.namespace) else { return };
            if s.live_namespaces.contains(&update.namespace) {
                return;
            }
            let resolved = resolve_overlay(existing, &update);
            s.runtime.insert(update.namespace.clone(), resolved.clone());
            if let Some(entry) = s.arrows.get_mut(&update.namespace) {
                patch_overlay(entry, &resolved);
            }
        });
    }

    pub fn reset(&self) {
        self.store.set(ArrowsState::default());
    }

    pub fn refresh_catalog(&self) {
        let refresh = self.refresh.lock().unwrap_or_else(PoisonError::into_inner).clone();
        refresh();
    }

    pub fn set_catalog_refresh(&self, refresh: impl Fn() + Send + Sync + 'static) {
        *self.refresh.lock().unwrap_or_else(PoisonError::into_inner) = Arc::new(refresh);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::domain::{ActiveRun, ArrowOrigin, InferenceConfidence, LastReturn, Outcome};
    use crate::persistence::entity_cache::tests::rec;

    fn record(ns: &str) -> ArrowCatalogRecord {
        rec("local", ns)
    }

    fn update(ns: &str, state: ArrowState) -> RuntimeUpdate {
        RuntimeUpdate { namespace: ns.into(), state, active_run: None, last_return: None }
    }

    fn state_of(store: &ArrowStore, ns: &str) -> Option<ArrowState> {
        store.entry(ns).map(|e| e.state)
    }

    fn run(method: &str) -> ActiveRun {
        ActiveRun { method: method.into(), pid: None, variables: Default::default(), steps: vec![], surface: None }
    }

    fn success(method: &str) -> LastReturn {
        LastReturn { method: method.into(), outcome: Outcome::Success }
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
        store.set_catalog(&[rec.clone()]);
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
        store.set_catalog(&[rec.clone()]);
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
        assert_eq!((e.origin, e.confidence), (Some(ArrowOrigin::Inferred), Some(InferenceConfidence::High)));
        store.set_catalog(&[record("b@1")]);
        assert_eq!(store.entry("b@1").unwrap().origin, None);
    }

    #[test]
    fn overlays_runtime_state_without_touching_catalog_fields() {
        let store = ArrowStore::default();
        store.set_catalog(&[ArrowCatalogRecord { name: "nice name".into(), icon: Some("i".into()), ..record("a@1") }]);
        store.apply_runtime_update(update("a@1", ArrowState::Running));
        let e = store.entry("a@1").unwrap();
        assert_eq!((e.state, e.name.as_str(), e.icon.as_deref()), (ArrowState::Running, "nice name", Some("i")));
    }

    #[test]
    fn preserves_the_last_outcome_when_a_later_frame_omits_it_including_across_set_catalog() {
        let store = ArrowStore::default();
        let rec = record("a@1");
        store.set_catalog(&[rec.clone()]);
        store.apply_runtime_update(RuntimeUpdate { last_return: Some(success("install")), ..update("a@1", ArrowState::Ready) });
        store.apply_runtime_update(update("a@1", ArrowState::Running));
        assert_eq!(store.entry("a@1").unwrap().last_return.unwrap().outcome, Outcome::Success);
        store.set_catalog(&[rec]);
        assert_eq!(store.entry("a@1").unwrap().last_return.unwrap().outcome, Outcome::Success);
    }

    #[test]
    fn catalog_status_walks_loading_ready_error_and_back() {
        let store = ArrowStore::default();
        assert_eq!(store.catalog(), CatalogStatus::Loading, "a cold rail never claims the library is empty");
        store.set_catalog(&[]);
        assert_eq!(store.catalog(), CatalogStatus::Ready);
        store.reset();
        assert_eq!(store.catalog(), CatalogStatus::Loading);
        store.set_catalog_error();
        assert_eq!(store.catalog(), CatalogStatus::Error);
        store.set_catalog(&[]);
        assert_eq!(store.catalog(), CatalogStatus::Ready, "a later successful seed clears the error");
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
        assert_eq!(state_of(&store, "dep@299"), Some(ArrowState::Ready), "the newest survives");
        store.set_catalog(&[record("dep@299"), record("dep@0")]);
        assert_eq!(state_of(&store, "dep@0"), Some(ArrowState::Absent), "the oldest was evicted");
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
        store.set_catalog(&[rec.clone()]);
        store.seed_initial_state(RuntimeUpdate { last_return: Some(success("install")), ..update("a@1", ArrowState::Ready) });
        store.seed_initial_state(update("a@1", ArrowState::Running));
        assert_eq!(store.entry("a@1").unwrap().last_return.unwrap().outcome, Outcome::Success);
        store.set_catalog(&[rec]);
        assert_eq!(store.entry("a@1").unwrap().last_return.unwrap().outcome, Outcome::Success);
    }

    #[test]
    fn reset_clears_the_projection_and_keeps_catalog_order() {
        let store = ArrowStore::default();
        store.set_catalog(&[record("b@1"), record("a@1")]);
        assert_eq!(store.entries().iter().map(|e| e.namespace.as_str()).collect::<Vec<_>>(), vec!["b@1", "a@1"]);
        store.reset();
        assert!(store.entries().is_empty());
    }
}
