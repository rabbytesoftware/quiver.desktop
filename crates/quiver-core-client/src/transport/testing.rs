//! Fixture parts shared by the transports' own tests and by the `quiver://`
//! proxy's, which drives a real `HttpTransport` at a real socket rather than a
//! stand-in. Lives here rather than in either module so the two canned-response
//! listeners cannot drift apart.

use tokio::io::{AsyncRead, AsyncReadExt};

/// Read the whole request the client sent, before answering it.
///
/// A fixture that answers WITHOUT reading leaves the request sitting unread
/// in its receive queue, and closing a socket that still holds unread
/// inbound data is an ABORTIVE close: the stack sends RST where it would
/// otherwise send FIN. Windows applies that strictly — the client's next
/// read fails with WSAECONNRESET (os error 10054) and its receive buffer is
/// discarded, response bytes already delivered included — so the client
/// loses the very answer the fixture just wrote. Linux and macOS leave data
/// the client has already buffered alone, which is the only reason a
/// write-and-close fixture ever passed anywhere. Draining first is also
/// exactly what a real HTTP server does, so the fixture is more honest for
/// it.
///
/// Bounded by the REQUEST, never by a clock: the head ends at the first
/// blank line, and the body is exactly the `Content-Length` the client
/// announced — none, for a GET. A peer that hangs up mid-request reads 0 and
/// ends the drain, so this can never park waiting for bytes that will never
/// come.
pub(crate) async fn drain_request<S: AsyncRead + Unpin>(sock: &mut S) {
    let mut req = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        if let Some(head) = req.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = head + 4;
            if req.len() - head >= announced_body_len(&req[..head]) {
                return;
            }
        }
        match sock.read(&mut chunk).await {
            // EOF, or a peer that reset: there is nothing further to
            // read, and waiting for more would be waiting forever.
            Ok(0) | Err(_) => return,
            Ok(n) => req.extend_from_slice(&chunk[..n]),
        }
    }
}

/// How many body bytes the request's head says follow it — zero when it says
/// nothing, which is what every GET says.
fn announced_body_len(head: &[u8]) -> usize {
    String::from_utf8_lossy(head)
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse().ok())
        .unwrap_or(0)
}
