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
        last_return: update
            .last_return
            .clone()
            .or_else(|| existing.last_return.clone()),
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
        Self {
            store: Store::default(),
            refresh: Arc::new(Mutex::new(Arc::new(|| {}))),
        }
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
            let gone: Vec<String> = s
                .runtime
                .keys()
                .filter(|ns| !still_present.contains(ns.as_str()))
                .cloned()
                .collect();
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
            let Some(existing) = s.arrows.get(&update.namespace) else {
                return;
            };
            if s.live_namespaces.contains(&update.namespace) {
                return;
            }
            let mut resolved = resolve_overlay(existing, &update);
            // The list carries no run: a row still in the same state keeps the one it has.
            if existing.state == update.state && update.active_run.is_none() {
                resolved.active_run = existing.active_run.clone();
            }
            s.runtime.insert(update.namespace.clone(), resolved.clone());
            if let Some(entry) = s.arrows.get_mut(&update.namespace) {
                patch_overlay(entry, &resolved);
            }
        });
    }

    /// After a reconnect the overlay painted by live frames can no longer be trusted: lets the next
    /// `seed_initial_state` replace it. The overlay itself stays until then, so rows do not blank.
    pub fn forget_live_state(&self) {
        self.store.update(|s| s.live_namespaces.clear());
    }

    pub fn reset(&self) {
        self.store.set(ArrowsState::default());
    }

    pub fn refresh_catalog(&self) {
        let refresh = self
            .refresh
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        refresh();
    }

    pub fn set_catalog_refresh(&self, refresh: impl Fn() + Send + Sync + 'static) {
        *self.refresh.lock().unwrap_or_else(PoisonError::into_inner) = Arc::new(refresh);
    }
}
