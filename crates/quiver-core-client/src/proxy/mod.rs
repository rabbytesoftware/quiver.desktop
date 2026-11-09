//! The `quiver://` URI scheme: every HTTP call the webview makes.
//!
//! Requests arrive as `quiver://localhost/v0/...` and are forwarded to whatever
//! connection is active — resolved PER REQUEST, so switching connections takes
//! effect on the next call with nothing to re-register.
//!
//! # Why this handler speaks CORS
//!
//! The page is served from the webview's own origin (`tauri://localhost`, or
//! `http://tauri.localhost` on Windows) and this scheme is a DIFFERENT origin,
//! so every `fetch()` the frontend makes here is cross-origin. Tauri adds
//! `Access-Control-Allow-Origin` for its own asset protocol and for nothing
//! else, so a handler that sets no CORS headers gets two things wrong:
//!
//!   1. `x-quiver-proxy` is not a CORS-safelisted response header, so without
//!      `Access-Control-Expose-Headers` the frontend reads `null` for it, and
//!      `isProxyFailure` in `src/lib/transport/api.ts` never returns true —
//!      the entire cold-start retry path is dead code.
//!   2. `DELETE`, and `POST` with a JSON content type, are not simple requests,
//!      so the browser sends an `OPTIONS` preflight first. Forwarded to gin,
//!      that answers whatever gin feels like; answered here, it answers what
//!      this proxy actually permits.
//!
//! Both are fixed below, on EVERY response — the marker is read on error
//! responses, so an error that omits the expose header is exactly as dead as
//! one that omits the marker.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use ::http::{header, HeaderValue, Method, Request, Response};

use crate::transport::Transport;

mod policy;

use policy::{is_refused_path, request_allowed};

/// Marks a response this proxy generated rather than relayed. `apiFetch`
/// retries idempotent reads on a marked 502/504 and never on an unmarked one:
/// a daemon's own 404 is meaningful and a 500 is a real server error, so
/// replaying either would be wrong.
pub const PROXY_ERROR_HEADER: &str = "x-quiver-proxy";

/// Who may read a response from this scheme.
///
/// `*` rather than the window's origin, which is what Tauri's asset protocol
/// echoes back. Two reasons. The origin differs per platform and per dev/bundle
/// mode (`tauri://localhost`, `http://tauri.localhost`, `http://localhost:5173`)
/// and nothing hands this handler that string; and `*` costs nothing here
/// because no browser credential is ever attached — the webview sends no
/// cookies to this scheme and the bearer token is added by the transport, in
/// Rust, after the request has left the page. A wildcard would matter if a
/// foreign origin could read what this handler returns, so `proxy_once` refuses
/// any request whose `Origin` is missing or not one of the app's own
/// (`request_allowed`) before a connection is even resolved. That matters because arrow pages
/// (`arrow-app://...`) run in iframes of this same webview, where this scheme
/// is reachable; they are additionally confined by their CSP
/// (`connect-src 'self'`).
const ALLOW_ORIGIN: HeaderValue = HeaderValue::from_static("*");

/// The methods `apiFetch` and the connection commands actually send. Listing
/// only these, rather than `*`, keeps the preflight answer an honest statement
/// of what the proxy serves.
const ALLOW_METHODS: HeaderValue = HeaderValue::from_static("GET, POST, DELETE, PATCH, OPTIONS");

/// The only header the frontend sets on a request that a preflight can reject.
/// (`Authorization` is not here on purpose: the page never sets it — the
/// transport adds the bearer server-side.)
const ALLOW_HEADERS: HeaderValue = HeaderValue::from_static("Content-Type");

/// Without this the frontend's `res.headers.get('x-quiver-proxy')` is `null` on
/// a cross-origin response, no matter what this proxy sets, because the header
/// is not on the CORS-safelist.
const EXPOSE_HEADERS: HeaderValue = HeaderValue::from_static(PROXY_ERROR_HEADER);

/// How long a preflight answer may be cached. Ten minutes: long enough that a
/// burst of DELETEs costs one preflight, short enough that changing the lists
/// above takes effect within one sitting.
const PREFLIGHT_MAX_AGE: HeaderValue = HeaderValue::from_static("600");

/// Upper bound on one proxied request.
///
/// Each request costs this process a socket. A daemon that accepts a connection
/// but never answers would otherwise pin that descriptor — and the tasks driving
/// it — for the life of the app: nothing caps requests in flight, and the
/// frontend's `fetch` has no AbortController, so it never gives up either.
/// Dropping the timed-out future drops the request sender, which is what tells
/// hyper's connection task to close and hand the descriptor back.
#[cfg(not(test))]
const PROXY_TIMEOUT: Duration = Duration::from_secs(300);

