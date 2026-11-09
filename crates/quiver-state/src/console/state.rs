//! The console's state: the buffer, the log cursor, the prompt draft and history. Pure data and the
//! rules for changing it; a store wraps it where it is wired to the view.

use super::commands::ConsoleCommand;
use super::entries::{ConsoleEntry, NewEntry, Note, StreamState, Support, Tone};
use super::frames::LogFrame;
use super::history::{push_history, step_history, Direction, HistoryCursor};
use super::ring::append_capped;
use super::versions::{supports_console, CoreVersions};

/// The most lines kept; the oldest go first. Matches the daemon's own ring.
pub const BUFFER_CAP: usize = 5000;
/// The most commands remembered for Up and Down.
pub const HISTORY_CAP: usize = 200;

#[derive(Clone, PartialEq, Debug)]
pub struct ConsoleState {
    pub open: bool,
    /// The connection every field below belongs to.
    pub connection_id: Option<String>,
    pub support: Support,
    pub versions: Option<CoreVersions>,
    pub commands: Vec<ConsoleCommand>,
    pub commands_loaded: bool,
    pub stream: StreamState,
    pub entries: Vec<ConsoleEntry>,
    next_id: u64,
    /// The highest log `seq` shown; the stream resumes after it.
    pub cursor: Option<u64>,
    pub expanded_id: Option<u64>,
    /// Bumped to bring the log back to its newest line (a command was run).
    pub tail: u64,
    pub draft: String,
    pub history: Vec<String>,
    pub history_cursor: HistoryCursor,
    /// Commands started and not yet finished.
    pub running: usize,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self {
            open: false,
            connection_id: None,
            support: Support::Unknown,
            versions: None,
            commands: vec![],
            commands_loaded: false,
            stream: StreamState::Idle,
            entries: vec![],
            next_id: 1,
            cursor: None,
            expanded_id: None,
            tail: 0,
            draft: String::new(),
            history: vec![],
            history_cursor: HistoryCursor::default(),
            running: 0,
        }
    }
}

fn numbered(entries: Vec<NewEntry>, from: u64) -> Vec<ConsoleEntry> {
    entries
        .into_iter()
        .zip(from..)
        .map(|(body, id)| ConsoleEntry { id, body })
        .collect()
}

impl ConsoleState {
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    /// Starts a clean slate for `connection_id`; a no-op when it is the one already shown. History is the
    /// user's, not the connection's, so it survives a switch (as do `open`, the ids and the draft's owner).
    pub fn adopt(&mut self, connection_id: &str) {
        if self.connection_id.as_deref() == Some(connection_id) {
            return;
        }
        *self = Self {
            open: self.open,
            connection_id: Some(connection_id.to_owned()),
            next_id: self.next_id,
            history: std::mem::take(&mut self.history),
            ..Self::default()
        };
    }

    pub fn set_versions(&mut self, versions: Option<CoreVersions>) {
        self.support = match &versions {
            None => Support::Unknown,
            Some(v) if supports_console(Some(v)) => Support::Supported,
            Some(_) => Support::Unsupported,
        };
        self.versions = versions;
    }

    pub fn set_commands(&mut self, commands: Vec<ConsoleCommand>) {
        self.commands = commands;
        self.commands_loaded = true;
    }

    pub fn set_stream(&mut self, stream: StreamState) {
        self.stream = stream;
    }

