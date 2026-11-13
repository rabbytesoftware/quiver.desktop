//! One running console command: turns its NDJSON frames into entries. A line split across frames is
//! joined, ANSI escapes are stripped, and an unterminated last line is flushed when the command ends.

use super::entries::{NewEntry, Note, Stream, Tone};
use super::frames::{parse_exec_frame, ExecFrame};

/// Removes `ESC [ <digits ; ?>* <letter>` sequences, and one trailing carriage return.
fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' || chars.peek() != Some(&'[') {
            out.push(c);
            continue;
        }
        let mut probe = chars.clone();
        probe.next();
        while probe
            .peek()
            .is_some_and(|p| p.is_ascii_digit() || matches!(p, ';' | '?'))
        {
            probe.next();
        }
        if probe.peek().is_some_and(char::is_ascii_alphabetic) {
            probe.next();
            chars = probe;
        } else {
            out.push(c);
        }
    }
    out.strip_suffix('\r').map_or(out.clone(), str::to_owned)
}

/// What a frame did to the run.
#[derive(Debug, Default, PartialEq)]
pub struct ExecStep {
    pub entries: Vec<NewEntry>,
    /// The run is over; the owner stops counting it as running.
    pub ended: bool,
}

#[derive(Debug, Default)]
pub struct ExecOutput {
    stdout: String,
    stderr: String,
    finished: bool,
}

impl ExecOutput {
    pub fn finished(&self) -> bool {
        self.finished
    }

    fn lines(
        &mut self,
        stream: Stream,
        data: &str,
    ) -> Vec<NewEntry> {
        let pending = if stream == Stream::Stdout {
            &mut self.stdout
        } else {
            &mut self.stderr
        };
        let joined = format!("{pending}{data}");
        let mut parts: Vec<&str> = joined.split('\n').collect();
        *pending = parts.pop().unwrap_or_default().to_owned();
        parts
            .into_iter()
            .map(|text| NewEntry::Out {
                stream,
                text: clean(text),
            })
            .collect()
    }

    /// An unterminated last line is still output.
    pub fn flush(&mut self) -> Vec<NewEntry> {
        let mut out = Vec::new();
        for (stream, pending) in [
            (Stream::Stdout, &mut self.stdout),
            (Stream::Stderr, &mut self.stderr),
        ] {
            if !pending.is_empty() {
                out.push(NewEntry::Out {
                    stream,
                    text: clean(pending),
                });
                pending.clear();
            }
        }
        out
    }

    pub fn on_frame(
        &mut self,
        text: &str,
    ) -> ExecStep {
        if self.finished {
            return ExecStep::default();
        }
        let note = |note| NewEntry::Note {
            tone: Tone::Error,
            note,
        };
        match parse_exec_frame(text) {
            ExecFrame::Out { stderr, data } => ExecStep {
                entries: self.lines(
                    if stderr {
                        Stream::Stderr
                    } else {
                        Stream::Stdout
                    },
                    &data,
                ),
                ended: false,
            },
            ExecFrame::Exit { code, error } => {
                let mut entries = self.flush();
                if code != 0 {
                    entries.push(note(Note::Exit { code, error }));
                }
                self.finished = true;
                ExecStep {
                    entries,
                    ended: true,
                }
            }
            ExecFrame::Error { status, message } => {
                let mut entries = self.flush();
                entries.push(note(if status > 0 {
                    Note::Refused { status, message }
                } else {
                    Note::Text(message)
                }));
                self.finished = true;
                ExecStep {
                    entries,
                    ended: true,
                }
            }
            ExecFrame::Raw(text) => ExecStep {
                entries: vec![NewEntry::Raw(text)],
                ended: false,
            },
        }
    }

