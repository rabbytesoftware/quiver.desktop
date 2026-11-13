//! The tiny observable layer: shared state plus a revision a view can subscribe to.

use std::sync::{Arc, PoisonError, RwLock};

use tokio::sync::watch;

/// Shared state behind an `Arc<RwLock<T>>`. Every [`Store::update`] bumps a revision counter that
/// [`Store::subscribe`] receivers observe, so a GPUI view can re-render on change.
pub struct Store<T> {
    value: Arc<RwLock<T>>,
    revision: Arc<watch::Sender<u64>>,
}

impl<T> Clone for Store<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            revision: Arc::clone(&self.revision),
        }
    }
}

impl<T: Default> Default for Store<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Store<T> {
    pub fn new(value: T) -> Self {
        let (tx, _rx) = watch::channel(0);
        Self {
            value: Arc::new(RwLock::new(value)),
            revision: Arc::new(tx),
        }
    }

    /// Reads through a closure, so no lock guard escapes.
    pub fn read<R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> R {
        f(&self.value.read().unwrap_or_else(PoisonError::into_inner))
    }

    /// Mutates and notifies subscribers.
    pub fn update<R>(
        &self,
        f: impl FnOnce(&mut T) -> R,
    ) -> R {
        let out = self.update_quiet(f);
        self.notify();
        out
    }

    /// Mutates WITHOUT notifying: for a caller that decides afterwards, with [`Store::notify`], whether anything
    /// observable changed (a poll that returns what the views already show must not repaint the window).
    pub fn update_quiet<R>(
        &self,
        f: impl FnOnce(&mut T) -> R,
    ) -> R {
        f(&mut self.value.write().unwrap_or_else(PoisonError::into_inner))
    }

    /// Wakes subscribers without touching the value.
    pub fn notify(&self) {
        self.revision.send_modify(|r| *r += 1);
    }

    /// Replaces the value and notifies subscribers.
    pub fn set(
        &self,
        value: T,
    ) {
        self.update(|v| *v = value);
    }

    pub fn revision(&self) -> u64 {
        *self.revision.borrow()
    }

    /// A receiver that changes every time the state does.
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.revision.subscribe()
    }
}

impl<T: Clone> Store<T> {
    pub fn snapshot(&self) -> T {
        self.read(T::clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn update_bumps_the_revision_and_wakes_subscribers() {
        let store = Store::new(1);
        let mut rx = store.subscribe();
        store.update(|v| *v += 1);
        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow(), 1);
        assert_eq!(store.snapshot(), 2);
        assert_eq!(store.revision(), 1);
    }

    #[test]
    fn clones_share_state() {
        let a = Store::new(String::from("x"));
        let b = a.clone();
        b.set("y".into());
        assert_eq!(a.read(|s| s.clone()), "y");
    }
}
