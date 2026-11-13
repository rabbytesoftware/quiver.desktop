use super::*;

#[tokio::test]
async fn open_ws_completes_the_upgrade_against_a_real_peer() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);
    let server = async {
        let (s, _) = listener.accept().await.unwrap();
        let _ = tokio_tungstenite::accept_async(s).await;
    };
    let (ws, _) = tokio::join!(t.open_ws("/v0/arrow"), server);
    assert!(ws.is_ok(), "upgrade must succeed");
}

// --- Coverage for branches the 12 tests above (the brief's own set)
// don't reach. Each of the following asserts something none of the tests
// above already prove. Two branches are deliberately left untested
// because the public API has no way to reach them: the `ok_or_else`
// "ws url has no authority" arm in `open_ws` (see the comment at its call
// site — `into_client_request` already rejects a missing authority before
// we get there), and the response-builder's `headers_mut`/`body` failure
// arms in `request()` (the status always comes from an already-valid
// `reqwest::StatusCode`, so the builder never enters an error state).
// These mirror the unreachable arms unix.rs documents rather than tests.

/// An authority-form URI (no `path_and_query`) drives `request()` down the
/// `unwrap_or_else` fallback to `req.uri().path()`, which for that URI form
/// is always `""` — so the request targets the bare base URL. Unlike
/// unix.rs's hyper-based transport (where an empty request target fails to
/// build), reqwest happily sends that as a request for `/`. Capturing the
/// actual request line is what proves the fallback ran, rather than merely
/// that some request succeeded.
#[tokio::test]
async fn a_uri_without_a_path_and_query_falls_back_to_the_bare_base_url() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);

    let req = Request::builder()
        .method("GET")
        .uri("localhost:1234") // authority-form: no path_and_query
        .body(Vec::new())
        .unwrap();

    let capture = async {
        let (mut sock, _) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 256];
        let n = sock.read(&mut buf).await.unwrap_or(0);
        let raw = String::from_utf8_lossy(&buf[..n]).to_string();
        let _ = sock
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .await;
        let _ = sock.shutdown().await;
        raw
    };

    let (resp, request_line) = tokio::join!(t.request(req), capture);
    assert!(
        resp.is_ok(),
        "fallback target must still be a valid request"
    );
    assert!(
        request_line.starts_with("GET / HTTP"),
        "expected the bare base URL (root path), got {request_line:?}"
    );
}

/// `request()` must forward both an arbitrary caller-set header and the
/// Authorization bearer onto the wire — two lines (the header-copy loop,
/// and the `auth_header` branch inside `request()`) that none of the
/// tests above exercise, since `get()` sets no extra headers and every
/// prior `.request()` call used a tokenless transport.
#[tokio::test]
async fn a_request_forwards_custom_headers_and_the_bearer_token() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), Some("s3cr3t".into()));

    let req = Request::builder()
        .method("GET")
        .uri("quiver://localhost/v0/health")
        .header("x-quiver-test", "sentinel")
        .body(Vec::new())
        .unwrap();

    let capture = async {
        let (mut sock, _) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 512];
        let n = sock.read(&mut buf).await.unwrap_or(0);
        let raw = String::from_utf8_lossy(&buf[..n]).to_string();
        let _ = sock
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .await;
        let _ = sock.shutdown().await;
        raw
    };

    let (resp, raw) = tokio::join!(t.request(req), capture);
    assert!(resp.is_ok());
    let lower = raw.to_lowercase();
    assert!(lower.contains("x-quiver-test: sentinel"), "got {raw:?}");
    assert!(
        lower.contains("authorization: bearer s3cr3t"),
        "got {raw:?}"
    );
}

