//! One console entry as the spans a row draws (docs/console-spec.md section 5): a log line is
//! `time  LEVEL  message key=value ...` where only warn and error colour their level and message, and a
//! value is coloured by its kind. The view maps a [`SpanTone`] to a palette colour.

use super::entries::{note_text, ConsoleEntry, NewEntry, Stream, Tone};
use super::frames::{format_log_time, FieldKind, LogLevel, LogRecord};
use crate::i18n::Translator;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpanTone {
    Normal,
    Muted,
    Warn,
    Error,
    Ok,
    Number,
    Bool,
    /// Drawn underlined.
    Path,
    /// The `>` of a typed command.
    Prompt,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Span {
    pub text: String,
    pub tone: SpanTone,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Line {
    pub id: u64,
    pub spans: Vec<Span>,
    /// A log record opens as JSON on click.
    pub expandable: bool,
}

fn span(
    text: impl Into<String>,
    tone: SpanTone,
) -> Span {
    Span {
        text: text.into(),
        tone,
    }
}

fn level_label(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    }
}

fn field_tone(kind: FieldKind) -> SpanTone {
    match kind {
        FieldKind::Text => SpanTone::Normal,
        FieldKind::Number | FieldKind::Duration => SpanTone::Number,
        FieldKind::Bool => SpanTone::Ok,
        FieldKind::Path => SpanTone::Path,
        FieldKind::Error => SpanTone::Error,
    }
}

fn log_spans(
    record: &LogRecord,
    offset_minutes: i32,
) -> Vec<Span> {
    let emphasis = match record.level {
        LogLevel::Warn => SpanTone::Warn,
        LogLevel::Error => SpanTone::Error,
        _ => SpanTone::Normal,
    };
    let level_tone = if emphasis == SpanTone::Normal {
        SpanTone::Muted
    } else {
        emphasis
    };
    let mut spans = vec![
        span(
            format_log_time(&record.iso, offset_minutes),
            SpanTone::Muted,
        ),
        span("  ", SpanTone::Normal),
        span(format!("{:<5}", level_label(record.level)), level_tone),
        span("  ", SpanTone::Normal),
        span(record.msg.clone(), emphasis),
    ];
    for field in &record.fields {
        spans.push(span(format!(" {}=", field.key), SpanTone::Muted));
        spans.push(span(field.value.clone(), field_tone(field.kind)));
    }
    spans
}

fn tone_of(tone: Tone) -> SpanTone {
    match tone {
        Tone::Info => SpanTone::Muted,
        Tone::Ok => SpanTone::Ok,
        Tone::Error => SpanTone::Error,
    }
}

/// `offset_minutes` is the viewer's UTC offset.
pub fn display_line(
    entry: &ConsoleEntry,
    tr: &Translator,
    offset_minutes: i32,
) -> Line {
    let (spans, expandable) = match &entry.body {
        NewEntry::Log(record) => (log_spans(record, offset_minutes), true),
        NewEntry::Raw(text) => (vec![span(text.clone(), SpanTone::Muted)], false),
        NewEntry::Cmd { text, .. } => (
            vec![
                span("> ", SpanTone::Prompt),
                span(text.clone(), SpanTone::Normal),
            ],
            false,
        ),
        NewEntry::Out { stream, text } => {
            let tone = match stream {
                Stream::Stdout => SpanTone::Normal,
                Stream::Stderr => SpanTone::Warn,
            };
            (vec![span(text.clone(), tone)], false)
        }
        NewEntry::Note { tone, note } => (vec![span(note_text(note, tr), tone_of(*tone))], false),
    };
    Line {
        id: entry.id,
        spans,
        expandable,
    }
}

#[cfg(test)]
mod tests {
    use super::super::entries::Note;
    use super::super::frames::LogField;
    use super::*;

    fn log(level: LogLevel) -> ConsoleEntry {
        ConsoleEntry {
            id: 7,
            body: NewEntry::Log(LogRecord {
                seq: 1,
                iso: "2026-10-04T14:02:14.390Z".into(),
                level,
                component: "release".into(),
                msg: "channel lookup slow".into(),
                fields: vec![
                    LogField {
                        key: "took".into(),
                        value: "1.8s".into(),
                        kind: FieldKind::Duration,
                    },
                    LogField {
                        key: "err".into(),
                        value: "boom".into(),
                        kind: FieldKind::Error,
                    },
                ],
            }),
        }
    }

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn a_log_line_reads_time_level_message_then_fields() {
        let line = display_line(&log(LogLevel::Info), &Translator::default(), 0);
        assert_eq!(
            text(&line),
            "14:02:14.390  INFO   channel lookup slow took=1.8s err=boom"
        );
        assert!(line.expandable);
    }

    #[test]
    fn only_warn_and_error_colour_their_level_and_message() {
        let tr = Translator::default();
        let info = display_line(&log(LogLevel::Info), &tr, 0);
        assert_eq!(info.spans[2].tone, SpanTone::Muted);
        assert_eq!(info.spans[4].tone, SpanTone::Normal);
        let warn = display_line(&log(LogLevel::Warn), &tr, 0);
        assert_eq!(
            (warn.spans[2].tone, warn.spans[4].tone),
            (SpanTone::Warn, SpanTone::Warn)
        );
        let error = display_line(&log(LogLevel::Error), &tr, 0);
        assert_eq!(error.spans[4].tone, SpanTone::Error);
    }

    #[test]
    fn values_are_coloured_by_kind_and_err_is_red() {
        let line = display_line(&log(LogLevel::Info), &Translator::default(), 0);
        let tones: Vec<_> = line.spans.iter().map(|s| s.tone).collect();
        assert_eq!(
            &tones[tones.len() - 3..],
            [SpanTone::Number, SpanTone::Muted, SpanTone::Error]
        );
    }

    #[test]
    fn commands_output_and_notes_are_not_expandable() {
        let tr = Translator::default();
        let cmd = ConsoleEntry {
            id: 1,
            body: NewEntry::Cmd {
                text: "arrow list".into(),
                at: 0,
            },
        };
        assert_eq!(text(&display_line(&cmd, &tr, 0)), "> arrow list");
        let note = ConsoleEntry {
            id: 2,
            body: NewEntry::Note {
                tone: Tone::Error,
                note: Note::Refused {
                    status: 403,
                    message: "no".into(),
                },
            },
        };
        let line = display_line(&note, &tr, 0);
        assert_eq!(
            (text(&line).as_str(), line.spans[0].tone, line.expandable),
            ("Refused (403): no", SpanTone::Error, false)
        );
    }
}
