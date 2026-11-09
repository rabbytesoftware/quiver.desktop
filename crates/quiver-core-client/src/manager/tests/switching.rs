use super::*;
use crate::bridge::open_bridge;
use std::time::Duration;
use tokio::net::TcpListener;

/// A switch leaves every stream of the outgoing connection behind unless it
/// retires them: their ids address a peer the app has just left, and their
/// JS will never call `ws_close` for them because it reopens fresh ids
/// against the new connection instead. Nothing else can end them — the peer
/// here never reacts, which is exactly what a wedged or remote daemon looks
/// like — so the reader finishing IS the proof that the switch retired it,
/// and with it that the socket came back.
#[tokio::test]
async fn a_switch_retires_the_outgoing_connections_streams() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            if let Ok(_ws) = tokio_tungstenite::accept_async(stream).await {
                // Upgraded, then never polled again.
                std::future::pending::<()>().await;
            }
        }
    });
    let transport = HttpTransport::new(format!("http://{addr}"), None);

    let bridge = WsBridgeManager::new();
    let announced = Arc::new(Mutex::new(Vec::new()));
    let reader = open_bridge(
        &transport,
        "c1".to_string(),
        "/v0/x".to_string(),
        Recorder(Arc::clone(&announced)),
        &bridge,
    )
    .await
    .expect("the stream must open");

    let torn_down = Arc::new(AtomicBool::new(false));
    let outgoing = StubConnection::new("local", Arc::clone(&torn_down));

    retire_streams_and_teardown(&bridge, &outgoing).await;

    // A timeout rather than a plain await: without the retirement this hangs
    // forever, and a hung suite is a worse signal than a failed assertion.
    tokio::time::timeout(Duration::from_secs(5), reader)
        .await
        .expect(
            "a switch must end the outgoing connection's readers: a stream left open \
				 holds a socket to a peer the app has switched away from, for the life of \
				 the process",
        )
        .expect("the reader task must not panic");

    assert!(
        torn_down.load(Ordering::Relaxed),
        "the outgoing connection must still be torn down"
    );
    // Cloned out of the mutex before asserting: `assert!` evaluates its format
    // arguments while the condition's temporary guard is still alive, and
    // locking a std `Mutex` twice on one thread deadlocks.
    let announced = announced.lock().unwrap().clone();
    assert!(
        announced.is_empty(),
        "a switch must retire streams silently: the close sentinel is what makes the \
			 shim reconnect, and reconnecting mid-switch dials the peer being left. \
			 Got {announced:?}"
    );
}

// ── The lock discipline of a switch (finding B) ──────────────────────────

/// The whole of finding B, stated as a test.
///
/// A switch's `prepare` step dials the new peer, and a peer that accepts a
/// connection and then stalls makes it take forever. It used to run under
/// `active`'s WRITE lock — so tokio's fair `RwLock` parked every subsequent
/// reader behind the waiting writer, and every `quiver://` request and every
/// `ws_open` in the app hung, past every timeout, until the app was killed.
///
/// Here `prepare` never resolves, and the manager must stay readable
/// throughout. Move the `active.write()` in `switch_to` back above
/// `prepare.await` and this stops finishing.
#[tokio::test]
async fn a_switch_that_never_finishes_preparing_still_leaves_the_app_readable() {
    let manager = Arc::new(manager_with(StubConnection::boxed(
        "local",
        Arc::new(AtomicBool::new(false)),
    )));

    let switching = tokio::spawn({
        let manager = Arc::clone(&manager);
        async move {
            manager
                .switch_to(async {
                    std::future::pending::<()>().await;
                    unreachable!("a stalled peer never answers")
                })
                .await
        }
    });

    // Give the switch a chance to reach (and park in) its prepare step
    // before reading, so the read really does race a switch in flight.
    tokio::task::yield_now().await;

    tokio::time::timeout(Duration::from_secs(5), manager.transport())
        .await
        .expect(
            "a switch that is still dialling must not hold any lock on the active \
				 connection: every quiver:// request in the app takes that read lock",
        );

    switching.abort();
}

/// A switch that cannot be completed must leave the previous connection —
/// and its streams — exactly as they were. `switch_to` must not touch
/// `active` until `prepare` has succeeded, which is why the build and start
/// halves live inside `prepare` rather than around it.
#[tokio::test]
async fn a_switch_whose_preparation_fails_changes_nothing() {
    let _serialised = crate::FD_TESTS.lock().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            if let Ok(_ws) = tokio_tungstenite::accept_async(stream).await {
                std::future::pending::<()>().await;
            }
        }
    });

    let torn_down = Arc::new(AtomicBool::new(false));
    let manager = manager_with(StubConnection::boxed("local", Arc::clone(&torn_down)));
    let bridge = Arc::clone(manager.bridge());
    let reader = open_bridge(
        &HttpTransport::new(format!("http://{addr}"), None),
        "c1".to_string(),
        "/v0/x".to_string(),
        Recorder(Arc::new(Mutex::new(Vec::new()))),
        &bridge,
    )
    .await
    .expect("the stream must open");

    let err = manager
        .switch_to(async { Err(Error::ConnectionNotFound("nope".into())) })
        .await
        .expect_err("a failed preparation must fail the switch");
    assert_eq!(err.to_string(), "connection nope not found");

    assert_eq!(
        active_id(&manager).await,
        "local",
        "a failed switch must leave the previous connection active"
    );
    assert!(
        !torn_down.load(Ordering::Relaxed),
        "a failed switch must not tear down the connection it failed to replace"
    );
    assert!(
        !reader.is_finished(),
        "a failed switch must leave the previous connection's streams open: the \
			 frontend is still talking to that peer"
    );

    bridge.close_all();
    let _ = tokio::time::timeout(Duration::from_secs(5), reader).await;
}

