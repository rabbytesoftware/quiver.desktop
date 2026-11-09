use super::*;
use crate::transport::testing::drain_request;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn get(path: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method("GET")
        .uri(format!("quiver://localhost{path}"))
        .body(Vec::new())
        .unwrap()
}

/// A transport pointed at a port nothing listens on, so every request fails
/// at connect. Binding then dropping is what makes the port both free and
/// known-unused. (Moved here from the deleted remote/transport.rs.)
async fn unreachable() -> HttpTransport {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    HttpTransport::new(format!("tcp://{addr}"), Some("tok".into()))
}

/// Promises more body than it sends, then hangs up — the only way to reach
/// the body-read error arm. (Moved here from the deleted remote/transport.rs.)
///
/// The hang-up has to be the CLEAN one a truncating server performs, which
/// is why the request is read first: see `drain_request`. Closing on top of
/// an unread request resets the connection instead, and then this fixture
/// proves nothing about a short body — the client fails on the reset before
/// it can notice the body was short.
async fn truncates_its_body(status: u16) -> HttpTransport {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            drain_request(&mut sock).await;
            let _ = sock
                .write_all(
                    format!("HTTP/1.1 {status} X\r\nContent-Length: 64\r\n\r\nshort").as_bytes(),
                )
                .await;
            let _ = sock.shutdown().await;
        }
    });
    HttpTransport::new(format!("tcp://{addr}"), None)
}

/// A peer that answers every request with the same canned bytes and then
/// closes. It reads the request first — a server that closes over an unread
/// request closes ABORTIVELY, and on Windows that RST takes the response
/// with it; see `drain_request`.
async fn serving(raw: &'static str) -> HttpTransport {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            drain_request(&mut sock).await;
            let _ = sock.write_all(raw.as_bytes()).await;
            let _ = sock.shutdown().await;
        }
    });
    HttpTransport::new(format!("http://{addr}"), None)
}

#[test]
fn tcp_scheme_is_normalised_to_http() {
    let t = HttpTransport::new("tcp://10.0.0.1:40257", None);
    assert_eq!(t.url("/v0/health"), "http://10.0.0.1:40257/v0/health");
}

#[test]
fn https_base_is_left_alone() {
    let t = HttpTransport::new("https://core.example.com", None);
    assert_eq!(t.url("/v0/health"), "https://core.example.com/v0/health");
}

#[test]
fn ws_url_derives_wss_from_https() {
    let t = HttpTransport::new("https://core.example.com", None);
    assert_eq!(t.ws_url("/v0/arrow"), "wss://core.example.com/v0/arrow");
}

#[test]
fn ws_url_derives_ws_from_loopback_http() {
    let t = HttpTransport::new("tcp://127.0.0.1:5555", None);
    assert_eq!(t.ws_url("/v0/arrow"), "ws://127.0.0.1:5555/v0/arrow");
}

/// Local Windows has no token. A transport built without one must not send
/// an Authorization header at all — an empty Bearer would be worse than none.
#[test]
fn a_transport_without_a_token_sends_no_auth_header() {
    let t = HttpTransport::new("http://127.0.0.1:1", None);
    assert!(t.auth_header().is_none());
}

#[test]
fn a_transport_with_a_token_sends_a_bearer() {
    let t = HttpTransport::new("http://127.0.0.1:1", Some("abc".into()));
    assert_eq!(t.auth_header().as_deref(), Some("Bearer abc"));
}

mod requests;
mod streaming;
mod websocket;
