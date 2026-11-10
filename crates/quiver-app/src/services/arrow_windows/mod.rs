//! One webview per open arrow app, embedded in the main window (docs/decision-log.md: "Arrow apps:
//! embedded child webview"). This supervisor owns those pages: it opens one when an app is shown,
//! answers the page's `arrow-app` protocol requests through [`ProtocolService`], keeps at most
//! [`MAX_OPEN_APPS`] alive (the least recently used hidden one is sent `Close`), shows only the
//! visible app's page at the rectangle the pane reports, and ends every page on Stop and on
//! shutdown, so no page outlives its host.

mod embedded;
mod launcher;
mod pump;
mod recorder;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use quiver_arrow_host::{
    app_url, ArrowAppRegistry, ArrowHosts, ArrowUpstream, HostToWindow, PageBounds,
    ProtocolService, WsSession,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

pub use embedded::EmbeddedPages;
pub use launcher::{HelperArgs, HelperLauncher, HelperLink};

use pump::Pump;
use recorder::ProtocolLog;

use crate::services::Result;

/// A window going away for a reason the UI did not ask for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowEvent {
    /// The user closed the window, or its process died. The app is no longer open.
    Closed { namespace: String },
    /// The open-app cap closed this app's window to make room.
    Evicted { namespace: String },
}

struct Window {
    /// Distinguishes this window from a later one of the same app (Stop, then open again).
    generation: u64,
    to_helper: UnboundedSender<HostToWindow>,
    pump: JoinHandle<()>,
    /// Dropping it destroys the page.
    _guard: Box<dyn Send>,
}

#[derive(Default)]
struct State {
    registry: ArrowAppRegistry,
    /// Where each app's page goes, as last reported by the pane.
    bounds: HashMap<String, PageBounds>,
    /// Something drawn by GPUI (a menu, a dialog) is open over the page. A native view always paints
    /// above GPUI's own pixels, so the page steps aside while that is so.
    covered: bool,
    windows: HashMap<String, Window>,
    subscribers: Vec<UnboundedSender<WindowEvent>>,
    next_generation: u64,
    shut_down: bool,
}

impl State {
    fn emit(
        &mut self,
        event: WindowEvent,
    ) {
        self.subscribers.retain(|tx| tx.send(event.clone()).is_ok());
    }

    /// Ends `namespace`'s window: asks it to close, then drops the process handle, which kills
    /// whatever did not exit on its own.
    fn close_window(
        &mut self,
        namespace: &str,
    ) -> bool {
        let Some(window) = self.windows.remove(namespace) else {
            return false;
        };
        let _ = window.to_helper.send(HostToWindow::Close);
        window.pump.abort();
        true
    }
}

pub(super) struct Inner {
    launcher: Arc<dyn HelperLauncher>,
    hosts: Arc<ArrowHosts>,
    protocol: ProtocolService,
    upstream: Arc<dyn ArrowUpstream>,
    log: Arc<ProtocolLog>,
    state: Mutex<State>,
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The helper for `namespace` is gone without being asked to be.
    pub(super) fn window_gone(
        &self,
        namespace: &str,
        generation: u64,
    ) {
        let mut state = self.state();
        if state.windows.get(namespace).map(|w| w.generation) != Some(generation) {
            return;
        }
        state.close_window(namespace);
        state.registry.stop(namespace);
        state.emit(WindowEvent::Closed {
            namespace: namespace.to_string(),
        });
    }
}

/// Cheap to clone; all clones supervise the same windows.
#[derive(Clone)]
pub struct ArrowWindows {
    inner: Arc<Inner>,
}

