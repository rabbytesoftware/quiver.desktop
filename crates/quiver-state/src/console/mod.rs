//! The daemon console and the build indicator (port of `src/features/console`; spec
//! `docs/console-spec.md`). Everything here is sans-IO: the controller returns the effects to carry out
//! and is told what happened.

pub mod build_info;
pub mod commands;
pub mod controller;
pub mod entries;
pub mod exec;
pub mod frames;
pub mod history;
pub mod lines;
pub mod ring;
pub mod state;
pub mod stream;
pub mod versions;

pub use commands::{complete_line, parse_commands, render_help, Completion, ConsoleCommand};
pub use controller::{ConsoleController, ConsoleEffect, TimerKind, FLUSH_MS};
pub use entries::{note_text, ConsoleEntry, NewEntry, Note, Stream, StreamState, Support, Tone};
pub use frames::{
    format_log_time, json_lines, parse_log_frame, FieldKind, LogFrame, LogLevel, LogRecord,
};
pub use history::{Direction, HistoryCursor};
pub use state::{ConsoleState, BUFFER_CAP, HISTORY_CAP};
pub use versions::{parse_versions, supports_console, CoreVersions, CONSOLE_FEATURE};
