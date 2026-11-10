//! Reads one arrow for the page: the fast pair (`GET /v0/arrow/:ns` and its manifest) gates the
//! screen, then the slow reads (readme, dependencies, dependents) overlay it as they land. The task
//! reads again whenever something invalidates the arrow queries or the connection comes back.

use std::sync::Arc;

use quiver_state::domain::{ArrowDetail, ConnectionStatus};
use quiver_state::observable::Store;
use quiver_state::queries::arrow::{
    fetch_arrow_detail, fetch_dependencies, fetch_dependents, fetch_readme, is_enabled,
};
use quiver_state::queries::keys::arrow_detail_key;
use tokio::task::JoinHandle;

use crate::services::AppServices;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Load {
    pub detail: Option<ArrowDetail>,
    pub error: Option<String>,
    /// The daemon answered 404: this arrow does not resolve.
    pub not_found: bool,
    pub loading: bool,
}

/// Stops the read loop with the view that started it.
pub struct ReadTask(pub JoinHandle<()>);

impl Drop for ReadTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub fn start(services: &Arc<AppServices>, namespace: String, load: Store<Load>) -> ReadTask {
    let (api, queries) = (services.api.clone(), services.queries.clone());
    let status = services.status.store().clone();
    ReadTask(services.handle().spawn(async move {
        if !is_enabled(&namespace) {
            return;
        }
        let key = arrow_detail_key(&namespace);
        let mut status_changes = status.subscribe();
        let mut invalidations = queries.subscribe();
        let mut seen = None;
        loop {
            let ready = status.snapshot() == ConnectionStatus::Ready;
            let epoch = queries.epoch(&key);
            if ready && seen != Some(epoch) {
                seen = Some(epoch);
                load.update(|l| {
                    l.loading = true;
                    l.error = None;
                });
                match fetch_arrow_detail(api.as_ref(), &namespace).await {
                    Ok(detail) => {
                        load.update(|l| {
                            l.loading = false;
                            l.not_found = false;
                            // The slow fields of the last read stay until the new ones land.
                            let slow = l.detail.take();
                            l.detail = Some(keep_slow_fields(detail, slow));
                        });
                        let (readme, deps, dependents) = tokio::join!(
                            fetch_readme(api.as_ref(), &namespace),
                            fetch_dependencies(api.as_ref(), &namespace),
                            fetch_dependents(api.as_ref(), &namespace)
                        );
                        load.update(|l| {
                            if let Some(d) = &mut l.detail {
                                if let Ok(readme) = readme {
                                    d.readme = readme;
                                }
                                if let Ok(deps) = deps {
                                    d.dependencies = deps;
                                }
                                if let Ok(dependents) = dependents {
                                    d.dependents = dependents;
                                }
                            }
                        });
                    }
                    Err(err) => load.update(|l| {
                        l.loading = false;
                        l.not_found = err.is_not_found();
                        l.error = Some(err.to_string());
                    }),
                }
            }
            tokio::select! {
                changed = status_changes.changed() => if changed.is_err() { break },
                changed = invalidations.changed() => if changed.is_err() { break },
            }
        }
    }))
}

fn keep_slow_fields(mut fresh: ArrowDetail, previous: Option<ArrowDetail>) -> ArrowDetail {
    if let Some(old) = previous {
        fresh.readme = old.readme;
        fresh.dependencies = old.dependencies;
        fresh.dependents = old.dependents;
        fresh.channels = old.channels;
    }
    fresh
}