impl ArrowWindows {
    pub fn new(
        launcher: Arc<dyn HelperLauncher>,
        hosts: Arc<ArrowHosts>,
        upstream: Arc<dyn ArrowUpstream>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                launcher,
                protocol: ProtocolService::new(hosts.clone(), upstream.clone()),
                upstream,
                hosts,
                log: Arc::default(),
                state: Mutex::default(),
            }),
        }
    }

    /// Window closed or evicted events, from now on.
    pub fn subscribe(&self) -> UnboundedReceiver<WindowEvent> {
        let (tx, rx) = unbounded_channel();
        self.inner.state().subscribers.push(tx);
        rx
    }

    /// Makes `namespace` the visible app, opening its window if it has none and closing the window
    /// the cap pushes out. Must run inside the async runtime.
    pub async fn show(
        &self,
        namespace: &str,
        title: &str,
    ) -> Result<()> {
        let mut state = self.inner.state();
        if state.shut_down {
            return Ok(());
        }
        let previous = state.registry.visible().map(str::to_owned);
        let shown = state.registry.show(namespace);
        if let Some(previous) = previous.filter(|p| p != namespace) {
            if let Some(window) = state.windows.get(&previous) {
                let _ = window
                    .to_helper
                    .send(HostToWindow::Visible { visible: false });
            }
        }
        for evicted in &shown.evicted {
            if state.close_window(evicted) {
                state.emit(WindowEvent::Evicted {
                    namespace: evicted.clone(),
                });
            }
        }
        let covered = state.covered;
        if let Some(window) = state.windows.get(namespace) {
            let _ = window
                .to_helper
                .send(HostToWindow::Visible { visible: !covered });
            return Ok(());
        }

        let args = HelperArgs {
            url: app_url(&self.inner.hosts.register(namespace)),
            title: title.to_string(),
        };
        let link = match self.inner.launcher.launch(&args) {
            Ok(link) => link,
            Err(e) => {
                state.registry.stop(namespace);
                return Err(e);
            }
        };
        let generation = state.next_generation;
        state.next_generation += 1;
        let (deliveries, to_page) = unbounded_channel();
        let sockets = WsSession::new(
            self.inner.hosts.register(namespace),
            self.inner.hosts.clone(),
            self.inner.upstream.clone(),
            deliveries,
        );
        let pump = tokio::spawn(
            Pump {
                inner: Arc::downgrade(&self.inner),
                protocol: self.inner.protocol.clone(),
                sockets,
                to_page,
                log: self.inner.log.clone(),
                namespace: namespace.to_string(),
                generation,
                from_helper: link.from_helper,
                to_helper: link.to_helper.clone(),
            }
            .run(),
        );
        if let Some(bounds) = state.bounds.get(namespace) {
            let _ = link.to_helper.send(HostToWindow::Bounds(*bounds));
        }
        let visible = !state.covered;
        let _ = link.to_helper.send(HostToWindow::Visible { visible });
        state.windows.insert(
            namespace.to_string(),
            Window {
                generation,
                to_helper: link.to_helper,
                pump,
                _guard: link.guard,
            },
        );
        Ok(())
    }

    /// The visible app's page goes away from view; it stays alive.
    pub fn hide(&self) {
        let mut state = self.inner.state();
        if let Some(window) = state
            .registry
            .visible()
            .and_then(|visible| state.windows.get(visible))
        {
            let _ = window
                .to_helper
                .send(HostToWindow::Visible { visible: false });
        }
        state.registry.hide();
    }

    /// A menu or dialog drawn by GPUI opened (`true`) or closed over the visible page.
    pub fn set_covered(
        &self,
        covered: bool,
    ) {
        let mut state = self.inner.state();
        if state.covered == covered {
            return;
        }
        state.covered = covered;
        if let Some(window) = state
            .registry
            .visible()
            .and_then(|visible| state.windows.get(visible))
        {
            let _ = window
                .to_helper
                .send(HostToWindow::Visible { visible: !covered });
        }
    }

    /// Where `namespace`'s page goes: the pane body's rectangle in the main window. Cheap to call on
    /// every frame; only a change reaches the page.
    pub fn set_bounds(
        &self,
        namespace: &str,
        bounds: PageBounds,
    ) {
        let mut state = self.inner.state();
        if state.bounds.get(namespace) == Some(&bounds) {
            return;
        }
        state.bounds.insert(namespace.to_owned(), bounds);
        if let Some(window) = state.windows.get(namespace) {
            let _ = window.to_helper.send(HostToWindow::Bounds(bounds));
        }
    }

    /// Remounts the app's page; its sockets die with the old document.
    pub fn reload(
        &self,
        namespace: &str,
    ) {
        let mut state = self.inner.state();
        state.registry.reload(namespace);
        if let Some(window) = state.windows.get(namespace) {
            let _ = window.to_helper.send(HostToWindow::Reload);
        }
    }

    /// Stop: the app leaves the stack and its window closes. False when it was not open.
    pub fn stop(
        &self,
        namespace: &str,
    ) -> bool {
        let mut state = self.inner.state();
        let was_open = state.registry.stop(namespace);
        state.close_window(namespace) || was_open
    }

    /// Closes the windows of apps whose surface is gone (`live` is the namespaces that still have
    /// one), e.g. after the daemon stopped them.
    pub fn prune(
        &self,
        live: &HashSet<String>,
    ) {
        let mut state = self.inner.state();
        for dropped in state.registry.prune(live) {
            state.close_window(&dropped);
        }
    }

    /// Open apps, least recently used first.
    pub fn open_apps(&self) -> Vec<String> {
        self.inner.state().registry.order().to_vec()
    }

    pub fn visible(&self) -> Option<String> {
        self.inner.state().registry.visible().map(str::to_owned)
    }

    /// The remount key of the app (0 until its first reload).
    pub fn reload_key(
        &self,
        namespace: &str,
    ) -> u64 {
        self.inner.state().registry.reload_key(namespace)
    }

    /// What the e2e state dump reports: the open apps, the visible one, each app's remount key and the
    /// protocol requests the helpers made (status and CSP of each answer).
    pub fn snapshot(&self) -> serde_json::Value {
        let state = self.inner.state();
        let reload_keys: serde_json::Map<String, serde_json::Value> = state
            .registry
            .order()
            .iter()
            .map(|ns| (ns.clone(), state.registry.reload_key(ns).into()))
            .collect();
        serde_json::json!({
            "open": state.registry.order(),
            "windows": state.windows.keys().collect::<Vec<_>>(),
            "visible": state.registry.visible(),
            "reloadKeys": reload_keys,
            "protocol": self.inner.log.snapshot(),
        })
    }

    /// Ends every helper. Idempotent; nothing opens afterwards.
    pub fn shutdown(&self) {
        let mut state = self.inner.state();
        state.shut_down = true;
        let open: Vec<String> = state.windows.keys().cloned().collect();
        for namespace in open {
            state.close_window(&namespace);
            state.registry.stop(&namespace);
        }
    }
}
