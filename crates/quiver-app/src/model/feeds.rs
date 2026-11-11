//! Background loops that keep app-wide queries fresh. They run on the tokio runtime and only write
//! to `quiver-state` stores; views see the result through `observe_store`.

use std::sync::Arc;

use quiver_state::console::{parse_versions, CoreVersions};
use quiver_state::core_api::CoreApiExt;
use quiver_state::domain::{ConnectionStatus, Home};
use quiver_state::observable::Store;
use quiver_state::queries::home::{fetch_home, refetch_interval};
use quiver_state::queries::keys::home_key;
use quiver_state::queries::Query;

use crate::services::AppServices;

/// Reads `GET /v0/home` once the connection is ready, again while Core says a refresh is in flight
/// (`refetch_interval`), and whenever something invalidates the home query.
pub fn start_home_feed(
    services: &Arc<AppServices>,
    query: Query<Home>,
) {
    let (api, queries) = (services.api.clone(), services.queries.clone());
    let status = services.status.store().clone();
    services.handle().spawn(async move {
        let mut status_changes = status.subscribe();
        let mut invalidations = queries.subscribe();
        let key = home_key();
        let mut seen_epoch = queries.epoch(&key);
        let mut fetched_for_status = None;
        loop {
            let now = status.snapshot();
            let epoch = queries.epoch(&key);
            if now == ConnectionStatus::Ready
                && (fetched_for_status != Some(now) || epoch != seen_epoch)
            {
                fetched_for_status = Some(now);
                seen_epoch = epoch;
                query.run(fetch_home(api.as_ref())).await;
            } else if now != ConnectionStatus::Ready {
                fetched_for_status = None;
            }
            let wait = refetch_interval(query.snapshot().data.as_ref());
            let timer = async {
                match wait {
                    Some(after) => tokio::time::sleep(after).await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                () = timer => {
                    seen_epoch = seen_epoch.wrapping_sub(1);
                }
                changed = status_changes.changed() => if changed.is_err() { break },
                changed = invalidations.changed() => if changed.is_err() { break },
            }
        }
    });
}

/// Reads the daemon's build (`GET /versions`) each time the connection becomes ready, for the build
/// indicator. A daemon that does not answer leaves the indicator on the app line alone.
pub fn start_versions_feed(
    services: &Arc<AppServices>,
    versions: Store<Option<CoreVersions>>,
) {
    let (api, queries) = (services.api.clone(), services.queries.clone());
    let status = services.status.store().clone();
    services.handle().spawn(async move {
        let mut changes = status.subscribe();
        // A core self-update swaps the daemon behind a connection that may never leave Ready; the swap
        // invalidates the arrow queries, so any invalidation re-reads the build too.
        let mut invalidations = queries.subscribe();
        loop {
            if status.snapshot() == ConnectionStatus::Ready {
                if let Ok(read) = api.get_json("/versions").await {
                    versions.set(parse_versions(&read));
                }
            } else {
                versions.set(None);
            }
            tokio::select! {
                changed = changes.changed() => if changed.is_err() { break },
                changed = invalidations.changed() => if changed.is_err() { break },
            }
        }
    });
}
