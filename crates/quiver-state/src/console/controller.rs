//! Everything the console does that is not rendering: noticing whether the daemon has a console, keeping
//! the log stream open while the console is, and running commands.
//!
//! Sans-IO. Each call returns the [`ConsoleEffect`]s the owner must carry out (open a socket, fetch
//! `/versions`, start a timer, run a command) and the owner reports back what happened. What the user sees
//! is in the [`ConsoleState`] it owns.

use std::collections::HashMap;

use super::commands::{render_help, ConsoleCommand};
use super::entries::{NewEntry, Note, Stream, StreamState, Support, Tone};
use super::exec::ExecOutput;
use super::frames::{parse_log_frame, LogFrame};
use super::state::ConsoleState;
use super::stream::{LogStream, StreamAction};
use super::versions::CoreVersions;

/// Frames from the stream are folded into the state in batches, not one render each.
pub const FLUSH_MS: u64 = 50;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimerKind {
    /// The reconnect backoff.
    Backoff,
    /// The batch flush.
    Flush,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ConsoleEffect {
    /// Report the answer with `on_versions(epoch, ..)`.
    FetchVersions {
        epoch: u64,
    },
    /// Report the answer with `on_commands(epoch, .., for_help)`.
    FetchCommands {
        epoch: u64,
        for_help: bool,
    },
    /// Report its events with `stream_opened`, `stream_message` and `stream_closed` under `attempt`.
    OpenStream {
        path: String,
        attempt: u64,
    },
    /// Close it without reporting the close.
    CloseStream {
        attempt: u64,
    },
    /// Report the expiry with `timer_elapsed(kind)`.
    StartTimer {
        kind: TimerKind,
        ms: u64,
    },
    CancelTimer {
        kind: TimerKind,
    },
    /// Send `line` exactly as typed; report each NDJSON line with `exec_frame(run, ..)`.
    StartExec {
        run: u64,
        line: String,
    },
    /// Close the connection, which cancels the command on the daemon.
    CancelExec {
        run: u64,
    },
}

#[derive(Debug, Default)]
pub struct ConsoleController {
    pub state: ConsoleState,
    ready: bool,
    /// Bumped on every connection change, so a slow answer for the previous connection is ignored.
    epoch: u64,
    stream: LogStream,
    queue: Vec<LogFrame>,
    /// The replay of the connection being made, held back until its `ready`. A `ready` that says `reset`
    /// means the daemon restarted and the replay replaces what is shown, which cannot be decided until the
    /// whole replay is in hand. `None` once the replay is over (live frames pass straight through).
    replay: Option<Vec<LogFrame>>,
    flush_pending: bool,
    runs: HashMap<u64, (u64, ExecOutput)>,
    next_run: u64,
}

impl ConsoleController {
    fn reconcile(&mut self, fx: &mut Vec<ConsoleEffect>) {
        let wanted = self.state.open && self.state.support == Support::Supported && self.ready;
        if wanted && !self.stream.running() {
            let actions = self.stream.start(self.state.cursor);
            self.apply_stream(actions, fx);
        }
        if !wanted && self.stream.running() {
            self.flush(fx);
            let actions = self.stream.stop();
            self.apply_stream(actions, fx);
        }
    }

    fn apply_stream(&mut self, actions: Vec<StreamAction>, fx: &mut Vec<ConsoleEffect>) {
        for action in actions {
            match action {
                StreamAction::Open { path, attempt } => {
                    fx.push(ConsoleEffect::OpenStream { path, attempt })
                }
                StreamAction::Close { attempt } => fx.push(ConsoleEffect::CloseStream { attempt }),
                StreamAction::Wait { ms } => fx.push(ConsoleEffect::StartTimer {
                    kind: TimerKind::Backoff,
                    ms,
                }),
                StreamAction::CancelWait => fx.push(ConsoleEffect::CancelTimer {
                    kind: TimerKind::Backoff,
                }),
                StreamAction::State(state) => {
                    // Every new socket begins with a replay -- and its frames can beat `open`, so this
                    // starts at the attempt, not at the open.
                    match state {
                        StreamState::Connecting | StreamState::Reconnecting => {
                            self.replay = Some(vec![])
                        }
                        StreamState::Idle => self.replay = None,
                        StreamState::Live => {}
                    }
                    self.state.set_stream(state);
                }
            }
        }
    }

    fn flush(&mut self, fx: &mut Vec<ConsoleEffect>) {
        if self.flush_pending {
            fx.push(ConsoleEffect::CancelTimer {
                kind: TimerKind::Flush,
            });
            self.flush_pending = false;
        }
        if !self.queue.is_empty() {
            let frames = std::mem::take(&mut self.queue);
            self.state.ingest(&frames);
        }
    }

    pub fn set_open(&mut self, open: bool) -> Vec<ConsoleEffect> {
        let mut fx = Vec::new();
        let changed = self.state.open != open;
        self.state.set_open(open);
        if changed {
            self.reconcile(&mut fx);
        }
        fx
    }

    pub fn toggle(&mut self) -> Vec<ConsoleEffect> {
        self.set_open(!self.state.open)
    }

    /// The connection or its readiness changed.
    pub fn sync(&mut self, connection_id: &str, ready: bool) -> Vec<ConsoleEffect> {
        let mut fx = Vec::new();
        let changed = self.state.connection_id.as_deref() != Some(connection_id);
        let became_ready = ready && !self.ready;
        self.ready = ready;
        if changed {
            self.epoch += 1;
            self.flush(&mut fx);
            let actions = self.stream.stop();
            self.apply_stream(actions, &mut fx);
            self.cancel_runs(&mut fx);
            self.queue.clear();
            self.replay = None;
            self.state.adopt(connection_id);
        }
        if ready && (changed || became_ready) {
            fx.push(ConsoleEffect::FetchVersions { epoch: self.epoch });
        }
        self.reconcile(&mut fx);
        fx
    }

    /// `None` is a daemon that could not be reached: keep what is known; the next `ready` tries again.
    pub fn on_versions(
        &mut self,
        epoch: u64,
        versions: Option<CoreVersions>,
    ) -> Vec<ConsoleEffect> {
        let mut fx = Vec::new();
        let Some(versions) = versions else { return fx };
        if epoch != self.epoch {
            return fx;
        }
        self.state.set_versions(Some(versions));
        if self.state.support == Support::Supported && !self.state.commands_loaded {
            fx.push(ConsoleEffect::FetchCommands {
                epoch,
                for_help: false,
            });
        }
        self.reconcile(&mut fx);
        fx
    }

    /// `commands` is `None` when the table could not be had: help and completion are conveniences, the
    /// console works without them.
    pub fn on_commands(
        &mut self,
        epoch: u64,
        commands: Option<Vec<ConsoleCommand>>,
        for_help: bool,
    ) {
        if epoch != self.epoch {
            return;
        }
        match commands {
            Some(commands) => {
                self.state.set_commands(commands);
                if for_help {
                    self.show_help();
                }
            }
            None if for_help => self.state.push(vec![NewEntry::Note {
                tone: Tone::Error,
                note: Note::HelpUnavailable,
            }]),
            None => {}
        }
    }

    fn show_help(&mut self) {
        let lines = render_help(&self.state.commands)
            .into_iter()
            .map(|text| NewEntry::Out {
                stream: Stream::Stdout,
                text,
            })
            .collect();
        self.state.push(lines);
    }

    pub fn submit(&mut self, line: &str, now_ms: i64) -> Vec<ConsoleEffect> {
        let mut fx = Vec::new();
        let text = line.trim();
        if text.is_empty() {
            return fx;
        }
        // Running a command shows its result: whatever the log was scrolled to, it returns to the newest
        // line, as a terminal does.
        self.state.follow_tail();
        self.state.remember(text);
        if text == "clear" {
            self.state.clear();
            return fx;
        }
        self.state.push(vec![NewEntry::Cmd {
            text: text.to_owned(),
            at: now_ms,
        }]);
        if text == "help" {
            if self.state.commands_loaded {
                self.show_help();
            } else {
                fx.push(ConsoleEffect::FetchCommands {
                    epoch: self.epoch,
                    for_help: true,
                });
            }
            return fx;
        }
        if self.state.support == Support::Unsupported {
            self.state.push(vec![NewEntry::Note {
                tone: Tone::Error,
                note: Note::Unsupported,
            }]);
            return fx;
        }
        self.state.run_started();
        let run = self.next_run;
        self.next_run += 1;
        self.runs.insert(run, (self.epoch, ExecOutput::default()));
        fx.push(ConsoleEffect::StartExec {
            run,
            line: text.to_owned(),
        });
        fx
    }

    /// One NDJSON line of a running command. Frames of a cancelled or finished run are dropped.
    pub fn exec_frame(&mut self, run: u64, text: &str) {
        let Some((epoch, output)) = self.runs.get_mut(&run) else {
            return;
        };
        let (epoch, step) = (*epoch, output.on_frame(text));
        self.state.push(step.entries);
        if step.ended {
            self.runs.remove(&run);
            if epoch == self.epoch {
                self.state.run_ended();
            }
        }
    }

    /// The command's connection ended without an `exit` frame: the console never waits on a command that
    /// is gone.
    pub fn exec_closed(&mut self, run: u64) {
        let Some((epoch, mut output)) = self.runs.remove(&run) else {
            return;
        };
        self.state.push(output.flush());
        if epoch == self.epoch {
            self.state.run_ended();
        }
    }

    fn cancel_runs(&mut self, fx: &mut Vec<ConsoleEffect>) {
        let mut ids: Vec<u64> = self.runs.keys().copied().collect();
        ids.sort_unstable();
        for run in ids {
            if let Some((_, mut output)) = self.runs.remove(&run) {
                output.abandon();
            }
            fx.push(ConsoleEffect::CancelExec { run });
        }
        while self.state.running > 0 {
            self.state.run_ended();
        }
    }

    /// The user cancels the commands that are running (Ctrl+C, the Cancel button): their connections are
    /// closed, which cancels them on the daemon, and `note` (the console's own words) says so. Nothing
    /// happens when no command runs.
    pub fn cancel_running(&mut self, note: &str) -> Vec<ConsoleEffect> {
        let mut fx = Vec::new();
        if self.runs.is_empty() {
            return fx;
        }
        self.cancel_runs(&mut fx);
        self.state.push(vec![NewEntry::Note {
            tone: Tone::Info,
            note: Note::Text(note.to_owned()),
        }]);
        fx
    }

    pub fn stream_opened(&mut self, attempt: u64) {
        let mut fx = Vec::new();
        let actions = self.stream.on_open(attempt);
        self.apply_stream(actions, &mut fx);
    }

    pub fn stream_message(&mut self, attempt: u64, text: &str) -> Vec<ConsoleEffect> {
        let mut fx = Vec::new();
        if !self.stream.accepts(attempt) {
            return fx;
        }
        let frame = parse_log_frame(text);
        match self.replay.as_mut() {
            None => self.queue.push(frame),
            Some(held) => {
                let is_ready = matches!(frame, LogFrame::Ready { .. });
                held.push(frame);
                if is_ready {
                    let replay = self.replay.take().unwrap_or_default();
                    self.queue.extend(replay);
                }
            }
        }
        if !self.queue.is_empty() && !self.flush_pending {
            self.flush_pending = true;
            fx.push(ConsoleEffect::StartTimer {
                kind: TimerKind::Flush,
                ms: FLUSH_MS,
            });
        }
        fx
    }

    pub fn stream_closed(&mut self, attempt: u64) -> Vec<ConsoleEffect> {
        let mut fx = Vec::new();
        let actions = self.stream.on_close(attempt);
        self.apply_stream(actions, &mut fx);
        fx
    }

    pub fn timer_elapsed(&mut self, kind: TimerKind) -> Vec<ConsoleEffect> {
        let mut fx = Vec::new();
        match kind {
            TimerKind::Flush => {
                self.flush_pending = false;
                self.flush(&mut fx);
            }
            TimerKind::Backoff => {
                let actions = self.stream.on_timer(self.state.cursor);
                self.apply_stream(actions, &mut fx);
            }
        }
        fx
    }

    pub fn dispose(&mut self) -> Vec<ConsoleEffect> {
        let mut fx = Vec::new();
        let actions = self.stream.stop();
        self.apply_stream(actions, &mut fx);
        self.cancel_runs(&mut fx);
        if self.flush_pending {
            fx.push(ConsoleEffect::CancelTimer {
                kind: TimerKind::Flush,
            });
            self.flush_pending = false;
        }
        self.queue.clear();
        self.replay = None;
        fx
    }
}

#[cfg(test)]
#[path = "controller_tests/mod.rs"]
mod tests;
