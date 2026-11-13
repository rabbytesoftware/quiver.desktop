use super::*;

#[tokio::test]
async fn daemon_initiated_close_retires_the_connection() {
    let _serialised = crate::FD_TESTS.lock().await;

    let (listener, transport) = listening().await;
    let manager = WsBridgeManager::new();

    let (tx, rx) = oneshot::channel();
    let opened = open_bridge(
        &transport,
        "c1".to_string(),
        "/v0/x".to_string(),
        sentinel_sink(tx),
        &manager,
    );
    let (opened, _) = tokio::join!(opened, accept_then_close(&listener));
    opened.unwrap();

    rx.await.expect("the shim must be told the daemon closed");

    assert!(
        manager.connections.lock().unwrap().is_empty(),
        "the daemon closed the stream, so the connection must be retired; a connection \
			 left in the map strands its writer task on the socket's write half and burns \
			 one of the app's descriptors for good"
    );
}

/// The production bug, as a regression test: 215 of these closes in one window is what
/// a pre-crash daemon log shows, against an app whose descriptor limit was 256.
///
/// This is the one test that has to count descriptors. A stranded writer still lets its
/// reader finish, so nothing local to the connection reveals it — only the process's
/// descriptor table does.
#[tokio::test]
async fn daemon_initiated_closes_do_not_leak_file_descriptors() {
    let _serialised = crate::FD_TESTS.lock().await;

    /// Enough closes that a leak on a FRACTION of them still separates the
    /// two halves, which is the lever that sets how small a leak this test
    /// can see: for one descriptor every `N`th close the halves' medians
    /// differ by `CYCLES / 2N`, so `CYCLES` buys sensitivity directly.
    /// Measured cost: the test runs in 60-90ms here against 10-20ms at 20
    /// cycles, and the whole suite goes from ~0.05s to ~0.10s.
    const CYCLES: usize = 200;

    /// How much the count may rise, on top of its single largest jump,
    /// without being called a leak.
    ///
    /// This once compared the medians of the run's two halves, and the
    /// margin was set on the reasoning that the only sustained shift — the
    /// other socket tests piling up on `crate::FD_TESTS` — saturates within
    /// the first cycles, i.e. inside the first half, where it cancels out.
    ///
    /// That is an assumption about how fast the SUITE runs, and under
    /// `cargo tarpaulin` it is false: ptrace slows everything enough that
    /// the pile-up lands mid-run instead. CI caught it twice on Rust this
    /// branch never touched — five descriptors appearing at cycle 66 in one
    /// run and cycle 93 in the next, then FLAT for the remaining hundred
    /// cycles. Both moved the late median by the whole five. Every plain
    /// `cargo test` job on the same commit passed.
    ///
    /// Discarding the largest jump costs nothing against a real leak, which
    /// spreads its rises across the run: at one descriptor every 37 closes
    /// there are five separate rises of one, and dropping a single one of
    /// them still leaves four. So 2 stays what it was — margin for a slower
    /// or busier machine — and the sensitivity it was measured against (one
    /// in 37 closes, 20/20 runs; one in 38 escapes, 0/15) is unchanged.
    /// The setting they replace (20 cycles, drift 6) needed SEVEN leaks per
    /// TEN closes: a bridge leaking on half of the 215 closes in this test's
    /// own header would have stranded 107 descriptors and passed.
    const DRIFT: usize = 2;

    let (listener, transport) = listening().await;
    let manager = WsBridgeManager::new();
    let mut counts = Vec::with_capacity(CYCLES);

    // Every cycle is one reconnect: the shim opens a fresh conn_id, the daemon closes it.
    for i in 0..CYCLES {
        let (tx, rx) = oneshot::channel();
        let opened = open_bridge(
            &transport,
            format!("c{i}"),
            "/v0/x".to_string(),
            sentinel_sink(tx),
            &manager,
        );
        let (opened, _) = tokio::join!(opened, accept_then_close(&listener));
        opened.unwrap();
        rx.await.unwrap();

        // Sampled after the connection is fully retired — the sentinel is sent only
        // once both split halves are gone — so a leak-free cycle returns to the level
        // the previous one started from, and a leaking one does not.
        counts.push(open_fds());
    }

    let growth = growth_beyond_one_arrival(&counts);
    assert!(
        growth <= DRIFT,
        "{CYCLES} daemon-initiated closes leaked file descriptors: the count rose by \
			 {growth} beyond its largest single jump ({counts:?}). Once the app's limit is \
			 reached quiver.core cannot be dialled at all — which the health watchdog reads \
			 as a dead backend and kills a daemon that was never unhealthy."
    );
}

/// The reader parks on `read.next()`. Against a daemon that has stopped reading, the
/// half-close that retiring performs is never noticed and never answered, so nothing but
/// explicit cancellation can end the reader — and this test hangs.
///
/// The reader finishing IS the assertion: it awaits the writer, so by the time it returns
/// both split halves are dropped and the socket is closed by ownership. Counting
/// descriptors here would add nothing and would only make the test race every other test
/// that opens one.
#[tokio::test]
async fn explicit_close_releases_the_socket_against_a_daemon_that_never_reacts() {
    let _serialised = crate::FD_TESTS.lock().await;

    let (listener, transport) = listening().await;
    spawn_wedged_daemon(listener);
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

    // Exactly what `ws_close` does, and the only thing it does.
    manager.close("c1");

    reader
        .await
        .expect("retiring a connection must end its reader, whatever the daemon does");
}

/// A page load orphans every connection the old page owned: its JS is gone and will never
/// call `ws_close` for ids it no longer remembers. Nothing else can notice — a `Channel`
/// keeps working across a reload — so `close_all` is all that stands between a reload and
/// a permanently stranded socket.
#[tokio::test]
async fn close_all_retires_connections_a_reloaded_page_abandoned() {
    let _serialised = crate::FD_TESTS.lock().await;

    let (listener, transport) = listening().await;
    spawn_wedged_daemon(listener);
    let manager = WsBridgeManager::new();

    let seen = Arc::new(Mutex::new(Vec::new()));
    let reader = open_bridge(
        &transport,
        "c1".to_string(),
        "/v0/x".to_string(),
        recording_sink(&seen),
        &manager,
    )
    .await
    .unwrap();

    manager.close_all();

    reader.await.expect("a page load must end its readers");

    assert!(manager.connections.lock().unwrap().is_empty());

    // The reader has finished, so anything it was going to announce, it has.
    let announced = seen.lock().unwrap().clone();
    assert!(
        announced.is_empty(),
        "a page load must not announce a close: the sentinel is what makes the \
			 shim reconnect, and the ids it would reconnect belong to a page that no \
			 longer exists. Got {announced:?}"
    );
}

// ── Beyond Crowbar's four lifetime tests ──────────────────────────────
// The four above are the descriptor-lifetime regression suite and say
// nothing about what the bridge carries, because Crowbar's peers close or
// wedge before a single frame moves. The rest of this module covers what
// the ported code actually does with a live stream — and each of these
// paths is reached in the app only through `lib.rs`'s command wrappers,
// which are coverage-excluded, so it has to be asserted from here.
