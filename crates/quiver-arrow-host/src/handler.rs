//! The `arrow-app` URI scheme: every request an arrow's page makes.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use http::Uri;

use crate::csp::harden;
use crate::hosts::ArrowHosts;
use crate::inject::inject_shim;
use crate::message::{ArrowRequest, ArrowResponse};
use crate::upstream::ArrowUpstream;
use crate::uri::{daemon_path, host_of};
use crate::{SHIM_JS, SHIM_PATH};

/// Same bound as the `quiver://` proxy; shorter under test so a hung
/// upstream does not stall the suite.
#[cfg(not(test))]
const ARROW_TIMEOUT: Duration = Duration::from_secs(300);
#[cfg(test)]
const ARROW_TIMEOUT: Duration = Duration::from_millis(500);

/// Headers that must not travel from an arrow's page to the daemon.
const STRIPPED: [&str; 6] = [
    "origin",
    "referer",
    "cookie",
    "authorization",
    "host",
    "accept-encoding",
];

fn plain(
    status: u16,
    body: &str,
) -> ArrowResponse {
    let mut resp = ArrowResponse::new(status, body.as_bytes().to_vec());
    resp.headers
        .set("content-type", "text/plain; charset=utf-8");
    harden(&mut resp);
    resp
}

fn shim_response(method: &str) -> ArrowResponse {
    let body = if method.eq_ignore_ascii_case("HEAD") {
        Vec::new()
    } else {
        SHIM_JS.as_bytes().to_vec()
    };
    let mut resp = ArrowResponse::new(200, body);
    resp.headers.set("content-type", "text/javascript");
    resp.headers.set("cache-control", "no-store");
    harden(&mut resp);
    resp
}

fn is_html(resp: &ArrowResponse) -> bool {
    resp.headers
        .get("content-type")
        .is_some_and(|v| v.to_ascii_lowercase().starts_with("text/html"))
}

/// True when a path segment could climb out of the arrow's namespace once the
/// daemon (or anything between) decodes or normalises it. Checked on the path
/// only: `..` is ordinary in a query string.
fn has_dot_segment(path: &str) -> bool {
    path.contains('\\')
        || path.split('/').any(|segment| {
            let lower = segment.to_ascii_lowercase();
            segment == ".." || lower.contains("%2e") || lower.contains("%5c")
        })
}

fn forward(
    namespace: &str,
    uri: &Uri,
    req: ArrowRequest,
) -> ArrowRequest {
    let path_and_query = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let mut headers = req.headers;
    for name in STRIPPED {
        headers.remove(name);
    }
    headers.set("accept-encoding", "identity");
    ArrowRequest {
        method: req.method,
        uri: daemon_path(namespace, path_and_query),
        headers,
        body: req.body,
    }
}

/// Serves one `arrow-app` request. The upstream is resolved inside the
/// timeout (like `quiver://`), because resolving it takes a fair lock.
pub async fn handle_request<F>(
    upstream: F,
    hosts: &ArrowHosts,
    req: ArrowRequest,
) -> ArrowResponse
where
    F: Future<Output = Arc<dyn ArrowUpstream>>,
{
    let Ok(uri) = req.uri.parse::<Uri>() else {
        return plain(400, "bad arrow host");
    };
    let Some(host) = host_of(&uri) else {
        return plain(400, "bad arrow host");
    };
    let Some(namespace) = hosts.namespace_for(&host) else {
        return plain(404, "unknown arrow");
    };

    let path = uri.path().to_string();
    if has_dot_segment(&path) {
        return plain(400, "bad arrow path");
    }
    if path == SHIM_PATH {
        return shim_response(&req.method);
    }
    if path.starts_with("/__arrow/") {
        return plain(404, "not found");
    }

    let forwarded = forward(&namespace, &uri, req);
    let proxied = async move { upstream.await.request(forwarded).await };
    let mut resp = match tokio::time::timeout(ARROW_TIMEOUT, proxied).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => return plain(502, &format!("quiver proxy: {e}")),
        Err(_) => return plain(504, "the arrow did not answer in time"),
    };

    if resp.is_success() && is_html(&resp) {
        resp.headers.remove("content-length");
        resp.body = inject_shim(&resp.body);
    }
    harden(&mut resp);
    resp
}

#[cfg(test)]
#[path = "handler_tests.rs"]
mod tests;
