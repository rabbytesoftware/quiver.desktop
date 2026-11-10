//! The search the sidebar field and the results page share. The field outlives the page, and the page
//! is rebuilt for every query, so the store and the controller live here, once per process
//! (`GlobalSearch`), and the page only reads the store and tells the controller which query it shows.

use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use gpui::{App, Global};
use quiver_state::store::search_pass::{PlatformSource, SearchController};
use quiver_state::store::SearchStore;

use crate::model::app_model;

struct NativePlatform;

#[async_trait]
impl PlatformSource for NativePlatform {
    async fn platform(&self) -> String {
        quiver_platform::platform_key()
    }
}

pub struct SearchSession {
    pub store: SearchStore,
    controller: Mutex<SearchController>,
    api: Arc<dyn quiver_state::core_api::CoreApi>,
    handle: tokio::runtime::Handle,
}

struct GlobalSearch(Arc<SearchSession>);

impl Global for GlobalSearch {}

/// The process-wide session, created on first use.
pub fn session(cx: &mut App) -> Arc<SearchSession> {
    if let Some(global) = cx.try_global::<GlobalSearch>() {
        return global.0.clone();
    }
    let (api, handle) = {
        let model = app_model(cx).read(cx);
        (model.services.api.clone(), model.handle().clone())
    };
    let store = SearchStore::default();
    let controller = SearchController::new(api.clone(), store.clone(), Arc::new(NativePlatform));
    let session = Arc::new(SearchSession {
        store,
        controller: Mutex::new(controller),
        api,
        handle,
    });
    cx.set_global(GlobalSearch(session.clone()));
    session
}

impl SearchSession {
    fn controller(&self) -> SearchController {
        self.controller
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The page shows `query` (committed by the field, already debounced). The controller ignores a
    /// query it already holds, so a rebuilt page for the same query keeps its results.
    pub fn show(&self, query: &str) {
        let _runtime = self.handle.enter();
        self.controller().set_query(query, true);
    }

    /// Enter in the field: run the pass now. The field and the page are siblings, so the request goes
    /// through the store (`SearchState::submit_query`) and the page forwards it.
    pub fn request_submit(&self, query: &str) {
        self.store.request_submit(query);
    }

    /// The page's side of [`Self::request_submit`]: fires the pass if Enter was pressed on `query`.
    pub fn take_submit(&self, query: &str) {
        if self.store.snapshot().submit_query.as_deref() != Some(query) {
            return;
        }
        self.store.clear_submit();
        let _runtime = self.handle.enter();
        self.controller().submit();
    }

    /// Retry after "Quiver is not running": forget the result and ask again from scratch.
    pub fn restart(&self, query: &str) {
        let _runtime = self.handle.enter();
        self.controller().dispose();
        self.store.reset();
        let fresh = SearchController::new(
            self.api.clone(),
            self.store.clone(),
            Arc::new(NativePlatform),
        );
        *self
            .controller
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = fresh.clone();
        fresh.set_query(query, true);
    }
}
