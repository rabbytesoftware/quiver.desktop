//! What an arrow can launch, for tiles. Library, shelf and search rows carry no manifest, so whether an
//! arrow has anything to run is learned from its manifest, once per arrow, the first time a tile for it is
//! drawn. Until it is known (and for good when there is nothing to run) the tile offers no Start button.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock, PoisonError};

use gpui::{App, Entity};
use quiver_state::arrow_details::facts::LifecycleFacts;
use quiver_state::arrow_details::from_detail::facts_from_detail;
use quiver_state::queries::arrow::fetch_arrow_detail;

use crate::model::AppModel;

#[derive(Default)]
struct Cache {
    known: HashMap<String, Option<LifecycleFacts>>,
    asked: HashSet<String>,
}

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

/// The arrow's lifecycle on this machine once learned; `None` while unknown. Asks for it the first time.
pub fn lifecycle_of(
    namespace: &str,
    app: &Entity<AppModel>,
    cx: &App,
) -> Option<Option<LifecycleFacts>> {
    let mut c = cache().lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(known) = c.known.get(namespace) {
        return Some(known.clone());
    }
    if !c.asked.insert(namespace.to_owned()) {
        return None;
    }
    drop(c);
    let (api, handle) = {
        let m = app.read(cx);
        (m.services.api.clone(), m.handle().clone())
    };
    let ns = namespace.to_owned();
    let job = handle.spawn(async move {
        let platform = quiver_platform::platform_key();
        let lifecycle = fetch_arrow_detail(api.as_ref(), &ns)
            .await
            .ok()
            .map(|detail| facts_from_detail(&detail, &platform).lifecycle);
        if let Some(lifecycle) = lifecycle {
            cache()
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .known
                .insert(ns, lifecycle);
            true
        } else {
            false
        }
    });
    let app = app.clone();
    cx.spawn(async move |cx| {
        if job.await.unwrap_or(false) {
            app.update(cx, |_, cx| cx.notify()).ok();
        }
    })
    .detach();
    None
}
