use super::*;

fn post(
    path: &str,
    body: &str,
) -> Request<Vec<u8>> {
    Request::builder()
        .method("POST")
        .uri(format!("quiver://localhost{path}"))
        .header("content-type", "application/json")
        .body(body.as_bytes().to_vec())
        .unwrap()
}

/// The point of `request_stream`: a chunk the daemon has written reaches the
/// caller BEFORE the daemon finishes the response. A transport that collected
/// the body first would deadlock here, since the server only finishes once
/// the client has acknowledged the first chunk.
#[tokio::test]
async fn request_stream_delivers_chunks_before_the_response_ends() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("stream");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());
    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel::<()>();

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf).await.unwrap();
        s.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n")
            .await
            .unwrap();
        ack_rx.await.unwrap();
        s.write_all(b"6\r\n world\r\n0\r\n\r\n").await.unwrap();
        let _ = s.shutdown().await;
    };
    let client = async {
        let mut resp = t
            .request_stream(post("/v0/console/exec", "{}"), Duration::from_secs(5))
            .await
            .expect("stream must open");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b"hello");
        ack_tx.send(()).unwrap();
        assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b" world");
        assert!(resp.chunks.recv().await.is_none(), "stream must end");
    };
    tokio::join!(server, client);
    let _ = std::fs::remove_file(&sock);
}

/// Dropping the receiver abandons the request: the daemon must see the
/// connection close, because that is what cancels the command it is running.
#[tokio::test]
async fn dropping_the_stream_closes_the_connection() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("stream-drop");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());
    let (first_tx, first_rx) = tokio::sync::oneshot::channel::<()>();

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf).await.unwrap();
        s.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n")
            .await
            .unwrap();
        first_tx.send(()).unwrap();
        // The client hanging up shows as EOF (or a reset) on read.
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
            .request_stream(post("/v0/console/exec", "{}"), Duration::from_secs(5))
            .await
            .expect("stream must open");
        assert_eq!(resp.chunks.recv().await.unwrap().unwrap(), b"hello");
        first_rx.await.unwrap();
        drop(resp);
    };
    tokio::join!(server, client);
    let _ = std::fs::remove_file(&sock);
}

async fn collect(mut resp: StreamResponse) -> Vec<Result<Vec<u8>, Error>> {
    let mut items = Vec::new();
    while let Some(item) = resp.chunks.recv().await {
        items.push(item);
    }
    items
}

/// The daemon hanging up mid-body is an error the consumer must hear about, after
/// what did arrive -- not a quiet end that reads as a finished command.
#[tokio::test]
async fn a_body_cut_short_is_reported_after_what_arrived() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("stream-cut");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf).await.unwrap();
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nhello")
            .await
            .unwrap();
        let _ = s.shutdown().await;
    };
    let client = async {
        let resp = t
            .request_stream(post("/v0/console/exec", "{}"), Duration::from_secs(5))
            .await
            .unwrap();
        collect(resp).await
    };
    let (_, items) = tokio::join!(server, client);
    assert_eq!(items.first().unwrap().as_ref().unwrap(), b"hello");
    assert!(
        matches!(items.last().unwrap(), Err(Error::Protocol(_))),
        "{items:?}"
    );
    let _ = std::fs::remove_file(&sock);
}

/// A command that goes quiet for longer than its deadline ends the stream with a
/// timeout error rather than holding the connection for ever.
#[tokio::test]
async fn a_body_that_stalls_past_the_deadline_ends_in_a_timeout() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("stream-stall");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf).await.unwrap();
        s.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n")
            .await
            .unwrap();
        // Say nothing more until the client has given up.
        let _ = tokio::time::timeout(Duration::from_secs(3), s.read(&mut buf)).await;
    };
    let client = async {
        let resp = t
            .request_stream(post("/v0/console/exec", "{}"), Duration::from_millis(300))
            .await
            .unwrap();
        collect(resp).await
    };
    let (_, items) = tokio::join!(server, client);
    assert_eq!(items[0].as_ref().unwrap(), b"hello");
    match items.last().unwrap() {
        Err(Error::Protocol(why)) => {
            assert!(why.contains("timed out"), "{why}")
        }
        other => panic!("expected a timeout, got {other:?}"),
    }
    let _ = std::fs::remove_file(&sock);
}

/// A daemon that accepts the request and never answers is cut off at the deadline too.
#[tokio::test]
async fn a_daemon_that_never_answers_is_a_timeout_before_the_status() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("stream-silent");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf).await.unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(3), s.read(&mut buf)).await;
    };
    let client = async {
        t.request_stream(post("/v0/console/exec", "{}"), Duration::from_millis(200))
            .await
            .err()
            .unwrap()
    };
    let (_, err) = tokio::join!(server, client);
    match err {
        Error::Protocol(why) => {
            assert!(why.contains("timed out"), "{why}")
        }
        other => panic!("expected a timeout, got {other:?}"),
    }
    let _ = std::fs::remove_file(&sock);
}

/// Trailers carry nothing this app reads: they are skipped, and the body still ends cleanly.
#[tokio::test]
async fn trailers_are_skipped_and_the_body_still_ends_cleanly() {
    let _serialised = crate::FD_TESTS.lock().await;

    let sock = test_socket("stream-trailers");
    let listener = serve(&sock);
    let t = UnixTransport::new(sock.to_string_lossy().to_string());

    let server = async {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf).await.unwrap();
        s.write_all(
				b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: X-Done\r\n\r\n5\r\nhello\r\n0\r\nX-Done: yes\r\n\r\n",
			)
			.await
			.unwrap();
        let _ = s.shutdown().await;
    };
    let client = async {
        let resp = t
            .request_stream(post("/v0/console/exec", "{}"), Duration::from_secs(5))
            .await
            .unwrap();
        collect(resp).await
    };
    let (_, items) = tokio::join!(server, client);
    let ok: Vec<_> = items.iter().filter_map(|i| i.as_ref().ok()).collect();
    assert_eq!(ok, vec![&b"hello".to_vec()]);
    assert!(items.iter().all(|i| i.is_ok()), "{items:?}");
    let _ = std::fs::remove_file(&sock);
}

#[tokio::test]
async fn request_stream_on_a_missing_socket_is_a_connect_error() {
    let _serialised = crate::FD_TESTS.lock().await;

    let t = UnixTransport::new("/tmp/qvx-definitely-not-here.sock");
    let err = t
        .request_stream(get("/v0/health"), Duration::from_secs(1))
        .await
        .err()
        .unwrap();
    assert!(matches!(err, Error::Connect(_)), "got {err:?}");
}
