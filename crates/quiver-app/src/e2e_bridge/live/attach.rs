//! Connects the bridge to the real app: the state dump, the `core.ready` signal and the per-route signals.

use std::sync::Arc;

use gpui::{AnyWindowHandle, App, Entity};
use quiver_state::domain::connection::ConnectionStatus;
use serde_json::json;

use super::{signal, start, StateSnapshot};
use crate::model::AppModel;

/// Starts the bridge (when `QUIVER_E2E_PORT` is set) for the main window.
pub fn attach(cx: &mut App, window: AnyWindowHandle, model: &Entity<AppModel>) {
    super::logs::install_logger();
    let services = model.read(cx).services.clone();
    let runtime = model.read(cx).handle().clone();
    let for_state = services.clone();
    let state: StateSnapshot = Arc::new(move || {
        let connections = for_state.connections.snapshot();
        let arrows: Vec<String> = for_state
            .arrows
            .store()
            .read(|s| s.arrows.keys().cloned().collect());
        json!({
            "status": format!("{:?}", for_state.status.status()).to_lowercase(),
            "activeConnection": connections.active_id,
            "connections": connections.connections.iter().map(|c| &c.id).collect::<Vec<_>>(),
            "arrows": arrows,
        })
    });
    start(cx, window, runtime.clone(), state);
    // Core readiness is a signal, so a test waits on it instead of sleeping.
    let mut revisions = services.status.store().subscribe();
    runtime.spawn(async move {
        loop {
            if services.status.status() == ConnectionStatus::Ready {
                signal("core.ready");
            }
            if revisions.changed().await.is_err() {
                break;
            }
        }
    });
}
