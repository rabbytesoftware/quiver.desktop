//! Connects the bridge to the real app: the state dump, the `core.ready` signal and the per-route signals.

use std::sync::Arc;

use gpui::{AnyWindowHandle, App, Entity};
use quiver_state::domain::connection::ConnectionStatus;
use serde_json::json;

use super::{signal, start, StateSnapshot};
use crate::model::AppModel;

/// Starts the bridge (when `QUIVER_E2E_PORT` is set) for the main window.
pub fn attach(
    cx: &mut App,
    window: AnyWindowHandle,
    model: &Entity<AppModel>,
) {
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
        // What the app's store knows of each open interface (the daemon's own view is read straight from it).
        let surfaces: serde_json::Map<String, serde_json::Value> = for_state
            .arrows
            .entries()
            .into_iter()
            .filter_map(|e| {
                let surface = e.active_run?.surface?;
                Some((e.namespace, json!({"ready": surface.ready})))
            })
            .collect();
        let states: serde_json::Map<String, serde_json::Value> = for_state
            .arrows
            .entries()
            .into_iter()
            .map(|e| (e.namespace, json!(format!("{:?}", e.state).to_lowercase())))
            .collect();
        json!({
            "arrowStates": states,
            "surfaces": surfaces,
            "status": format!("{:?}", for_state.status.status()).to_lowercase(),
            "activeConnection": connections.active_id,
            "connections": connections.connections.iter().map(|c| &c.id).collect::<Vec<_>>(),
            "arrows": arrows,
            "arrowWindows": for_state.arrow_windows.snapshot(),
            "nativeMenu": crate::services::native_menu::record(),
        })
    });
    start(cx, window, runtime.clone(), state);
    // Core readiness is a signal, so a test waits on it instead of sleeping. `connection.ready:<id>` says the same
    // for each connection once it is the active one and its daemon answered (a remote added later, a switch back).
    let mut status_revisions = services.status.store().subscribe();
    let mut connection_revisions = services.connections.store().subscribe();
    runtime.spawn(async move {
        loop {
            if services.status.status() == ConnectionStatus::Ready {
                signal("core.ready");
                signal(&format!(
                    "connection.ready:{}",
                    services.connections.snapshot().active_id
                ));
            }
            tokio::select! {
                changed = status_revisions.changed() => if changed.is_err() { break },
                changed = connection_revisions.changed() => if changed.is_err() { break },
            }
        }
    });
}
