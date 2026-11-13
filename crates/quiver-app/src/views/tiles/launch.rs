//! What an arrow can launch, for tiles. Library, shelf and search rows carry no manifest, so whether an
//! arrow has anything to run is learned from its manifest. Each arrow is asked once per session, the first
//! time a tile for it is drawn; the answers are kept in the state file, so after the first run a tile knows
//! at once and the session's lookup only refreshes it. Until an arrow is known (and for good when it has
//! nothing to run) its tile offers no Start button.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Mutex, OnceLock, PoisonError};

use gpui::{App, Entity};
use quiver_state::arrow_details::actions::launch_label;
use quiver_state::arrow_details::from_detail::facts_from_detail;
use quiver_state::i18n::en::arrow;
use quiver_state::i18n::Key;
use quiver_state::queries::arrow::fetch_arrow_detail;

use crate::model::AppModel;

const STORAGE_KEY: &str = "tile.launch.v1";

#[derive(Default)]
struct Cache {
    loaded: bool,
    /// namespace -> "none" | "start" | "open".
    known: BTreeMap<String, String>,
    asked: HashSet<String>,
}

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

fn label_of(word: &str) -> Option<Key> {
    match word {
        "start" => Some(arrow::ACTION_START),
        "open" => Some(arrow::ACTION_OPEN),
        _ => None,
    }
}

/// The launch label of an arrow once learned (`Some(None)`: known to have nothing to run); `None` while unknown.
/// Asks for it the first time in a session.
pub fn launch_of(
    namespace: &str,
    app: &Entity<AppModel>,
    cx: &App,
) -> Option<Option<Key>> {
    let (api, handle, store) = {
        let m = app.read(cx);
        (
            m.services.api.clone(),
            m.handle().clone(),
            m.services.key_value.clone(),
        )
    };
    let mut c = cache().lock().unwrap_or_else(PoisonError::into_inner);
    if !c.loaded {
        c.loaded = true;
        if let Ok(Some(text)) = store.get(STORAGE_KEY) {
            c.known = serde_json::from_str(&text).unwrap_or_default();
        }
    }
    let known = c.known.get(namespace).map(|w| label_of(w));
    if !c.asked.insert(namespace.to_owned()) {
        return known;
    }
    drop(c);
    let ns = namespace.to_owned();
    let job = handle.spawn(async move {
        let platform = quiver_platform::platform_key();
        let detail = fetch_arrow_detail(api.as_ref(), &ns).await.ok()?;
        let facts = facts_from_detail(&detail, &platform);
        let word = match facts.lifecycle.as_ref().filter(|l| !l.execute.is_empty()) {
            None => "none",
            Some(l) if launch_label(Some(l)) == arrow::ACTION_OPEN => "open",
            Some(_) => "start",
        };
        let mut c = cache().lock().unwrap_or_else(PoisonError::into_inner);
        let changed = c.known.insert(ns, word.to_owned()).as_deref() != Some(word);
        if changed {
            if let Ok(text) = serde_json::to_string(&c.known) {
                store.set(STORAGE_KEY, &text).ok();
            }
        }
        Some(changed)
    });
    let app = app.clone();
    cx.spawn(async move |cx| {
        if let Ok(Some(true)) = job.await {
            app.update(cx, |_, cx| cx.notify()).ok();
        }
    })
    .detach();
    known
}
