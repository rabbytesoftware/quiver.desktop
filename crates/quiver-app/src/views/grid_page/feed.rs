use std::future::Future;
use std::sync::Arc;

use quiver_state::core_api::CoreApi;
use quiver_state::domain::ConnectionStatus;
use quiver_state::queries::keys::QueryKey;
use quiver_state::queries::Query;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::services::AppServices;

/// A running read; dropping it stops the loop.
pub struct Feed(tokio::task::JoinHandle<()>);

impl Drop for Feed {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Reads with `fetch` once the connection is ready, again whenever something invalidates `key`, and
/// mirrors what a mutation wrote optimistically into the query client's cache for `key` (follow /
/// unfollow flip `followed` before Core answers, and roll back on error).
pub fn start_feed<T, F, Fut>(
    services: &Arc<AppServices>,
    query: Query<T>,
    key: QueryKey,
    fetch: F,
) -> Feed
where
    T: Clone + PartialEq + Serialize + DeserializeOwned + Send + Sync + 'static,
    F: Fn(Arc<dyn CoreApi>) -> Fut + Send + 'static,
    Fut: Future<Output = quiver_state::Result<T>> + Send + 'static,
{
    let (api, queries) = (services.api.clone(), services.queries.clone());
    let status = services.status.store().clone();
    Feed(services.handle().spawn(async move {
        let mut status_changes = status.subscribe();
        let mut invalidations = queries.subscribe();
        let mut seen_epoch = usize::MAX;
        let mut fetched = false;
        loop {
            let ready = status.snapshot() == ConnectionStatus::Ready;
            let epoch = queries.epoch(&key);
            if ready && (!fetched || epoch != seen_epoch) {
                fetched = true;
                seen_epoch = epoch;
                query.run(fetch(api.clone())).await;
                if let Some(data) = query.snapshot().data {
                    queries.set_data(&key, &data);
                }
            } else if !ready {
                fetched = false;
            } else if let Some(cached) = queries.get_data::<T>(&key) {
                if query.snapshot().data.as_ref() != Some(&cached) {
                    query.store().update(|s| s.data = Some(cached));
                }
            }
            tokio::select! {
                changed = status_changes.changed() => if changed.is_err() { break },
                changed = invalidations.changed() => if changed.is_err() { break },
            }
        }
    }))
}
