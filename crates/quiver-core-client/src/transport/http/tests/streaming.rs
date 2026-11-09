use super::*;

fn post(path: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method("POST")
        .uri(format!("quiver://localhost{path}"))
        .header("content-type", "application/json")
        .body(b"{}".to_vec())
        .unwrap()
}

/// A chunk written by the peer reaches the caller before the peer finishes.
#[tokio::test]
async fn request_stream_delivers_chunks_before_the_response_ends() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), Some("tok".into()));
    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel::<()>();

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 2048];
        let n = s.read(&mut buf).await.unwrap();
        let head = String::from_utf8_lossy(&buf[..n]).to_lowercase();
        assert!(head.contains("authorization: bearer tok"), "{head}");
        s.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n")
            .await
            .unwrap();
        ack_rx.await.unwrap();
        s.write_all(b"6\r\n world\r\n0\r\n\r\n").await.unwrap();
        let _ = s.shutdown().await;
    };
    let client = async {
        let mut resp = t
            .request_stream(post("/v0/console/exec"), Duration::from_secs(5))
            .await
            .expect("stream must open");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b"hello");
        ack_tx.send(()).unwrap();
        assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b" world");
        assert!(resp.chunks.recv().await.is_none());
    };
    tokio::join!(server, client);
}

/// The request carries its own deadline, not the client's: a stalled peer is
/// cut off at the deadline asked for (150ms), well inside the client-wide test
/// value (750ms) that would otherwise apply. The 600ms only bounds the wait, so
/// a deadline that was ignored fails the test instead of passing it slowly.
#[tokio::test]
async fn request_stream_ends_a_stalled_peer_at_the_requests_own_deadline() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);
    let (done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 2048];
        let _ = s.read(&mut buf).await.unwrap();
        s.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n")
            .await
            .unwrap();
        // Say nothing more: hold the connection until the client is done with it.
        let _ = done_rx.await;
    };
    let client = async {
        let resp = t
            .request_stream(post("/v0/console/exec"), Duration::from_millis(150))
            .await
            .unwrap();
        let items = tokio::time::timeout(Duration::from_millis(600), collect(resp))
            .await
            .expect("the request's own deadline must end the stream, not the client's");
        done_tx.send(()).unwrap();
        items
    };
    let (_, items) = tokio::join!(server, client);
    assert_eq!(items[0].as_ref().unwrap(), b"hello");
    assert!(
        matches!(items.last().unwrap(), Err(Error::Protocol(_))),
        "{items:?}"
    );
}

#[tokio::test]
async fn dropping_the_stream_closes_the_connection() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);
    let (first_tx, first_rx) = tokio::sync::oneshot::channel::<()>();

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 2048];
        let _ = s.read(&mut buf).await.unwrap();
        s.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n")
            .await
            .unwrap();
        first_tx.send(()).unwrap();
        let closed = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match s.read(&mut buf).await {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {}
                }
            }
        })
        .await;
        assert!(closed.is_ok(), "the daemon never saw the client hang up");
    };
    let client = async {
        let mut resp = t
            .request_stream(post("/v0/console/exec"), Duration::from_secs(5))
            .await
            .expect("stream must open");
        assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b"hello");
        first_rx.await.unwrap();
        drop(resp);
    };
    tokio::join!(server, client);
}

async fn collect(mut resp: StreamResponse) -> Vec<Result<Vec<u8>, Error>> {
    let mut items = Vec::new();
    while let Some(item) = resp.chunks.recv().await {
        items.push(item);
    }
    items
}

#[tokio::test]
async fn request_stream_to_a_dead_port_is_a_connect_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = unreachable().await;
    let err = t
        .request_stream(post("/v0/console/exec"), Duration::from_secs(1))
        .await
        .err()
        .unwrap();
    assert!(matches!(err, Error::Connect(_)), "got {err:?}");
}

/// A peer that accepts and hangs up before answering is a protocol failure, not a refused connection.
#[tokio::test]
async fn request_stream_to_a_peer_that_hangs_up_is_a_protocol_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        drain_request(&mut s).await;
        drop(s);
    };
    let client = t.request_stream(post("/v0/console/exec"), Duration::from_secs(2));
    let (_, err) = tokio::join!(server, client);
    assert!(matches!(err.err().unwrap(), Error::Protocol(_)));
}

/// The caller's own headers go through; the inbound Host (the local proxy's) does not.
#[tokio::test]
async fn request_stream_forwards_headers_but_not_the_inbound_host() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let n = s.read(&mut buf).await.unwrap();
        let head = String::from_utf8_lossy(&buf[..n]).to_lowercase();
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .await
            .unwrap();
        head
    };
    let req = Request::builder()
        .method("POST")
        .uri("quiver://localhost/v0/console/exec")
        .header("host", "the-local-proxy.invalid")
        .header("x-trace", "abc")
        .body(b"{}".to_vec())
        .unwrap();
    let (head, resp) = tokio::join!(server, t.request_stream(req, Duration::from_secs(2)));
    assert!(head.contains("x-trace: abc"), "{head}");
    assert!(!head.contains("the-local-proxy.invalid"), "{head}");
    assert_eq!(resp.unwrap().status, 200);
}

/// The daemon hanging up mid-body is reported after what arrived.
#[tokio::test]
async fn request_stream_reports_a_body_cut_short() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let t = HttpTransport::new(format!("http://{addr}"), None);

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        drain_request(&mut s).await;
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nhello")
            .await
            .unwrap();
        let _ = s.shutdown().await;
    };
    let client = async {
        let resp = t
            .request_stream(post("/v0/console/exec"), Duration::from_secs(2))
            .await
            .unwrap();
        collect(resp).await
    };
    let (_, items) = tokio::join!(server, client);
    assert!(
        matches!(items.last().unwrap(), Err(Error::Protocol(_))),
        "{items:?}"
    );
}