    /// Folds frames from the log stream into the buffer. Hand it a whole replay, `ready` included, in one
    /// call: a `ready` that says `reset` (the daemon restarted) discards the daemon lines already held
    /// before the replay is shown.
    pub fn ingest(&mut self, frames: &[LogFrame]) {
        let reset = frames
            .iter()
            .any(|f| matches!(f, LogFrame::Ready { reset: true, .. }));
        let mut cursor = if reset { None } else { self.cursor };
        let mut added = Vec::new();
        if reset {
            added.push(NewEntry::Note {
                tone: Tone::Info,
                note: Note::Restarted,
            });
        }
        for frame in frames {
            match frame {
                // A reconnect replays from the cursor, but a record can still arrive twice (a replay
                // racing the live feed); the cursor is what makes that harmless.
                LogFrame::Log(record) => {
                    if cursor.is_some_and(|c| record.seq <= c) {
                        continue;
                    }
                    cursor = Some(record.seq);
                    added.push(NewEntry::Log(record.clone()));
                }
                // Nothing to show: it only ends the replay, and `reset` was read above.
                LogFrame::Ready { .. } => {}
                LogFrame::Raw(text) => added.push(NewEntry::Raw(text.clone())),
            }
        }
        // A restarted daemon's lines are another process's: keep the user's own commands and what they
        // printed, drop the old log.
        let kept: Vec<ConsoleEntry> = if reset {
            self.entries
                .iter()
                .filter(|e| !matches!(e.body, NewEntry::Log(_)))
                .cloned()
                .collect()
        } else {
            std::mem::take(&mut self.entries)
        };
        self.cursor = cursor;
        if reset {
            self.expanded_id = None;
        }
        let count = added.len() as u64;
        self.entries = append_capped(&kept, &numbered(added, self.next_id), BUFFER_CAP);
        self.next_id += count;
    }

    pub fn push(&mut self, entries: Vec<NewEntry>) {
        if entries.is_empty() {
            return;
        }
        let count = entries.len() as u64;
        let numbered = numbered(entries, self.next_id);
        self.entries = append_capped(&self.entries, &numbered, BUFFER_CAP);
        self.next_id += count;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.expanded_id = None;
    }

    /// Asks the log to jump to, and keep following, its newest line.
    pub fn follow_tail(&mut self) {
        self.tail += 1;
    }

    pub fn toggle_expanded(&mut self, id: u64) {
        self.expanded_id = if self.expanded_id == Some(id) {
            None
        } else {
            Some(id)
        };
    }

    pub fn set_draft(&mut self, draft: &str) {
        self.draft = draft.to_owned();
        self.history_cursor = HistoryCursor::default();
    }

    pub fn remember(&mut self, line: &str) {
        self.history = push_history(&self.history, line, HISTORY_CAP);
        self.history_cursor = HistoryCursor::default();
        self.draft.clear();
    }

    pub fn step_history(&mut self, direction: Direction) {
        let step = step_history(&self.history, &self.history_cursor, &self.draft, direction);
        self.history_cursor = step.cursor;
        self.draft = step.text;
    }

    pub fn run_started(&mut self) {
        self.running += 1;
    }