/// A successful switch installs the new connection and retires the outgoing
/// one — the ordering `retire_streams_and_teardown` documents, now reached
/// through the path production actually takes.
#[tokio::test]
async fn a_successful_switch_installs_the_new_connection_and_retires_the_old() {
    let torn_down = Arc::new(AtomicBool::new(false));
    let manager = manager_with(StubConnection::boxed("local", Arc::clone(&torn_down)));

    manager
        .switch_to(async {
            Ok((
                StubConnection::boxed("remote-1", Arc::new(AtomicBool::new(false))),
                CoreStatus::Ready,
            ))
        })
        .await
        .expect("the switch must succeed");

    assert_eq!(active_id(&manager).await, "remote-1");
    assert!(
        torn_down.load(Ordering::Relaxed),
        "the outgoing connection must be torn down"
    );
}

// ── The exit path ────────────────────────────────────────────────────────

/// Quitting the app must reap what the app started. `lib.rs`'s `RunEvent`
/// handler has nothing to reach the active connection with but this, and
/// before it existed the local daemon simply outlived every session.
///
/// It runs twice on the normal path — `ExitRequested` then `Exit` — which is
/// safe because everything it reaches is idempotent; the handle only comes out
/// of its slot once (`sidecar::tests::a_spawned_child_is_taken_exactly_once`).
#[tokio::test]
async fn shutdown_tears_down_the_active_connection() {
    let torn_down = Arc::new(AtomicBool::new(false));
    let manager = manager_with(StubConnection::boxed("local", Arc::clone(&torn_down)));

    manager.shutdown().await;

    assert!(
        torn_down.load(Ordering::Relaxed),
        "quitting must tear the active connection down: nothing else can kill \
			 the daemon this app spawned"
    );
}

/// Quitting while a switch is still dialling must not hang the app. The switch
/// holds `switching` for its whole duration and a stalled peer holds it for
/// the health timeout — so `shutdown` deliberately does not take it, and takes
/// a read lock on a connection no writer can be holding.
///
/// Make `shutdown` wait on `switching` (or on `active.write()`) and this stops
/// finishing: the quit would sit behind a peer that never answers.
#[tokio::test]
async fn shutdown_during_a_stalled_switch_still_reaps() {
    let torn_down = Arc::new(AtomicBool::new(false));
    let manager = Arc::new(manager_with(StubConnection::boxed(
        "local",
        Arc::clone(&torn_down),
    )));

    let switching = tokio::spawn({
        let manager = Arc::clone(&manager);
        async move {
            manager
                .switch_to(async {
                    std::future::pending::<()>().await;
                    unreachable!("a stalled peer never answers")
                })
                .await
        }
    });

    // Let the switch reach its prepare step, so the quit really does race a
    // switch in flight.
    tokio::task::yield_now().await;

    tokio::time::timeout(Duration::from_secs(5), manager.shutdown())
        .await
        .expect("a quit must not wait on a switch that is still dialling");
    assert!(
        torn_down.load(Ordering::Relaxed),
        "and it must still reap the connection that is active"
    );

    switching.abort();
}

/// `switching` exists only to keep the serialisation the write lock used to
/// provide, now that the write lock is no longer held across the dialling
/// half of a switch. Two switches that interleave would install in an order
/// unrelated to the order they were asked for, and could retire the streams
/// of a connection the other has already replaced.
///
/// Each `prepare` records when it entered and left; overlapping intervals
/// are the defect. Drop the `switching` lock from `switch_to` and the two
/// yields below let them interleave, which turns this red.
#[tokio::test]
async fn two_switches_do_not_interleave() {
    let manager = Arc::new(manager_with(StubConnection::boxed(
        "local",
        Arc::new(AtomicBool::new(false)),
    )));
    let log = Arc::new(Mutex::new(Vec::<String>::new()));

    let switch = |id: &'static str| {
        let manager = Arc::clone(&manager);
        let log = Arc::clone(&log);
        async move {
            manager
                .switch_to(async move {
                    log.lock().unwrap().push(format!("enter {id}"));
                    // Two yields: ample opportunity for the other
                    // switch's prepare to slip in, if anything let it.
                    tokio::task::yield_now().await;
                    tokio::task::yield_now().await;
                    log.lock().unwrap().push(format!("leave {id}"));
                    Ok((
                        StubConnection::boxed(id, Arc::new(AtomicBool::new(false))),
                        CoreStatus::Ready,
                    ))
                })
                .await
        }
    };

    let (a, b) = tokio::join!(switch("a"), switch("b"));
    a.expect("switch a must succeed");
    b.expect("switch b must succeed");

    let log = log.lock().unwrap().clone();
    assert!(
        log == ["enter a", "leave a", "enter b", "leave b"]
            || log == ["enter b", "leave b", "enter a", "leave a"],
        "switches must not interleave; got {log:?}"
    );
}
