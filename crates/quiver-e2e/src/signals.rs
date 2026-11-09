//! Named, latched signals: the "wait on a signal, never a sleep" primitive.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

/// A set of signals that have fired. Raising is idempotent; waiting returns at once for a signal already raised.
#[derive(Clone, Default)]
pub struct Signals {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    fired: Mutex<HashSet<String>>,
    changed: Notify,
}

impl Signals {
    pub fn raise(&self, name: &str) {
        if let Ok(mut fired) = self.inner.fired.lock() {
            fired.insert(name.to_string());
        }
        self.inner.changed.notify_waiters();
    }

    pub fn has(&self, name: &str) -> bool {
        self.inner.fired.lock().map(|f| f.contains(name)).unwrap_or(false)
    }

    /// Resolves once `name` has been raised.
    pub async fn wait(&self, name: &str) {
        loop {
            let notified = self.inner.changed.notified();
            tokio::pin!(notified);
            // Register interest before checking, so a raise between the check and the await is not lost.
            notified.as_mut().enable();
            if self.has(name) {
                return;
            }
            notified.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_signal_raised_before_waiting_returns_at_once() {
        let s = Signals::default();
        s.raise("ready");
        s.wait("ready").await;
    }

    #[tokio::test]
    async fn waiting_wakes_on_a_later_raise() {
        let s = Signals::default();
        let waiter = tokio::spawn({
            let s = s.clone();
            async move { s.wait("go").await }
        });
        s.raise("other");
        s.raise("go");
        waiter.await.unwrap();
    }
}
