//! Observing the DAEMON, which the app does not signal about (its own state, the OS process table).
//!
//! The rule for scenarios is "wait on a signal, never a sleep"; for the app that is a bridge signal or a node.
//! What happens inside a separate daemon process has neither, so those conditions are read the way the old
//! harness read them: a bounded re-read of the daemon API. `eventually` is the one place that does it, so the
//! exception is visible and small; every UI condition goes through `App` instead.

use std::future::Future;
use std::time::{Duration, Instant};

pub async fn eventually<T, F, Fut>(what: &str, timeout: Duration, mut check: F) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(found) = check().await {
            return found;
        }
        assert!(
            Instant::now() < deadline,
            "timed out after {timeout:?} waiting for {what}"
        );
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

pub const LONG: Duration = Duration::from_secs(120);
pub const SHORT: Duration = Duration::from_secs(30);
