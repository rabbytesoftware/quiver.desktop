//! The helper's side of the host IPC: JSON lines on stdin (host -> window) and stdout (window -> host).

use quiver_arrow_host::{decode_line, encode_line, HostToWindow, WindowToHost};
use std::io::{BufRead, Write};

/// Reads host messages until stdin closes (the host quit or crashed) and hands each to `on_message`.
/// When stdin closes the callback receives [`HostToWindow::Close`] so the window never outlives its host.
pub fn read_host_messages(on_message: impl Fn(HostToWindow) + Send + 'static) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            match decode_line::<HostToWindow>(&line) {
                Ok(msg) => on_message(msg),
                Err(e) => eprintln!("arrow-window: bad host message: {e}"),
            }
        }
        on_message(HostToWindow::Close);
    });
}

/// Sends one message to the host. Failure means the host is gone; the caller closes the window.
pub fn send_to_host(msg: &WindowToHost) -> bool {
    let Ok(line) = encode_line(msg) else {
        return false;
    };
    let mut out = std::io::stdout().lock();
    writeln!(out, "{line}").and_then(|()| out.flush()).is_ok()
}
