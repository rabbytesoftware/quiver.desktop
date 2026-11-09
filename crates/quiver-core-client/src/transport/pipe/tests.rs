use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

fn test_name(tag: &str) -> String {
    format!("qvx-{}-{tag}", std::process::id())
}

fn serve(name: &str) -> NamedPipeServer {
    ServerOptions::new()
        .first_pipe_instance(true)
        .create(pipe_path(name))
        .expect("create pipe")
}

/// Answers one connection with one canned HTTP/1.1 response, after reading
/// the request (see `testing::drain_request` for why the order matters on
/// Windows). Hands the server back so the caller controls when it closes:
/// disconnecting early would discard the response before the client read it.
async fn accept_and_reply(server: NamedPipeServer, raw: &'static str) -> NamedPipeServer {
    server.connect().await.expect("connect");
    let mut server = server;
    let mut buf = [0u8; 1024];
    let _ = server.read(&mut buf).await;
    let _ = server.write_all(raw.as_bytes()).await;
    let _ = server.flush().await;
    server
}

fn get(path: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method("GET")
        .uri(format!("quiver://localhost{path}"))
        .body(Vec::new())
        .unwrap()
}

#[test]
fn pipe_path_prefixes_the_pipe_namespace() {
    assert_eq!(pipe_path("quiver"), r"\\.\pipe\quiver");
}

#[tokio::test]
async fn a_missing_pipe_is_a_connect_error() {
    let t = PipeTransport::new("qvx-definitely-not-here");
    let err = t.request(get("/v0/health")).await.unwrap_err();
    assert!(matches!(err, Error::Connect(_)), "got {err:?}");
}

#[tokio::test]
async fn open_ws_on_a_missing_pipe_is_a_connect_error() {
    let t = PipeTransport::new("qvx-definitely-not-here");
    let err = t.open_ws("/v0/arrow").await.err().unwrap();
    assert!(matches!(err, Error::Connect(_)), "got {err:?}");
}

#[tokio::test]
async fn forwards_a_request_and_returns_the_whole_response() {
    let name = test_name("ok");
    let server = serve(&name);
    let t = PipeTransport::new(&name);

    let (resp, _server) = tokio::join!(
        t.request(get("/v0/health")),
        accept_and_reply(
            server,
            "HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n{\"status\":\"ok\"}"
        )
    );

    let resp = resp.expect("request must succeed");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.body(), br#"{"status":"ok"}"#);
}

#[tokio::test]
async fn open_ws_completes_the_upgrade_against_a_real_peer() {
    let name = test_name("ws");
    let server = serve(&name);
    let t = PipeTransport::new(&name);

    let peer = async {
        server.connect().await.unwrap();
        let _ = tokio_tungstenite::accept_async(server).await;
    };
    let (ws, _) = tokio::join!(t.open_ws("/v0/arrow"), peer);
    assert!(ws.is_ok(), "upgrade must succeed");
}
