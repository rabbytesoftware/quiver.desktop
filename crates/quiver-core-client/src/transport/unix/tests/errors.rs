use super::*;

#[tokio::test]
async fn a_missing_socket_is_a_connect_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = UnixTransport::new("/tmp/qvx-definitely-not-here.sock");
    let err = t.request(get("/v0/health")).await.unwrap_err();
    assert!(matches!(err, Error::Connect(_)), "got {err:?}");
}

#[tokio::test]
async fn open_ws_on_a_missing_socket_is_a_connect_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = UnixTransport::new("/tmp/qvx-definitely-not-here.sock");
    // `.unwrap_err()` needs the `Ok` type to be `Debug`, and `WsStream`
    // isn't. `.err().unwrap()` drops the `Ok` value instead of formatting
    // it, sidestepping that bound.
    let err = t.open_ws("/v0/arrow").await.err().unwrap();
    assert!(matches!(err, Error::Connect(_)), "got {err:?}");
}

#[tokio::test]
async fn open_ws_completes_the_upgrade_against_a_real_peer() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("ws");
    let listener = UnixListener::bind(&sock).unwrap();
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let server = async {
        let (s, _) = listener.accept().await.unwrap();
        let _ = tokio_tungstenite::accept_async(s).await;
    };
    let (ws, _) = tokio::join!(t.open_ws("/v0/arrow"), server);
    assert!(ws.is_ok(), "upgrade must succeed");
    let _ = std::fs::remove_file(&sock);
}

// --- Coverage for branches the 5 tests above the brief specifies don't
// reach: the request-target fallback (which, in turn, surfaces the
// request-builder's own `map_err`), and every other reachable failure
// point downstream of a successful connect. `handshake`'s `map_err` and
// the response-builder's `map_err` are not exercised below because they
// have no black-box trigger — HTTP/1 handshake does no I/O, and the
// response builder only ever receives an already-valid `StatusCode`, so
// those two arms are unreachable short of injecting a broken stream.

/// A request target with no `path_and_query` (e.g. a CONNECT-style
/// authority-form URI) exercises the `unwrap_or_else` fallback to
/// `req.uri().path()` — which, for that URI form, is always `""`. That
/// value never reaches the peer: building the outgoing request with an
/// empty target fails immediately, before `send_request` is even called.
/// Standing up a real listener and observing zero bytes read (rather
/// than trusting `Err` alone) is what ties this assertion to the
/// fallback's actual output — an unrelated failure (e.g. a bad
/// `socket_path`) wouldn't get this far to write anything either, but it
/// also wouldn't leave a listener sitting there having accepted a
/// connection that then received nothing.
#[tokio::test]
async fn a_uri_without_a_path_and_query_fails_to_build_a_request() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("fallback");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let req = Request::builder()
        .method("GET")
        .uri("localhost:1234") // authority-form: no path_and_query
        .body(Vec::new())
        .unwrap();

    let capture = async {
        let (mut s, _) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 16];
        s.read(&mut buf).await.unwrap_or(0)
    };

    let (resp, bytes_received) = tokio::join!(t.request(req), capture);

    assert_eq!(
        bytes_received, 0,
        "the empty fallback target must never reach the wire"
    );
    let err = resp.unwrap_err();
    assert!(
        matches!(&err, Error::Protocol(msg) if msg.contains("empty")),
        "got {err:?}"
    );
    let _ = std::fs::remove_file(&sock);
}

/// A response cut short of its promised `Content-Length` is a protocol
/// error, not a panic or a silently truncated body.
#[tokio::test]
async fn a_truncated_body_is_a_protocol_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("trunc");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let (resp, _) = tokio::join!(
        t.request(get("/v0/health")),
        accept_and_reply(
            &listener,
            "HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\nshort"
        )
    );

    let err = resp.unwrap_err();
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
    let _ = std::fs::remove_file(&sock);
}

/// A path with a character that is invalid in a URI (a raw space) fails
/// to build a WebSocket client request rather than being silently
/// mangled into a different route.
#[tokio::test]
async fn an_invalid_ws_path_is_a_protocol_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("wsbadpath");
    let _listener = UnixListener::bind(&sock).unwrap();
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let err = t.open_ws("/v0/ar row").await.err().unwrap();
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
    let _ = std::fs::remove_file(&sock);
}

/// A peer that accepts and then closes the connection before completing
/// the WebSocket handshake fails the upgrade with a protocol error.
#[tokio::test]
async fn a_peer_that_closes_before_the_upgrade_is_a_protocol_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("wsclose");
    let listener = UnixListener::bind(&sock).unwrap();
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let server = async {
        let (s, _) = listener.accept().await.unwrap();
        drop(s);
    };
    let (ws, _) = tokio::join!(t.open_ws("/v0/arrow"), server);
    let err = ws.err().unwrap();
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
    let _ = std::fs::remove_file(&sock);
}

/// A peer that accepts and then closes the connection before reading the
/// request fails to send it with a protocol error, rather than hanging.
#[tokio::test]
async fn a_peer_that_closes_before_reading_is_a_protocol_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("closefast");
    let listener = UnixListener::bind(&sock).unwrap();
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let server = async {
        let (s, _) = listener.accept().await.unwrap();
        drop(s);
    };
    let (resp, _) = tokio::join!(t.request(get("/v0/health")), server);
    let err = resp.unwrap_err();
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
    let _ = std::fs::remove_file(&sock);
}
