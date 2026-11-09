use super::*;

/// The bridge's actual job. Frames are forwarded RAW because the frontend
/// parses the whole DTO itself, so a sink that sees anything but the exact
/// bytes the daemon sent is a broken bridge. A non-text frame is not a DTO:
/// it must be skipped, and must NOT be mistaken for the stream ending.
#[tokio::test]
async fn text_frames_reach_the_sink_verbatim_and_other_frames_are_skipped() {
    let _serialised = crate::FD_TESTS.lock().await;

    const FRAME: &str = r#"{"id":"a","name":"arrow"}"#;

    let (listener, transport) = listening().await;
    let manager = WsBridgeManager::new();

    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&seen);
    let (closed_tx, closed_rx) = oneshot::channel();
    let closed_tx = Mutex::new(Some(closed_tx));
    let sink = ClosureSink(move |frame: String| {
        recorded.lock().unwrap().push(frame.clone());
        if frame == WS_CLOSE_SENTINEL {
            if let Some(tx) = closed_tx.lock().unwrap().take() {
                let _ = tx.send(());
            }
        }
    });

    let peer = async {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("upgrade");
        ws.send(Message::Binary(vec![0xde, 0xad]))
            .await
            .expect("send binary");
        ws.send(Message::Text(FRAME.to_string()))
            .await
            .expect("send text");
        ws.close(None).await.expect("close");
    };

    let opened = open_bridge(
        &transport,
        "c1".to_string(),
        "/v0/x".to_string(),
        sink,
        &manager,
    );
    let (opened, _) = tokio::join!(opened, peer);
    opened.unwrap();

    closed_rx
        .await
        .expect("the daemon closed, so the sentinel must arrive");

    assert_eq!(
        *seen.lock().unwrap(),
        vec![FRAME.to_string(), WS_CLOSE_SENTINEL.to_string()],
        "the sink must see the text frame byte-for-byte and nothing else before \
			 the sentinel: a skipped frame that reached it would be parsed as a DTO, \
			 and one that ended the stream would trigger a needless reconnect"
    );
}

/// The webview → daemon leg (`ws_send`). Asserting on what the daemon
/// RECEIVED, rather than on `send` returning `Ok`, is the only way to tell a
/// delivered frame from one that was merely queued for a writer task that
/// never wrote it.
#[tokio::test]
async fn a_frame_sent_by_the_webview_reaches_the_daemon_verbatim() {
    let _serialised = crate::FD_TESTS.lock().await;

    const FRAME: &str = r#"{"op":"subscribe","id":"a"}"#;

    let (listener, transport) = listening().await;
    let manager = WsBridgeManager::new();

    let peer = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("upgrade");
        ws.next().await.expect("a frame").expect("a readable frame")
    });

    let reader = open_bridge(
        &transport,
        "c1".to_string(),
        "/v0/x".to_string(),
        silent_sink(),
        &manager,
    )
    .await
    .unwrap();

    manager
        .send("c1", FRAME.to_string())
        .expect("a live connection must accept frames");

    assert_eq!(
        peer.await.unwrap(),
        Message::Text(FRAME.to_string()),
        "the daemon must receive the frame the webview published, unaltered"
    );

    manager.close("c1");
    reader.await.expect("the reader must retire with it");
}

/// The shim sends on ids it believes are open; a stale one must come back as
/// an error it can act on, not a silently dropped frame.
#[test]
fn sending_on_a_connection_that_was_never_opened_is_an_error() {
    let manager = WsBridgeManager::new();

    let err = manager.send("nope", "{}".to_string()).unwrap_err();

    assert_eq!(err.to_string(), "no open ws connection nope");
}

/// The one state in which a live map entry has a dead writer: the writer task
/// ended (its socket died) before the reader retired the entry. Distinct from
/// the case above — the id IS open — and the shim is told so, rather than
/// having the frame vanish into a channel nobody drains.
#[test]
fn sending_over_a_connection_whose_writer_has_ended_is_an_error() {
    let manager = WsBridgeManager::new();
    let (tx, rx) = mpsc::unbounded_channel();
    let (cancel, _cancelled) = oneshot::channel();
    drop(rx); // the writer task is gone, but its entry is not
    manager.connections.lock().unwrap().insert(
        "c1".to_string(),
        Connection {
            generation: 0,
            tx,
            _cancel: cancel,
        },
    );

    let err = manager.send("c1", "{}".to_string()).unwrap_err();

    assert_eq!(err.to_string(), "ws connection c1 is closed");
}