    /// The run was cancelled or its connection left: nothing more is shown.
    pub fn abandon(&mut self) {
        self.finished = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(
        stream: Stream,
        text: &str,
    ) -> NewEntry {
        NewEntry::Out {
            stream,
            text: text.into(),
        }
    }

    fn frame(
        stream: &str,
        data: &str,
    ) -> String {
        serde_json::json!({"type":"out","stream":stream,"data":data}).to_string()
    }

    #[test]
    fn output_is_shown_a_line_at_a_time_joining_a_line_split_across_frames() {
        let mut run = ExecOutput::default();
        assert!(run.on_frame(&frame("stdout", "resol")).entries.is_empty());
        let step = run.on_frame(&frame("stdout", "ving\nsecond\nthi"));
        assert_eq!(
            step.entries,
            [
                out(Stream::Stdout, "resolving"),
                out(Stream::Stdout, "second")
            ]
        );
        assert_eq!(
            run.on_frame(&frame("stdout", "rd\n")).entries,
            [out(Stream::Stdout, "third")]
        );
    }

    #[test]
    fn stdout_and_stderr_are_kept_apart() {
        let mut run = ExecOutput::default();
        run.on_frame(&frame("stdout", "a"));
        let step = run.on_frame(&frame("stderr", "err\n"));
        assert_eq!(step.entries, [out(Stream::Stderr, "err")]);
        assert_eq!(run.flush(), [out(Stream::Stdout, "a")]);
    }

    #[test]
    fn a_blank_output_line_is_kept() {
        let mut run = ExecOutput::default();
        assert_eq!(run.on_frame(&frame("stdout", "a\n\nb\n")).entries.len(), 3);
    }

    #[test]
    fn terminal_escapes_and_carriage_returns_are_stripped() {
        let mut run = ExecOutput::default();
        let step = run.on_frame(&frame(
            "stdout",
            "\u{1b}[31mred\u{1b}[0m ok\r\n\u{1b}[?25lx\n\u{1b}[12\n",
        ));
        let texts: Vec<_> = step.entries.iter().map(|e| format!("{e:?}")).collect();
        assert!(texts[0].contains("red ok") && !texts[0].contains('\r'));
        assert!(texts[1].contains("\"x\""));
        assert!(
            texts[2].contains("[12"),
            "an incomplete sequence is left alone"
        );
    }

    #[test]
    fn exit_flushes_an_unterminated_line_and_a_clean_exit_adds_nothing() {
        let mut run = ExecOutput::default();
        run.on_frame(&frame("stdout", "tail"));
        let step = run.on_frame(r#"{"type":"exit","code":0,"error":""}"#);
        assert_eq!(step.entries, [out(Stream::Stdout, "tail")]);
        assert!(step.ended && run.finished());
    }

    #[test]
    fn a_failure_adds_an_error_note_with_the_code() {
        let mut run = ExecOutput::default();
        let step = run.on_frame(r#"{"type":"exit","code":2,"error":"usage"}"#);
        assert_eq!(
            step.entries,
            [NewEntry::Note {
                tone: Tone::Error,
                note: Note::Exit {
                    code: 2,
                    error: "usage".into()
                }
            }]
        );
    }

    #[test]
    fn a_refusal_is_reported_in_the_daemons_words_and_a_dead_daemon_as_plain_text() {
        let refused = ExecOutput::default()
            .on_frame(r#"{"type":"error","status":403,"message":"not available"}"#);
        assert_eq!(
            refused.entries,
            [NewEntry::Note {
                tone: Tone::Error,
                note: Note::Refused {
                    status: 403,
                    message: "not available".into()
                }
            }]
        );
        let down = ExecOutput::default()
            .on_frame(r#"{"type":"error","status":0,"message":"unreachable"}"#);
        assert_eq!(
            down.entries,
            [NewEntry::Note {
                tone: Tone::Error,
                note: Note::Text("unreachable".into())
            }]
        );
        assert!(refused.ended && down.ended);
    }

    #[test]
    fn an_undecodable_frame_is_shown_raw_and_frames_after_the_end_are_ignored() {
        let mut run = ExecOutput::default();
        assert_eq!(
            run.on_frame("garbage").entries,
            [NewEntry::Raw("garbage".into())]
        );
        run.on_frame(r#"{"type":"exit","code":0}"#);
        assert_eq!(
            run.on_frame(&frame("stdout", "late\n")),
            ExecStep::default()
        );
    }

    #[test]
    fn an_abandoned_run_shows_nothing_more() {
        let mut run = ExecOutput::default();
        run.abandon();
        assert_eq!(run.on_frame(&frame("stdout", "x\n")), ExecStep::default());
    }
}
