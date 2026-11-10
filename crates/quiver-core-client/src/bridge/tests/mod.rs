use super::*;
use crate::transport::http::HttpTransport;
use std::time::Duration;
use tokio::net::TcpListener;

#[test]
fn close_sentinel_without_a_close_frame_is_the_bare_sentinel() {
    assert_eq!(close_sentinel(None), WS_CLOSE_SENTINEL);
}

#[test]
fn close_sentinel_carries_the_daemons_code_and_reason() {
    let info = (1000_u16, "completed".to_string());
    assert_eq!(
        close_sentinel(Some(&info)),
        format!("{WS_CLOSE_SENTINEL}\u{0}1000\u{0}completed")
    );
}

// Every test below that opens a socket takes `crate::FD_TESTS`, the
// crate-wide guard — not just the one that counts descriptors. `open_fds()`
// is process-wide, so a listener held by ANY concurrent test lands in the
// count; see the guard's own doc for what that cost when it was scoped to
// this module alone.

/// File descriptors this process holds open (`/dev/fd` on macOS, a symlink to
/// `/proc/self/fd` on Linux). Process-wide, and therefore only meaningful while
/// `FD_TESTS` is held — see the note there.
fn open_fds() -> usize {
    std::fs::read_dir("/dev/fd").map(|d| d.count()).unwrap_or(0)
}

/// How much a run of descriptor counts rose for reasons other than its own
/// single biggest jump.
///
/// No single count is trustworthy, even with `crate::FD_TESTS` held: every
/// `#[tokio::test]` in this binary holds an IO driver (a kqueue and a wakeup
/// pipe) for as long as it runs, and the guard makes the OTHER socket tests
/// sit there blocked — holding theirs — for as long as the counting one runs.
/// Measured at `RUST_TEST_THREADS=24`: a single run reads 103 descriptors
/// throughout and dips to 99 once or twice as unrelated tests come and go.
///
/// So the question has to be "did the count grow", never "how high is it".
/// The shape of the growth is what answers it. A leak is PER CYCLE: it rises
/// a little, again and again, for as long as the loop runs. Those blocked
/// suite-mates are an ARRIVAL: they take their descriptors once and then sit
/// there, so the count steps up and goes flat, however far it stepped.
///
/// Summing the rises and discarding the largest keeps the first and drops
/// the second. Comparing two halves of the run cannot: it only asks where
/// the level ended up, so one arrival that outlives the halfway point reads
/// exactly like a leak. That is not theoretical — see `DRIFT`.
/// Rises are measured against the running MAXIMUM, not against the previous
/// sample. A leak only ever shows up as the count reaching levels it has
/// not reached before, so that is the thing worth measuring — and it is the
/// only reading that survives a transient DIP.
///
/// Consecutive-sample deltas do not. CI produced
/// `[.. 25, 25, 21, 24, 24, 25, 25 ..]` on this branch, which touches no
/// Rust: descriptors momentarily released and immediately reclaimed. Net
/// change zero, peak never above the 26 seen earlier — yet the recovery
/// alone counted +3 and +1, because falls were never credited against the
/// rises they precede. Two runs failed that way, on identical code, while
/// macOS, Windows and tarpaulin passed.
///
/// Against the running maximum the same dip contributes nothing, and both
/// shapes the threshold was calibrated on are unchanged — see
/// `an_arrival_is_not_a_leak_but_a_slow_leak_is`, whose expected values
/// still hold exactly: an arrival is one jump to a new plateau, and a leak
/// keeps setting new maxima all run long.
fn growth_beyond_one_arrival(counts: &[usize]) -> usize {
    let mut peak = match counts.first() {
        Some(&first) => first,
        None => return 0,
    };
    let mut rises: Vec<usize> = Vec::new();
    for &count in &counts[1..] {
        if count > peak {
            rises.push(count - peak);
            peak = count;
        }
    }
    rises.sort_unstable();
    rises.pop();
    rises.iter().sum()
}