/// A dial that fails must fail the open, with an error naming the route and
/// the cause — and must leave nothing behind: an entry for a connection that
/// never existed would accept `ws_send` calls forever and never deliver one.
#[tokio::test]
async fn a_dial_that_fails_leaves_no_connection_behind() {
    let _serialised = crate::FD_TESTS.lock().await;

    let (listener, transport) = listening().await;
    drop(listener); // the port is now both free and known-unused

    let manager = WsBridgeManager::new();
    let err = open_bridge(
        &transport,
        "c1".to_string(),
        "/v0/x".to_string(),
        silent_sink(),
        &manager,
    )
    .await
    .unwrap_err();

    assert!(
        err.to_string().contains("/v0/x") && err.to_string().contains("connect failed"),
        "the error must name the route and the cause; got {err:?}"
    );
    assert!(
        manager.connections.lock().unwrap().is_empty(),
        "a failed open must not register a connection"
    );
}

/// A writer whose socket has died must stop draining. The reader awaits the
/// writer, so a writer that will not stop is a reader that never finishes —
/// and a descriptor that never comes back, which is the whole failure this
/// file exists to prevent. The connection's channel is deliberately still
/// open here (this test holds a sender), so nothing BUT the failed write can
/// end the writer.
#[tokio::test]
async fn a_writer_whose_socket_died_stops_draining() {
    let _serialised = crate::FD_TESTS.lock().await;

    let (listener, transport) = listening().await;
    // The peer resets only when told to. It has to complete the upgrade
    // first, and the RST has to come after WE have finished reading the
    // upgrade's response: an RST does not merely close the connection, it
    // also empties the receiving side's buffer, and on Windows that is
    // enforced to the byte — the 101 the peer had already sent is thrown
    // away with it and `open_bridge` fails at the handshake with
    // WSAECONNRESET, before this test reaches the writer it is about. (Unix
    // hands over bytes it has already buffered regardless, which is why the
    // unsynchronised version only ever failed on the Windows runner.) The
    // signal below is sent once `open_bridge` has returned, i.e. once the
    // handshake is entirely in our hands and the only thing left for the
    // socket to carry is the reset itself.
    let (kill, killed) = oneshot::channel::<()>();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        // tokio deprecates `set_linger` because a NON-ZERO linger blocks the
        // thread when the socket drops. Zero is the opposite: close returns
        // immediately and sends the RST instead of a FIN, which is the only
        // deterministic way to make our next write fail. A clean FIN would
        // not do — writing to a half-closed socket succeeds, and the writer
        // would never learn. The alternative — closing with unread data
        // queued — depends on that data arriving first, which nothing here
        // can observe.
        #[allow(deprecated)]
        stream
            .set_linger(Some(Duration::ZERO))
            .expect("linger 0 makes close send an RST");
        let ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("upgrade");
        let _ = killed.await;
        drop(ws);
    });

    let manager = WsBridgeManager::new();
    let reader = open_bridge(
        &transport,
        "c1".to_string(),
        "/v0/x".to_string(),
        silent_sink(),
        &manager,
    )
    .await
    .unwrap();

    // The upgrade is complete on both sides, so nothing is left in flight
    // for the reset to destroy: from here it can only mean what this test
    // says it means, which is that the socket is dead.
    kill.send(())
        .expect("the peer must still be holding the socket open");

    let tx = manager
        .sender("c1")
        .expect("a live connection has a sender");

    // The reader retires the entry the moment it sees the reset, so an empty
    // map means the RST has been processed and the socket is unwritable.
    tokio::time::timeout(Duration::from_secs(5), async {
        while !manager.connections.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("the reader must retire a connection whose socket was reset");

    tx.send(Message::Text("{}".to_string()))
        .expect("the writer task is still draining its channel");

    tokio::time::timeout(Duration::from_secs(5), reader)
        .await
        .expect("a write that cannot succeed must end the writer, and with it the reader")
        .unwrap();

    drop(tx);
}
