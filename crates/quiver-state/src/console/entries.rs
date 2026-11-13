use super::frames::LogRecord;
use crate::i18n::{en::console, Params, Translator};

/// Something the console says itself, as data: the view turns it into words.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Note {
    Restarted,
    Exit { code: i64, error: String },
    Refused { status: u16, message: String },
    Unsupported,
    HelpUnavailable,
    Text(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Info,
    Ok,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// An entry before it has been given an id.
#[derive(Clone, PartialEq, Debug)]
pub enum NewEntry {
    Log(LogRecord),
    Raw(String),
    Cmd { text: String, at: i64 },
    Out { stream: Stream, text: String },
    Note { tone: Tone, note: Note },
}

#[derive(Clone, PartialEq, Debug)]
pub struct ConsoleEntry {
    pub id: u64,
    pub body: NewEntry,
}

/// Whether the connected daemon has a console. `Unknown` until `/versions` has answered (or when it cannot
/// be reached), so the UI never claims "unsupported" about a daemon it has simply not heard from.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Support {
    #[default]
    Unknown,
    Supported,
    Unsupported,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum StreamState {
    #[default]
    Idle,
    Connecting,
    Live,
    Reconnecting,
}

/// What the console says about itself, in words.
pub fn note_text(
    note: &Note,
    tr: &Translator,
) -> String {
    match note {
        Note::Restarted => tr.t(console::NOTE_RESTARTED),
        Note::Exit { code, error } if error.is_empty() => tr.t_with(
            console::NOTE_EXIT,
            &Params::new().with("code", code.to_string()),
        ),
        Note::Exit { code, error } => tr.t_with(
            console::NOTE_EXIT_WITH_ERROR,
            &Params::new()
                .with("code", code.to_string())
                .with("error", error.as_str()),
        ),
        Note::Refused { status, message } => tr.t_with(
            console::NOTE_REFUSED,
            &Params::new()
                .with("status", u32::from(*status))
                .with("message", message.as_str()),
        ),
        Note::Unsupported => tr.t(console::NOTE_UNSUPPORTED),
        Note::HelpUnavailable => tr.t(console::NOTE_HELP_UNAVAILABLE),
        Note::Text(text) => text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_read_in_words() {
        let tr = Translator::default();
        assert_eq!(note_text(&Note::Restarted, &tr), "Daemon restarted");
        assert_eq!(
            note_text(
                &Note::Exit {
                    code: 0,
                    error: String::new()
                },
                &tr
            ),
            "Exited with code 0"
        );
        assert!(note_text(
            &Note::Exit {
                code: 2,
                error: "bad usage".into()
            },
            &tr
        )
        .contains("bad usage"));
        assert_eq!(
            note_text(
                &Note::Refused {
                    status: 403,
                    message: "nope".into()
                },
                &tr
            ),
            "Refused (403): nope"
        );
        assert_eq!(note_text(&Note::Text("hi".into()), &tr), "hi");
    }
}