/// Same constant, shortened under test. `tokio`'s `test-util` feature — needed
/// to pause the clock and jump straight to a timer — is not enabled in this
/// crate, so a real but tiny wait is the remaining option that costs the suite
/// half a second instead of an actual five minutes.
///
/// It must stay ABOVE `http::CONNECT_TIMEOUT`, which is what ships (10s under
/// 300s) and what `the_proxy_ceiling_stays_above_the_transports_connect_ceiling`
/// pins. At the 20ms this used to be, that ordering was INVERTED under test —
/// the proxy's ceiling sat below the transport's own connect ceiling, so a
/// request to a dead port raced its connect failure against this timeout.
/// Loopback refuses in microseconds on Unix and the failure always won; the
/// Windows runner is slower than that, the timeout won, and two tests that
/// assert on the marked 502 a connect failure produces saw the marked 504 of a
/// proxy that gave up first. Above `CONNECT_TIMEOUT` there is no race left to
/// lose: reqwest bounds the connect itself and reports the expiry as a connect
/// error, so a dead port is a `Error::Connect` within 250ms on any
/// platform, however slowly the network stack answers.
#[cfg(test)]
const PROXY_TIMEOUT: Duration = Duration::from_millis(500);

/// No `quiver://` document should render inside a frame. An arrow page can
/// navigate its own iframe here (CSP governs fetches, not a frame navigating
/// itself). WebKitGTK does NOT enforce either header on custom scheme
/// responses (seen in the Linux E2E run), so these are a secondary guard:
/// the primary one is that such a navigation carries no Origin and
/// `request_allowed` refuses it. The shell only ever `fetch`es this scheme,
/// where both headers are inert.
const FRAME_ANCESTORS: HeaderValue = HeaderValue::from_static("frame-ancestors 'none'");
const FRAME_OPTIONS: HeaderValue = HeaderValue::from_static("DENY");

/// Stamp the headers every response from this scheme must carry, whoever built
/// it. `insert` rather than `append`: a daemon that sets its own
/// `Access-Control-Allow-Origin` must not end up with two, which browsers
/// reject outright, and its own framing headers must not survive next to ours.
fn with_cors(mut resp: Response<Vec<u8>>) -> Response<Vec<u8>> {
    let headers = resp.headers_mut();
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN);
    headers.insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, EXPOSE_HEADERS);
    headers.insert(header::CONTENT_SECURITY_POLICY, FRAME_ANCESTORS);
    headers.insert(header::X_FRAME_OPTIONS, FRAME_OPTIONS);
    resp
}

fn error_response(status: u16, msg: &str) -> Response<Vec<u8>> {
    // The builder is given a status code, two fixed headers and a body, none of
    // which can fail; the fallback exists only so no panic path is left in.
    with_cors(
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/plain")
            .header(PROXY_ERROR_HEADER, "error")
            .body(msg.as_bytes().to_vec())
            .unwrap_or_else(|_| Response::new(msg.as_bytes().to_vec())),
    )
}

/// The answer to a CORS preflight. Built here rather than forwarded: gin knows
/// nothing about the origins this app runs on, and a preflight that reaches the
/// daemon has already cost a socket and a round trip for a question only this
/// process can answer.
fn preflight_response() -> Response<Vec<u8>> {
    let mut resp = with_cors(
        Response::builder()
            .status(204)
            .body(Vec::new())
            .unwrap_or_else(|_| Response::new(Vec::new())),
    );
    let headers = resp.headers_mut();
    headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, ALLOW_METHODS);
    headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, ALLOW_HEADERS);
    headers.insert(header::ACCESS_CONTROL_MAX_AGE, PREFLIGHT_MAX_AGE);
    resp
}

/// The whole proxy decision, free of any webview plumbing so it can be driven
/// against a real socket. Everything worth asserting on lives here.
///
/// `transport` is a FUTURE, not a reference, and it is awaited INSIDE the
/// timeout. Resolving the active connection means taking `ConnectionManager`'s
/// read lock, and tokio's `RwLock` is fair: a reader that arrives while a
/// writer waits queues behind that writer. Awaited before the timeout, as this
/// used to be, a request that never even reaches a transport waits forever —
/// past `PROXY_TIMEOUT`, past anything. Inside it, the worst case is the same
/// marked 504 a stalled daemon produces, which is a thing the frontend already
/// knows how to retry.
pub async fn proxy_once<F>(transport: F, req: Request<Vec<u8>>) -> Response<Vec<u8>>
where
    F: Future<Output = Arc<dyn Transport>>,
{
    if !request_allowed(&req) {
        return error_response(403, "origin not allowed");
    }

    if is_refused_path(req.uri().path()) {
        return error_response(403, "path not served on this scheme");
    }

    // Answered before the transport is even resolved: a preflight asks what
    // THIS proxy permits, and no daemon can answer that.
    if req.method() == Method::OPTIONS {
        return preflight_response();
    }

    let proxied = async move {
        let transport = transport.await;
        transport.request(req).await
    };

    match tokio::time::timeout(PROXY_TIMEOUT, proxied).await {
        Ok(Ok(resp)) => with_cors(resp),
        Ok(Err(e)) => error_response(502, &format!("quiver proxy: {e}")),
        Err(_) => error_response(504, "quiver.core did not answer in time"),
    }
}

#[cfg(test)]
mod tests;
