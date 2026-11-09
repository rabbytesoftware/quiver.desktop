//! The reachability probe.

use std::time::Duration;

use crate::transport::http::HttpTransport;
use crate::transport::Transport;
use crate::{Error, Result};

/// How long the reachability probe in [`RemoteConnection::start`] may take.
///
/// Its own deadline, and a short one, because it cannot share the transport's:
/// `http::REQUEST_TIMEOUT` has to sit above the proxy's 300s ceiling so that a
/// long proxied request is bounded by the proxy rather than silently truncated
/// here — and a probe that takes five and a half minutes to say "unreachable"
/// is not a probe. `tokio::time::timeout` at the call site rather than a second
/// `reqwest::Client` so that `unix::UnixTransport`, which has no reqwest client
/// at all, is covered by the same bound.
///
/// This is what a switch's `prepare` step blocks on (`ConnectionManager::switch_to`),
/// so it is also the longest a switch to a stalled peer can hold `switching`.
#[cfg(not(test))]
const HEALTH_TIMEOUT: Duration = Duration::from_secs(10);

/// Same constant, shortened under test, for the reason `proxy::PROXY_TIMEOUT`
/// documents: no `test-util` feature in this crate, so the wait is real.
#[cfg(test)]
const HEALTH_TIMEOUT: Duration = Duration::from_millis(20);

fn health_request() -> Result<::http::Request<Vec<u8>>> {
    ::http::Request::builder()
        .method("GET")
        .uri("quiver://localhost/v0/health")
        .body(Vec::new())
        .map_err(|e| Error::Health(e.to_string()))
}

/// Is this peer answering right now? The probe half of [`RemoteConnection::start`],
/// lifted out of it so it can be driven without an `EventSink` — `start`'s other
/// job is emitting status, which needs one.
///
/// Bounded by [`HEALTH_TIMEOUT`]. The unbounded version of this call is the one
/// that wedged the app: a peer that completed a TCP handshake and then said
/// nothing left it pending forever, under `ConnectionManager`'s write lock.
/// The lock is gone (see `manager::ConnectionManager::switch_to`) and so is the
/// unboundedness — either alone would have been half a fix, because a switch
/// that never returns is still a switch the user cannot escape.
pub async fn probe_health(transport: &dyn Transport) -> Result<()> {
    let req = health_request()?;
    match tokio::time::timeout(HEALTH_TIMEOUT, transport.request(req)).await {
        Ok(Ok(resp)) if resp.status().is_success() => Ok(()),
        Ok(Ok(resp)) => Err(Error::Health(format!(
            "health check returned status {}",
            resp.status()
        ))),
        Ok(Err(e)) => Err(Error::Health(format!("health check failed: {e}"))),
        Err(_) => Err(Error::Health(format!(
            "health check timed out after {}ms",
            HEALTH_TIMEOUT.as_millis()
        ))),
    }
}

/// Stage one of adding a remote: is anything answering at this URL at all,
/// before the user is asked for a pairing code that only the right daemon can
/// even accept. Tokenless and stateless -- `/v0/health` needs no auth, and
/// this candidate is not yet, and may never become, a saved connection.
pub async fn check_remote_health(url: impl Into<String>) -> Result<()> {
    let transport = HttpTransport::new(url, None);
    probe_health(&transport).await
}