/// The inbound `Host` header (as set by callers dialing through the
/// `quiver://localhost` URI scheme) names the local proxy, not the remote
/// peer this transport actually talks to. Forwarding it verbatim would
/// silently address the request to the wrong virtual host on any
/// vhosted/proxied deployment — and reqwest will not override an
/// explicitly-set Host — so `request()` must drop it and let reqwest
/// derive the correct one from the target URL instead.
#[tokio::test]
async fn an_inbound_host_header_is_not_forwarded_to_the_remote_peer() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);

    let req = Request::builder()
        .method("GET")
        .uri("quiver://localhost/v0/health")
        .header("host", "localhost")
        .body(Vec::new())
        .unwrap();

    let capture = async {
        let (mut sock, _) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 512];
        let n = sock.read(&mut buf).await.unwrap_or(0);
        let raw = String::from_utf8_lossy(&buf[..n]).to_string();
        let _ = sock
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .await;
        let _ = sock.shutdown().await;
        raw
    };

    let (resp, raw) = tokio::join!(t.request(req), capture);
    assert!(resp.is_ok());
    let lower = raw.to_lowercase();
    assert!(
        lower.contains(format!("host: {addr}").to_lowercase().as_str()),
        "expected Host to be the peer's own authority, got {raw:?}"
    );
    assert!(
        !lower.contains("host: localhost"),
        "inbound Host header must not be forwarded verbatim, got {raw:?}"
    );
}

/// A path with a raw space is not a valid URI. `into_client_request`'s own
/// parse fails, which must surface as `Protocol`, not panic or hang.
#[tokio::test]
async fn an_invalid_ws_path_is_a_protocol_error() {
    let t = HttpTransport::new("http://127.0.0.1:1", None);
    let err = t.open_ws("/v0/ar row").await.err().unwrap();
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
}

/// A token containing a byte that is invalid in a header value (here, a
/// newline) fails `auth.parse()` inside `open_ws` — a distinct guard, on a
/// distinct input, from the path-parse failure above.
#[tokio::test]
async fn open_ws_with_an_invalid_token_is_a_protocol_error() {
    let t = HttpTransport::new("http://127.0.0.1:1", Some("bad\ntoken".into()));
    let err = t.open_ws("/v0/arrow").await.err().unwrap();
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
}

// ── Request timeouts (finding B) ─────────────────────────────────────────

/// `reqwest::Client::new()` sets no timeouts at all, so a peer that accepts
/// a connection and then never answers pinned the calling task — and its
/// socket — for the life of the app. `RemoteConnection::start` is one such
/// caller, and it used to run under `ConnectionManager`'s write lock.
///
/// Without `REQUEST_TIMEOUT` this test does not fail, it HANGS, so the
/// assertion is wrapped in a wait an order of magnitude longer than the
/// (cfg(test)-shortened) bound it is checking.
#[tokio::test]
async fn a_peer_that_accepts_and_then_says_nothing_ends_at_the_request_timeout() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);

    // Accept and hold: the connection stays open, and nothing is ever
    // written back.
    let held = tokio::spawn(async move {
        let (sock, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
        drop(sock);
    });

    let err = tokio::time::timeout(Duration::from_secs(10), t.request(get("/v0/health")))
        .await
        .expect(
            "a request must be bounded: an unbounded one holds its socket and its \
				 caller for the life of the process",
        )
        .expect_err("a stalled peer is not a successful response");
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");

    held.abort();
}

/// The whole-request bound has to stay ABOVE the proxy's own 300s ceiling,
/// or it silently truncates every long proxied request to whatever it is —
/// the proxy would never get to return its own marked 504, and the frontend
/// would see an unmarked failure it does not retry. Checked on the real
/// constants, not the shortened test ones.
#[test]
fn the_request_timeout_does_not_undercut_the_proxys_own_ceiling() {
    // Re-stated rather than imported: `proxy::PROXY_TIMEOUT` is shortened
    // under cfg(test) too, so importing it would compare two test values
    // and prove nothing about what ships.
    const PROXY_CEILING: Duration = Duration::from_secs(300);

    assert!(
        SHIPPED_REQUEST_TIMEOUT > PROXY_CEILING,
        "a request timeout at or below the proxy's 300s ceiling makes the proxy's \
			 own timeout unreachable: every long request would fail here instead, \
			 unmarked, and the frontend does not retry an unmarked failure"
    );
    assert!(
        SHIPPED_CONNECT_TIMEOUT < SHIPPED_REQUEST_TIMEOUT,
        "the connect phase happens inside the request, so its bound must be tighter"
    );
    // The pair the client is actually built with keeps the same ordering,
    // whichever pair cfg picks — otherwise a test could not tell which of
    // the two fired.
    assert!(CONNECT_TIMEOUT < REQUEST_TIMEOUT);
}

// ── WebSocket address resolution and TLS (finding E) ─────────────────────