    pub fn run_ended(&mut self) {
        self.running = self.running.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::super::frames::{LogLevel, LogRecord};
    use super::*;
    use crate::console::versions::CONSOLE_FEATURE;

    fn record(seq: u64) -> LogFrame {
        LogFrame::Log(LogRecord {
            seq,
            iso: String::new(),
            level: LogLevel::Info,
            component: String::new(),
            msg: format!("m{seq}"),
            fields: vec![],
        })
    }

    fn seqs(state: &ConsoleState) -> Vec<u64> {
        state
            .entries
            .iter()
            .filter_map(|e| match &e.body {
                NewEntry::Log(r) => Some(r.seq),
                _ => None,
            })
            .collect()
    }

    fn cmd(text: &str) -> NewEntry {
        NewEntry::Cmd {
            text: text.into(),
            at: 0,
        }
    }

    #[test]
    fn ingest_advances_the_cursor_and_drops_records_at_or_below_it() {
        let mut s = ConsoleState::default();
        s.ingest(&[
            record(1),
            record(2),
            LogFrame::Ready {
                seq: 2,
                reset: false,
            },
        ]);
        assert_eq!((seqs(&s), s.cursor), (vec![1, 2], Some(2)));
        s.ingest(&[record(2), record(3)]);
        assert_eq!((seqs(&s), s.cursor), (vec![1, 2, 3], Some(3)));
    }

    #[test]
    fn a_reset_discards_the_old_daemons_lines_but_keeps_the_users_own() {
        let mut s = ConsoleState::default();
        s.ingest(&[record(1), record(2)]);
        s.push(vec![cmd("arrow list")]);
        s.toggle_expanded(1);
        s.ingest(&[
            record(1),
            LogFrame::Ready {
                seq: 1,
                reset: true,
            },
        ]);
        assert_eq!(
            seqs(&s),
            [1],
            "the replay of the new process, from its own seq 1"
        );
        assert_eq!(s.cursor, Some(1));
        assert_eq!(s.expanded_id, None);
        assert!(s
            .entries
            .iter()
            .any(|e| matches!(e.body, NewEntry::Cmd { .. })));
        assert!(s.entries.iter().any(|e| matches!(
            e.body,
            NewEntry::Note {
                note: Note::Restarted,
                ..
            }
        )));
    }

    #[test]
    fn raw_frames_are_shown_as_text() {
        let mut s = ConsoleState::default();
        s.ingest(&[LogFrame::Raw("odd".into())]);
        assert_eq!(s.entries[0].body, NewEntry::Raw("odd".into()));
    }

    #[test]
    fn ids_never_repeat_and_the_buffer_keeps_the_newest_cap() {
        let mut s = ConsoleState::default();
        let many: Vec<NewEntry> = (0..BUFFER_CAP + 10).map(|i| cmd(&i.to_string())).collect();
        s.push(many);
        s.push(vec![cmd("last")]);
        assert_eq!(s.entries.len(), BUFFER_CAP);
        assert_eq!(
            s.entries.last().map(|e| e.id),
            Some((BUFFER_CAP + 11) as u64)
        );
        s.push(vec![]);
        assert_eq!(s.entries.len(), BUFFER_CAP);
    }

    #[test]
    fn adopting_a_new_connection_resets_everything_but_history_and_openness() {
        let mut s = ConsoleState::default();
        s.adopt("local");
        s.set_open(true);
        s.ingest(&[record(5)]);
        s.remember("arrow list");
        s.set_draft("half typed");
        s.adopt("local");
        assert_eq!(s.cursor, Some(5), "adopting the same connection is a no-op");
        s.adopt("remote-1");
        assert!(s.open);
        assert_eq!((s.entries.len(), s.cursor, s.draft.as_str()), (0, None, ""));
        assert_eq!(s.history, ["arrow list"]);
        assert_eq!(s.connection_id.as_deref(), Some("remote-1"));
    }

    #[test]
    fn support_follows_what_versions_said() {
        let mut s = ConsoleState::default();
        s.set_versions(Some(CoreVersions {
            features: vec![CONSOLE_FEATURE.into()],
            ..CoreVersions::default()
        }));
        assert_eq!(s.support, Support::Supported);
        s.set_versions(Some(CoreVersions::default()));
        assert_eq!(s.support, Support::Unsupported);
        s.set_versions(None);
        assert_eq!(s.support, Support::Unknown);
    }

    #[test]
    fn history_walks_through_the_draft() {
        let mut s = ConsoleState::default();
        s.remember("one");
        s.remember("two");
        s.set_draft("typing");
        s.step_history(Direction::Up);
        assert_eq!(s.draft, "two");
        s.step_history(Direction::Up);
        assert_eq!(s.draft, "one");
        s.step_history(Direction::Down);
        s.step_history(Direction::Down);
        assert_eq!(s.draft, "typing");
    }

    #[test]
    fn clear_toggle_tail_and_run_counting() {
        let mut s = ConsoleState::default();
        s.push(vec![cmd("a")]);
        s.toggle_expanded(1);
        assert_eq!(s.expanded_id, Some(1));
        s.toggle_expanded(1);
        assert_eq!(s.expanded_id, None);
        s.clear();
        assert!(s.entries.is_empty());
        s.follow_tail();
        assert_eq!(s.tail, 1);
        s.toggle();
        assert!(s.open);
        s.run_started();
        s.run_ended();
        s.run_ended();
        assert_eq!(s.running, 0);
    }
}
