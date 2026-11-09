//! The runs in flight, so each can be cancelled by id.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::task::AbortHandle;

use super::exec::run_exec;
use crate::bridge::FrameSink;
use crate::transport::Transport;

/// Tracks the runs in flight so each can be cancelled by id, and so a page
/// reload can end every one its outgoing page started.
///
/// Aborting a run's task drops its response, which closes the connection,
/// which cancels the command on the daemon.
#[derive(Default)]
pub struct ConsoleExecManager {
    runs: Arc<Mutex<HashMap<String, AbortHandle>>>,
}

impl ConsoleExecManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run `line` on `transport` in the background, pushing its frames to `sink`,
    /// under `exec_id` so it can be cancelled. Returns at once.
    ///
    /// This is the whole of `console_exec`: the transport is
    /// resolved by the caller per call, so a command typed after a connection
    /// switch reaches the new daemon with nothing to re-register.
    ///
    /// The handle is the run itself: the app lets it go, a test awaits it to know
    /// the run has ended (or been cancelled) rather than waiting a while.
    pub fn start<S: FrameSink>(
        &self,
        exec_id: String,
        transport: Arc<dyn Transport>,
        line: String,
        sink: S,
        deadline: Duration,
    ) -> tokio::task::JoinHandle<()> {
        let runs = Arc::clone(&self.runs);
        let id = exec_id.clone();
        // The run waits to be registered before it starts, so it cannot finish -- and
        // forget itself -- before its entry exists, which would leave the entry in
        // the map for good.
        let (registered, wait) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let _ = wait.await;
            run_exec(transport.as_ref(), &line, &sink, deadline).await;
            runs.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
        });
        self.register(exec_id, task.abort_handle());
        let _ = registered.send(());
        task
    }

    pub(super) fn register(&self, exec_id: String, handle: AbortHandle) {
        // A re-used id strands the previous run, so it is aborted.
        if let Some(previous) = self
            .runs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(exec_id, handle)
        {
            previous.abort();
        }
    }

    /// Abort one run. Unknown ids are fine: the run may already have ended.
    pub fn cancel(&self, exec_id: &str) {
        if let Some(handle) = self
            .runs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(exec_id)
        {
            handle.abort();
        }
    }

    /// Abort every run: the page that started them is gone.
    pub fn cancel_all(&self) {
        for (_, handle) in self.runs.lock().unwrap_or_else(|e| e.into_inner()).drain() {
            handle.abort();
        }
    }

    #[cfg(test)]
    pub(super) fn in_flight(&self) -> usize {
        self.runs.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}