/// The defect: `TcpStream::connect(authority)` needs a `:port`, and a
/// remote configured as `https://core.example.com` has none — so `wss://`
/// failed at ADDRESS PARSING, before a packet was sent, and never reached
/// the TLS it also was not doing.
#[test]
fn a_ws_url_without_a_port_defaults_to_the_schemes_own() {
    let target = |u: &str| ws_target(&u.parse::<http::Uri>().unwrap()).unwrap();

    assert_eq!(
        target("wss://core.example.com/v0/arrow"),
        ("core.example.com".to_string(), 443),
        "wss with no port is 443"
    );
    assert_eq!(
        target("ws://core.example.com/v0/arrow"),
        ("core.example.com".to_string(), 80),
        "ws with no port is 80"
    );
}

/// An explicit port always wins over the scheme's default — the local
/// Windows daemon is `ws://127.0.0.1:40257`, and sending it to :80 would
/// break the one path that worked before.
#[test]
fn an_explicit_port_is_kept() {
    let target = |u: &str| ws_target(&u.parse::<http::Uri>().unwrap()).unwrap();

    assert_eq!(
        target("ws://127.0.0.1:40257/v0/arrow"),
        ("127.0.0.1".to_string(), 40257)
    );
    assert_eq!(
        target("wss://core.example.com:8443/v0/arrow"),
        ("core.example.com".to_string(), 8443)
    );
}

/// A URI's IPv6 host arrives bracketed — that is URI syntax, not part of
/// the address, and `("[::1]", 443)` does not resolve.
#[test]
fn an_ipv6_host_loses_its_uri_brackets() {
    let target = |u: &str| ws_target(&u.parse::<http::Uri>().unwrap()).unwrap();

    assert_eq!(
        target("ws://[::1]:9000/v0/arrow"),
        ("::1".to_string(), 9000)
    );
    assert_eq!(target("wss://[::1]/v0/arrow"), ("::1".to_string(), 443));
}

/// The other half of finding E: `client_async` performed no TLS handshake,
/// so a `wss://` dial spoke PLAINTEXT WebSocket at what was supposed to be
/// a TLS listener. Against a plaintext WS server that upgrade SUCCEEDS,
/// which is precisely the silent failure — the app would have believed it
/// had an encrypted stream.
///
/// Here the peer is a real, plaintext WebSocket server and the transport is
/// `https://`, so `ws_url` produces `wss://`. A transport that actually
/// negotiates TLS cannot complete this handshake; the pre-fix one completed
/// it happily. Revert `client_async_tls_with_config` to `client_async` and
/// this goes red.
#[tokio::test]
async fn a_wss_dial_refuses_to_complete_against_a_plaintext_peer() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    // `https://` so `ws_url` derives `wss://` — the same derivation
    // `ws_url_derives_wss_from_https` pins above.
    let t = HttpTransport::new(format!("https://{addr}"), None);

    let server = async {
        if let Ok((s, _)) = listener.accept().await {
            // A plaintext WebSocket server: it speaks HTTP, not TLS.
            let _ = tokio_tungstenite::accept_async(s).await;
        }
    };

    let (ws, _) = tokio::join!(t.open_ws("/v0/arrow"), server);
    let err = ws.err().expect(
        "a wss:// dial must not complete against a peer that never negotiated TLS: \
			 completing it is the app believing an unencrypted stream is encrypted",
    );
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
}

/// `open_ws` inserts its own Authorization header — a separate code path
/// from `request()`'s — so a passing `request()` token test does not prove
/// this one works. The peer captures the raw handshake bytes and then
/// hangs up before answering, which both proves the header was sent and
/// exercises the upgrade failing against a peer that never completes it.
#[tokio::test]
async fn open_ws_sends_a_bearer_header_before_the_peer_hangs_up() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), Some("tok123".into()));

    let capture = async {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 512];
        let n = sock.read(&mut buf).await.unwrap_or(0);
        String::from_utf8_lossy(&buf[..n]).to_string()
    };

    let (ws, raw) = tokio::join!(t.open_ws("/v0/arrow"), capture);
    assert!(
        raw.to_lowercase().contains("authorization: bearer tok123"),
        "expected bearer header in handshake, got {raw:?}"
    );
    let err = ws.err().unwrap();
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
}
