//! The teardown half of a connection switch.

use crate::bridge::WsBridgeManager;
use crate::types::QuiverConnection;

/// The teardown half of a connection switch: retire every bridged stream, then
/// tear down the connection they were dialled over.
///
/// Every open stream belongs to the OUTGOING connection — its id addresses that
/// peer, and the frontend reopens against the new one once the switch is done —
/// so leaving them behind strands a socket apiece exactly the way a page reload
/// did before `on_page_load` retired them (see `connection::bridge`, teardown
/// paths (2) and (3)).
///
/// Streams first, connection second. That ordering has a subject now:
/// `local::LocalConnection::teardown` kills the daemon, and killing the daemon
/// takes down every socket the local connection's streams are riding on. A reader
/// that sees its socket die (rather than being cancelled) treats it as the daemon
/// ending the stream and announces `WS_CLOSE_SENTINEL`, which tells the JS shim
/// to reconnect. `close_all()` first is what cancels those readers instead.
///
/// Two things this comment will not overclaim, because neither is tested:
///
///   * the ordering is still not *demonstrated*. Reaching it needs a live daemon
///     whose death closes a real bridged socket, which the suite cannot stand up;
///     swapping these two lines leaves it green. What is tested is each half —
///     `a_switch_retires_the_outgoing_connections_streams` for the retirement,
///     and `sidecar::tests` for the reap.
///   * retiring first NARROWS the race rather than closing it. `close_all()` only
///     makes the reader's cancel channel ready; the kill immediately after makes
///     its socket ready too, and `bridge`'s `tokio::select!` is unbiased, so a
///     reader woken with both pending may still take the socket arm and announce.
///     Left alone deliberately: the consequence is one spurious reconnect, and
///     `ws_open` resolves the transport per open and queues on `active`'s read
///     lock behind this switch's writer — so it lands on the NEW connection,
///     which is where the frontend was going anyway. `biased;` in that `select!`
///     would make it exact, at the cost of a change to code this item is not
///     about.
///
/// It is a free function rather than two lines inline so it can be driven
/// directly under test.
pub(super) async fn retire_streams_and_teardown(
    bridge: &WsBridgeManager,
    outgoing: &dyn QuiverConnection,
) {
    bridge.close_all();
    outgoing.teardown().await;
}
