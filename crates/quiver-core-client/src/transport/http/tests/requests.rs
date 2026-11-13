use super::*;

#[tokio::test]
async fn forwards_a_request_and_returns_the_whole_response() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = serving("HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}").await;
    let resp = t.request(get("/v0/health")).await.expect("must succeed");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.body(), br#"{"status":"ok"}"#);
}

#[tokio::test]
async fn a_daemon_error_status_is_a_response_not_an_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = serving("HTTP/1.1 500 Oops\r\nContent-Length: 2\r\n\r\n{}").await;
    assert_eq!(
        t.request(get("/v0/arrow")).await.expect("not Err").status(),
        500
    );
}

#[tokio::test]
async fn an_unreachable_host_is_a_connect_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let err = unreachable()
        .await
        .request(get("/v0/health"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Connect(_)), "got {err:?}");
}

#[tokio::test]
async fn a_truncated_body_is_a_protocol_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let err = truncates_its_body(200)
        .await
        .request(get("/v0/arrow"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
}

/// `send()` spans both the connect phase and everything after a
/// connection is established. A peer that accepts the TCP connection and
/// then closes before sending a byte back must surface as `Protocol`, not
/// `Connect` — `reqwest::Error::is_connect()` is `false` for this case,
/// matching unix.rs's identical distinction between `UnixStream::connect`
/// (Connect) and `send_request` (Protocol), and its equivalent test
/// (`a_peer_that_closes_before_reading_is_a_protocol_error`).
#[tokio::test]
async fn a_peer_that_accepts_then_closes_before_responding_is_a_protocol_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);

    let server = async {
        let (s, _) = listener.accept().await.unwrap();
        drop(s);
    };
    let (resp, _) = tokio::join!(t.request(get("/v0/health")), server);
    let err = resp.unwrap_err();
    assert!(matches!(err, Error::Protocol(_)), "got {err:?}");
}

#[tokio::test]
async fn open_ws_against_a_dead_port_is_a_connect_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    // `.unwrap_err()` needs `Ok`'s type to be `Debug`, and `WsStream`
    // isn't. `.err().unwrap()` drops the `Ok` value instead of formatting
    // it, sidestepping that bound — same fix as unix.rs's equivalent test.
    let err = unreachable()
        .await
        .open_ws("/v0/arrow")
        .await
        .err()
        .unwrap();
    assert!(matches!(err, Error::Connect(_)), "got {err:?}");
}
