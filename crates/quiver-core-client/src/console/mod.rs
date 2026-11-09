//! Running a console command on the active daemon, and streaming its output.
//!
//! `POST /v0/console/exec` answers with NDJSON: `out` frames as the command
//! writes, then exactly one `exit` frame (see `docs/console-spec.md`). The
//! webview cannot read a streamed body through the `quiver://` proxy, which
//! returns a response whole, so Rust is the HTTP client here: it dials the
//! active connection's own transport, splits the body into lines and pushes
//! each line, verbatim, down a Tauri `Channel` the frontend supplies.
//!
//! What this module does NOT do is interpret a command. The line is handed to
//! the daemon exactly as typed; the daemon owns the grammar, the allow-list and
//! every limit. This side only moves bytes, bounds its own memory, and makes
//! sure a frame is never lost silently:
//!
//!   * a daemon that answers with an error status (400 empty line, 403 not
//!     allowed, 429 busy, 401) becomes one `error` frame carrying the status and
//!     the daemon's message;
//!   * a connection that ends before an `exit` frame arrives becomes one `error`
//!     frame saying so, so the console never waits on a command that is gone;
//!   * dropping a run (cancel, page reload) closes the connection, which is what
//!     cancels the command on the daemon's side.
//!
//! Frame contents are never logged here: they are command output and may carry
//! anything the user ran.

mod exec;
mod runs;
mod splitter;

use std::time::Duration;

use crate::{Error, Result};

pub use exec::run_exec;
pub use runs::ConsoleExecManager;
pub use splitter::NdjsonSplitter;

/// Longest command line this side forwards. The daemon enforces 1024 bytes
/// itself; this is only a guard so a pathological paste cannot become a
/// megabyte IPC argument before the daemon gets to say no.
pub const MAX_LINE_BYTES: usize = 4096;

/// The daemon bounds a command at 10 minutes; this is the same bound plus a
/// little grace so the daemon's own timeout answer arrives before ours.
pub const EXEC_DEADLINE: Duration = Duration::from_secs(630);

/// Longest command line this side forwards. The daemon enforces 1024 bytes
/// itself; this is only a guard so a pathological paste cannot become a
/// megabyte IPC argument before the daemon gets to say no.
pub fn check_line(line: &str) -> Result<()> {
    if line.len() > MAX_LINE_BYTES {
        return Err(Error::LineTooLong(MAX_LINE_BYTES));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
