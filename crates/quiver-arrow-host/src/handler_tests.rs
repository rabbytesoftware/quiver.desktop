use std::future::ready;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::*;
use crate::csp::ARROW_CSP;
use crate::upstream::WsRelay;
use crate::{Error, Result};

/// Answers every request with `reply` and remembers what it was asked.
struct Stub {
    reply: ArrowResponse,
    seen: Mutex<Vec<ArrowRequest>>,
}

impl Stub {
    fn new(reply: ArrowResponse) -> Arc<Self> {
        Arc::new(Self {
            reply,
            seen: Mutex::new(Vec::new()),
        })
    }
    fn last(&self) -> ArrowRequest {
        self.seen
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("a request")
    }
    fn calls(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
}

#[async_trait]
impl ArrowUpstream for Stub {
    async fn request(&self, req: ArrowRequest) -> Result<ArrowResponse> {
        self.seen.lock().unwrap().push(req);
        Ok(self.reply.clone())
    }
    async fn open_ws(&self, _: &str) -> Result<Box<dyn WsRelay>> {
        Err(Error::Upstream("not used".into()))
    }
}

/// Must never be reached: a request that gets here escaped a check.
struct Dead;

#[async_trait]
impl ArrowUpstream for Dead {
    async fn request(&self, _: ArrowRequest) -> Result<ArrowResponse> {
        panic!("upstream must not be used")
    }
    async fn open_ws(&self, _: &str) -> Result<Box<dyn WsRelay>> {
        panic!("upstream must not be used")
    }
}

struct Hangs;

#[async_trait]
impl ArrowUpstream for Hangs {
    async fn request(&self, _: ArrowRequest) -> Result<ArrowResponse> {
        std::future::pending().await
    }
    async fn open_ws(&self, _: &str) -> Result<Box<dyn WsRelay>> {
        Err(Error::Upstream("not used".into()))
    }
}

struct Refuses;

#[async_trait]
impl ArrowUpstream for Refuses {
    async fn request(&self, _: ArrowRequest) -> Result<ArrowResponse> {
        Err(Error::Upstream("refused".into()))
    }
    async fn open_ws(&self, _: &str) -> Result<Box<dyn WsRelay>> {
        Err(Error::Upstream("not used".into()))
    }
}

fn ok(body: &str, content_type: &str) -> ArrowResponse {
    let mut resp = ArrowResponse::new(200, body.as_bytes().to_vec());
    resp.headers.append("content-type", content_type);
    resp.headers
        .append("content-length", body.len().to_string());
    resp.headers.append("access-control-allow-origin", "*");
    resp.headers.append("x-frame-options", "DENY");
    resp
}

fn request(uri: &str) -> ArrowRequest {
    ArrowRequest::get(uri)
}

fn setup() -> (ArrowHosts, String) {
    let hosts = ArrowHosts::new();
    let host = hosts.register("github.com/user/chat");
    (hosts, host)
}

async fn run(
    upstream: Arc<dyn ArrowUpstream>,
    hosts: &ArrowHosts,
    req: ArrowRequest,
) -> ArrowResponse {
    handle_request(ready(upstream), hosts, req).await
}

#[tokio::test]
async fn unknown_host_is_404_and_never_reaches_the_upstream() {
    let (hosts, _) = setup();
    let resp = run(Arc::new(Dead), &hosts, request("arrow-app://deadbeef/")).await;
    assert_eq!(resp.status, 404);
    assert!(resp.headers.contains("content-security-policy"));
}

#[tokio::test]
async fn forwards_to_the_daemon_ui_route_with_path_and_query() {
    let (hosts, host) = setup();
    let stub = Stub::new(ok("x", "text/javascript"));
    run(
        stub.clone(),
        &hosts,
        request(&format!("arrow-app://{host}/assets/app.js?v=1")),
    )
    .await;
    assert_eq!(
        stub.last().uri,
        "/v0/ui/github.com%2Fuser%2Fchat/assets/app.js?v=1"
    );
}

#[tokio::test]
async fn windows_style_host_resolves() {
    let (hosts, host) = setup();
    let stub = Stub::new(ok("x", "text/plain"));
    let resp = run(
        stub.clone(),
        &hosts,
        request(&format!("http://arrow-app.{host}/a")),
    )
    .await;
    assert_eq!(resp.status, 200);
    assert_eq!(stub.calls(), 1);
}

/// The exact form wry produces on Windows for a registered host resolves,
/// and the bare label (the old, non-secure-context form) no longer does.
#[tokio::test]
async fn windows_localhost_form_resolves_and_the_bare_label_does_not() {
    let (hosts, _) = setup();
    let label = crate::hosts::host_label("github.com/user/chat");
    let stub = Stub::new(ok("x", "text/plain"));
    let resp = run(
        stub.clone(),
        &hosts,
        request(&format!("http://arrow-app.{label}.localhost/x")),
    )
    .await;
    assert_eq!(resp.status, 200);
    assert_eq!(stub.calls(), 1);

    for bare in [
        format!("http://arrow-app.{label}/x"),
        format!("arrow-app://{label}/x"),
    ] {
        let resp = run(Arc::new(Dead), &hosts, request(&bare)).await;
        assert_eq!(resp.status, 404, "{bare}");
    }
}

/// The page must not be able to ride the user's credentials or leak its
/// origin to the daemon, and bodies must arrive uncompressed so the shim
/// can be injected.
#[tokio::test]
async fn strips_credentials_and_forces_identity_encoding() {
    let (hosts, host) = setup();
    let stub = Stub::new(ok("x", "text/plain"));
    let mut req = request(&format!("arrow-app://{host}/"));
    for (k, v) in [
        ("Origin", "arrow-app://x"),
        ("Referer", "arrow-app://x/"),
        ("Cookie", "a=b"),
        ("Authorization", "Bearer evil"),
        ("Host", "evil"),
        ("Accept-Encoding", "gzip, br"),
        ("x-custom", "kept"),
    ] {
        req.headers.append(k, v);
    }
    run(stub.clone(), &hosts, req).await;

    let seen = stub.last();
    let h = &seen.headers;
    for gone in ["origin", "referer", "cookie", "authorization", "host"] {
        assert!(h.get(gone).is_none(), "{gone}");
    }
    assert_eq!(h.get("accept-encoding"), Some("identity"));
    assert_eq!(h.get("x-custom"), Some("kept"));
}

#[tokio::test]
async fn html_gets_the_shim_and_a_fresh_length() {
    let (hosts, host) = setup();
    let html = "<html><head></head><body>hi</body></html>";
    let stub = Stub::new(ok(html, "text/html; charset=utf-8"));
    let resp = run(stub, &hosts, request(&format!("arrow-app://{host}/"))).await;

    let body = String::from_utf8(resp.body.clone()).unwrap();
    assert!(body.contains("<head><script src=\"/__arrow/shim.js\"></script>"));
    assert!(resp.headers.get("content-length").is_none());
}

#[tokio::test]
async fn non_html_is_left_alone_and_error_pages_are_not_injected() {
    let (hosts, host) = setup();
    let js = Stub::new(ok("var a;", "text/javascript"));
    let resp = run(js, &hosts, request(&format!("arrow-app://{host}/a.js"))).await;
    assert_eq!(resp.body.as_slice(), b"var a;");

    let mut not_found = ok("<head>nope", "text/html");
    not_found.status = 404;
    let resp = run(
        Stub::new(not_found),
        &hosts,
        request(&format!("arrow-app://{host}/x")),
    )
    .await;
    assert_eq!(resp.body.as_slice(), b"<head>nope");
}

#[tokio::test]
async fn every_response_is_hardened() {
    let (hosts, host) = setup();
    let resp = run(
        Stub::new(ok("x", "text/plain")),
        &hosts,
        request(&format!("arrow-app://{host}/")),
    )
    .await;
    let h = &resp.headers;
    assert_eq!(h.get("content-security-policy"), Some(ARROW_CSP));
    assert!(h.get("access-control-allow-origin").is_none());
    assert!(h.get("x-frame-options").is_none());
}

#[tokio::test]
async fn the_shim_is_served_locally_and_never_forwarded() {
    let (hosts, host) = setup();
    let resp = run(
        Arc::new(Dead),
        &hosts,
        request(&format!("arrow-app://{host}/__arrow/shim.js")),
    )
    .await;
    assert_eq!(resp.status, 200);
    assert_eq!(resp.headers.get("content-type"), Some("text/javascript"));
    assert_eq!(resp.body.as_slice(), SHIM_JS.as_bytes());

    let mut head = request(&format!("arrow-app://{host}/__arrow/shim.js"));
    head.method = "HEAD".into();
    assert!(run(Arc::new(Dead), &hosts, head).await.body.is_empty());

    let other = run(
        Arc::new(Dead),
        &hosts,
        request(&format!("arrow-app://{host}/__arrow/other")),
    )
    .await;
    assert_eq!(other.status, 404);
}

#[tokio::test]
async fn upstream_failures_become_502_and_504_with_a_csp() {
    let (hosts, host) = setup();
    let refused = run(
        Arc::new(Refuses),
        &hosts,
        request(&format!("arrow-app://{host}/")),
    )
    .await;
    assert_eq!(refused.status, 502);
    assert!(refused.headers.contains("content-security-policy"));

    let slow = run(
        Arc::new(Hangs),
        &hosts,
        request(&format!("arrow-app://{host}/")),
    )
    .await;
    assert_eq!(slow.status, 504);
}

#[tokio::test]
async fn dot_segments_in_the_path_are_400_and_never_reach_the_upstream() {
    let (hosts, host) = setup();
    for path in [
        "/%2e%2e/x",
        "/a/%2E%2E/b",
        "/a/..%5cb",
        "/a/%5Cb",
        "/a/b%2fc/%2e",
        "/a/../b",
        "/..",
        "/a\\b",
    ] {
        let resp = run(
            Arc::new(Dead),
            &hosts,
            request(&format!("arrow-app://{host}{path}")),
        )
        .await;
        assert_eq!(resp.status, 400, "{path}");
        assert!(resp.headers.contains("content-security-policy"), "{path}");
    }
}

#[tokio::test]
async fn dots_in_the_query_or_a_segment_name_are_not_dot_segments() {
    let (hosts, host) = setup();
    let stub = Stub::new(ok("x", "text/plain"));
    let resp = run(
        stub.clone(),
        &hosts,
        request(&format!("arrow-app://{host}/a..b/c?x=a..b")),
    )
    .await;
    assert_eq!(resp.status, 200);
    assert_eq!(stub.calls(), 1);
}
