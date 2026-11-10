//! One native window per open arrow app, each in its own helper process (docs/decision-log.md:
//! "Arrow apps: separate WebView process"). This supervisor owns those processes: it starts one
//! when an app is shown, answers the helper's `arrow-app` protocol requests through
//! [`ProtocolService`], keeps at most [`MAX_OPEN_APPS`] alive (the least recently used hidden one is
//! sent `Close`), and ends every helper on Stop, on the user closing a window and on shutdown, so no
//! window outlives its host.

mod launcher;
mod process;
mod recorder;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use quiver_arrow_host::{
    app_url, ArrowAppRegistry, ArrowHosts, ArrowUpstream, HostToWindow, ProtocolService,
    WindowToHost,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::{JoinHandle, JoinSet};

pub use launcher::{HelperArgs, HelperLauncher, HelperLink};
pub use process::{ProcessLauncher, HELPER_BINARY};

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
    /// Dropping it ends the helper process.
    _guard: Box<dyn Send>,
}

#[derive(Default)]
struct State {
    registry: ArrowAppRegistry,
    windows: HashMap<String, Window>,
    subscribers: Vec<UnboundedSender<WindowEvent>>,
    next_generation: u64,
    shut_down: bool,
}

impl State {
    fn emit(&mut self, event: WindowEvent) {
        self.subscribers.retain(|tx| tx.send(event.clone()).is_ok());
    }

    /// Ends `namespace`'s window: asks it to close, then drops the process handle, which kills
    /// whatever did not exit on its own.
    fn close_window(&mut self, namespace: &str) -> bool {
        let Some(window) = self.windows.remove(namespace) else {
            return false;
        };
        let _ = window.to_helper.send(HostToWindow::Close);
        window.pump.abort();
        true
    }
}

struct Inner {
    launcher: Arc<dyn HelperLauncher>,
    hosts: Arc<ArrowHosts>,
    protocol: ProtocolService,
    log: Arc<ProtocolLog>,
    state: Mutex<State>,
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The helper for `namespace` is gone without being asked to be.
    fn window_gone(&self, namespace: &str, generation: u64) {
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
                protocol: ProtocolService::new(hosts.clone(), upstream),
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
    pub async fn show(&self, namespace: &str, title: &str) -> Result<()> {
        let mut state = self.inner.state();
        if state.shut_down {
            return Ok(());
        }
        let shown = state.registry.show(namespace);
        for evicted in &shown.evicted {
            if state.close_window(evicted) {
                state.emit(WindowEvent::Evicted {
                    namespace: evicted.clone(),
                });
            }
        }
        if state.windows.contains_key(namespace) {
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
        let pump = tokio::spawn(pump(
            Arc::downgrade(&self.inner),
            self.inner.protocol.clone(),
            self.inner.log.clone(),
            namespace.to_string(),
            generation,
            link.from_helper,
            link.to_helper.clone(),
        ));
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

    /// The visible app's window goes away from view; it stays alive.
    pub fn hide(&self) {
        self.inner.state().registry.hide();
    }

    /// Remounts the app's page; its sockets die with the old document.
    pub fn reload(&self, namespace: &str) {
        let mut state = self.inner.state();
        state.registry.reload(namespace);
        if let Some(window) = state.windows.get(namespace) {
            let _ = window.to_helper.send(HostToWindow::Reload);
        }
    }

    /// Stop: the app leaves the stack and its window closes. False when it was not open.
    pub fn stop(&self, namespace: &str) -> bool {
        let mut state = self.inner.state();
        let was_open = state.registry.stop(namespace);
        state.close_window(namespace) || was_open
    }

    /// Closes the windows of apps whose surface is gone (`live` is the namespaces that still have
    /// one), e.g. after the daemon stopped them.
    pub fn prune(&self, live: &HashSet<String>) {
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
    pub fn reload_key(&self, namespace: &str) -> u64 {
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

/// Serves one helper until it says it closed or its process ends.
async fn pump(
    inner: Weak<Inner>,
    protocol: ProtocolService,
    log: Arc<ProtocolLog>,
    namespace: String,
    generation: u64,
    mut from_helper: UnboundedReceiver<WindowToHost>,
    to_helper: UnboundedSender<HostToWindow>,
) {
    // Dropped with the pump, which aborts any request still being answered.
    let mut answering = JoinSet::new();
    while let Some(message) = from_helper.recv().await {
        match message {
            WindowToHost::Ready => log::debug!("arrow window for {namespace} is ready"),
            WindowToHost::Closed => break,
            WindowToHost::ProtocolRequest(request) => {
                let protocol = protocol.clone();
                let to_helper = to_helper.clone();
                let (log, namespace) = (log.clone(), namespace.clone());
                answering.spawn(async move {
                    let response = protocol.answer(request.clone()).await;
                    log.record(&namespace, &request, &response);
                    let _ = to_helper.send(HostToWindow::ProtocolResponse(response));
                });
            }
        }
    }
    if let Some(inner) = inner.upgrade() {
        inner.window_gone(&namespace, generation);
    }
}
