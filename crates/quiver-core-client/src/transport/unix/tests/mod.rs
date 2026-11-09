use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;

/// Short path: sun_path is capped at 104 bytes and macOS $TMPDIR is long.
fn test_socket(tag: &str) -> std::path::PathBuf {
    let p = std::path::PathBuf::from(format!("/tmp/qvx-{}-{tag}.sock", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

/// Binds the listener; `accept_and_reply` does the writing.
fn serve(sock: &std::path::Path) -> UnixListener {
    UnixListener::bind(sock).expect("bind")
}

/// Answers every connection with one canned HTTP/1.1 response, after reading
/// the request like a real server does (see `testing::drain_request`: closing
/// over an unread request is an abortive close, and the client can then fail
/// writing its body before it ever reads the answer).
async fn accept_and_reply(listener: &UnixListener, raw: &'static str) {
    if let Ok((mut s, _)) = listener.accept().await {
        crate::transport::testing::drain_request(&mut s).await;
        let _ = s.write_all(raw.as_bytes()).await;
        let _ = s.shutdown().await;
    }
}

fn get(path: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method("GET")
        .uri(format!("quiver://localhost{path}"))
        .body(Vec::new())
        .unwrap()
}

#[tokio::test]
async fn forwards_a_request_and_returns_the_whole_response() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("ok");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let (resp, _) = tokio::join!(
        t.request(get("/v0/health")),
        accept_and_reply(
            &listener,
            "HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}"
        )
    );

    let resp = resp.expect("request must succeed");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.body(), br#"{"status":"ok"}"#);
    let _ = std::fs::remove_file(&sock);
}

/// A 4xx/5xx is the daemon ANSWERING. It must come back as a Response, not
/// an Err — the proxy relays it verbatim so the frontend can tell a 404
/// (meaningful) from a dead socket (retryable).
#[tokio::test]
async fn a_daemon_error_status_is_a_response_not_an_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("404");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let (resp, _) = tokio::join!(
        t.request(get("/v0/arrow/nope")),
        accept_and_reply(
            &listener,
            "HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\n\r\n{}"
        )
    );

    assert_eq!(resp.expect("must not be an Err").status(), 404);
    let _ = std::fs::remove_file(&sock);
}

mod errors;
mod streaming;