/// The statistic itself, against the two shapes it has to tell apart. The
/// arrival is the run CI actually produced on Rust this branch never
/// touched; the leak is the rate the threshold was calibrated for.
#[test]
fn an_arrival_is_not_a_leak_but_a_slow_leak_is() {
    let mut arrival = vec![21_usize; 93];
    arrival.extend(std::iter::repeat_n(25, 6));
    arrival.extend(std::iter::repeat_n(26, 101));
    assert_eq!(arrival.len(), 200);
    assert_eq!(
        growth_beyond_one_arrival(&arrival),
        1,
        "five descriptors taken once and held is one jump, not a leak"
    );

    // The shape CI actually produced, twice, on a branch that touches no Rust:
    // a transient dip and its recovery, peaking no higher than it already had.
    // Measured against consecutive samples this scored 3 and failed the run.
    let mut dip = vec![25_usize; 36];
    dip.extend(std::iter::repeat_n(26, 38));
    dip.extend(std::iter::repeat_n(25, 9));
    dip.push(21);
    dip.extend([24, 24]);
    dip.extend(std::iter::repeat_n(25, 114));
    assert_eq!(dip.len(), 200);
    assert_eq!(
        growth_beyond_one_arrival(&dip),
        0,
        "reclaiming descriptors it had already released is not growth"
    );

    // One descriptor every 37 closes — the weakest leak this test claims to see.
    let leak: Vec<usize> = (0..200).map(|i| 21 + i / 37).collect();
    assert_eq!(
        growth_beyond_one_arrival(&leak),
        4,
        "a leak spreads its rises, so discarding the largest barely dents it"
    );
}

/// A transport pointed at a live loopback listener. TCP rather than a unix
/// socket so these tests run identically on every platform: the bridge only
/// ever sees the `WsStream` a transport hands back, so which kind of socket
/// carries it is not what is under test here.
async fn listening() -> (TcpListener, HttpTransport) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    (listener, HttpTransport::new(format!("http://{addr}"), None))
}

struct ClosureSink<F>(F);

impl<F: Fn(String) + Send + Sync + 'static> FrameSink for ClosureSink<F> {
    fn send(
        &self,
        frame: String,
    ) {
        (self.0)(frame)
    }
}

/// A sink that fires `tx` when the close sentinel arrives, so a test can block on
/// the connection actually being torn down rather than on a clock.
fn sentinel_sink(tx: oneshot::Sender<()>) -> impl FrameSink {
    let tx = Mutex::new(Some(tx));
    ClosureSink(move |frame: String| {
        if frame == WS_CLOSE_SENTINEL {
            if let Some(tx) = tx.lock().unwrap().take() {
                let _ = tx.send(());
            }
        }
    })
}

fn silent_sink() -> impl FrameSink {
    ClosureSink(|_| {})
}

/// Records every frame it is handed, so a test can assert on what did NOT
/// reach the shim as well as on what did.
fn recording_sink(seen: &Arc<Mutex<Vec<String>>>) -> impl FrameSink {
    let seen = Arc::clone(seen);
    ClosureSink(move |frame: String| seen.lock().unwrap().push(frame))
}

/// The daemon closing a stream on its own initiative — what it does on every
/// watcher release and workspace switch.
async fn accept_then_close(listener: &TcpListener) {
    if let Ok((stream, _)) = listener.accept().await {
        if let Ok(ws) = tokio_tungstenite::accept_async(stream).await {
            drop(ws);
        }
    }
}

/// A wedged daemon: it completes the upgrade and then never touches the socket again
/// — never reads it, never answers, never closes it.
///
/// Getting this peer right is the whole point of the two tests below. Retiring a
/// connection ends the writer, which half-closes our socket (`SHUT_WR`) — and a
/// *healthy* daemon reacts to that EOF by closing its end, which wakes our reader all
/// by itself. So a fake daemon that reads its socket cannot tell a working teardown
/// from a broken one: both look identical. This one never reads, so nothing but our
/// own cancellation can free the descriptor, which is precisely the case cancellation
/// exists for — and the case a wedged daemon actually presents.
fn spawn_wedged_daemon(listener: TcpListener) {
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            if let Ok(_ws) = tokio_tungstenite::accept_async(stream).await {
                // Hold the connection open forever, without ever polling it.
                std::future::pending::<()>().await;
            }
        }
    });
}

mod frames;
mod lifetime;
