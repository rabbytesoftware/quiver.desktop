//! Decoding what `GET /v0/console/logs` sends, defensively.
//!
//! Every frame is one JSON text frame (see `docs/console-spec.md`). A frame this file does not understand
//! is never an error: it becomes a `Raw` frame carrying its text, so a newer daemon (or a bug on either
//! side) shows up in the console as odd output rather than as a crash.

use serde_json::{Map, Value};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldKind {
    Text,
    Number,
    Duration,
    Bool,
    Path,
    Error,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LogField {
    pub key: String,
    pub value: String,
    pub kind: FieldKind,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LogRecord {
    pub seq: u64,
    /// RFC 3339 as the daemon sent it.
    pub iso: String,
    pub level: LogLevel,
    pub component: String,
    pub msg: String,
    pub fields: Vec<LogField>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LogFrame {
    Log(LogRecord),
    /// The end of the replay. `reset` means the daemon restarted: what the client holds belongs to another
    /// process.
    Ready {
        seq: u64,
        reset: bool,
    },
    Raw(String),
}

/// `^(\d+(\.\d+)?(ns|us|µs|ms|s|m|h))+$`.
fn is_duration(value: &str) -> bool {
    let mut rest = value;
    if rest.is_empty() {
        return false;
    }
    while !rest.is_empty() {
        let int_len = rest.bytes().take_while(u8::is_ascii_digit).count();
        if int_len == 0 {
            return false;
        }
        rest = &rest[int_len..];
        if let Some(frac) = rest.strip_prefix('.') {
            let n = frac.bytes().take_while(u8::is_ascii_digit).count();
            if n == 0 {
                return false;
            }
            rest = &frac[n..];
        }
        // `ms` must win over `m`, and `µs` is multi-byte: try the longest unit first.
        let Some(unit) = ["ms", "ns", "us", "µs", "s", "m", "h"]
            .into_iter()
            .find(|u| rest.starts_with(u))
        else {
            return false;
        };
        rest = &rest[unit.len()..];
    }
    true
}

/// `^(~|/|[A-Za-z]:[\\/]|github\.com/)`.
fn is_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.starts_with('~')
        || value.starts_with('/')
        || value.starts_with("github.com/")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/'))
}

/// How a field's value is styled. Decided by its JSON type first, then by its key and shape.
pub fn field_kind(key: &str, value: &Value) -> FieldKind {
    match value {
        Value::Bool(_) => FieldKind::Bool,
        Value::Number(_) => FieldKind::Number,
        _ if key == "err" || key == "error" => FieldKind::Error,
        Value::String(s) if is_duration(s) => FieldKind::Duration,
        Value::String(s) if is_path(s) => FieldKind::Path,
        _ => FieldKind::Text,
    }
}

fn display(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        other => other.to_string(),
    }
}

pub fn to_level(raw: Option<&Value>) -> LogLevel {
    match raw
        .and_then(Value::as_str)
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("debug" | "trace") => LogLevel::Debug,
        Some("warn" | "warning") => LogLevel::Warn,
        Some("error" | "fatal" | "panic") => LogLevel::Error,
        _ => LogLevel::Info,
    }
}

fn parse_log(frame: &Map<String, Value>) -> Option<LogFrame> {
    let seq = frame.get("seq").and_then(number_u64)?;
    let msg = frame.get("msg").and_then(Value::as_str)?;
    let text = |key: &str| {
        frame
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let fields = frame
        .get("fields")
        .and_then(Value::as_object)
        .map(|fields| {
            fields
                .iter()
                .map(|(key, value)| LogField {
                    key: key.clone(),
                    value: display(value),
                    kind: field_kind(key, value),
                })
                .collect()
        })
        .unwrap_or_default();
    Some(LogFrame::Log(LogRecord {
        seq,
        iso: text("time"),
        level: to_level(frame.get("level")),
        component: text("component"),
        msg: msg.to_owned(),
        fields,
    }))
}

fn number_u64(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_f64()
            .filter(|f| *f >= 0.0 && f.fract() == 0.0)
            .map(|f| f as u64)
    })
}

/// Decodes one frame from the text the socket delivered.
pub fn parse_log_frame(data: &str) -> LogFrame {
    let Ok(Value::Object(frame)) = serde_json::from_str::<Value>(data) else {
        return LogFrame::Raw(data.to_owned());
    };
    let raw = || LogFrame::Raw(data.to_owned());
    match frame.get("type").and_then(Value::as_str) {
        Some("log") => parse_log(&frame).unwrap_or_else(raw),
        Some("ready") => match frame.get("seq").and_then(number_u64) {
            Some(seq) => LogFrame::Ready {
                seq,
                reset: frame.get("reset") == Some(&Value::Bool(true)),
            },
            None => raw(),
        },
        _ => raw(),
    }
}

/// `HH:mm:ss.SSS` at a UTC offset in minutes (the viewer's zone); the record's own text when it is not a
/// date.
pub fn format_log_time(iso: &str, offset_minutes: i32) -> String {
    let Some(ms) = parse_rfc3339_ms(iso) else {
        return iso.to_owned();
    };
    let local = ms + i64::from(offset_minutes) * 60_000;
    let day_ms = local.rem_euclid(86_400_000);
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        day_ms / 3_600_000,
        day_ms % 3_600_000 / 60_000,
        day_ms % 60_000 / 1000,
        day_ms % 1000
    )
}

/// Epoch milliseconds of an RFC 3339 timestamp; fractional seconds beyond milliseconds are truncated.
pub fn parse_rfc3339_ms(iso: &str) -> Option<i64> {
    let (date, rest) = iso.split_once(['T', 't', ' '])?;
    let mut d = date.split('-');
    let (year, month, day): (i64, i64, i64) = (
        d.next()?.parse().ok()?,
        d.next()?.parse().ok()?,
        d.next()?.parse().ok()?,
    );
    if d.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let tz_at = rest.find(['Z', 'z', '+', '-']);
    let (clock, zone) = tz_at.map_or((rest, ""), |at| rest.split_at(at));
    let mut c = clock.split(':');
    let (h, m): (i64, i64) = (c.next()?.parse().ok()?, c.next()?.parse().ok()?);
    let sec_text = c.next().unwrap_or("0");
    let (sec, frac) = sec_text.split_once('.').unwrap_or((sec_text, ""));
    let s: i64 = sec.parse().ok()?;
    let millis: i64 = format!("{frac:0<3}")[..3].parse().ok()?;
    if h > 23 || m > 59 || s > 60 {
        return None;
    }
    let offset = match zone {
        "" | "Z" | "z" => 0,
        z => {
            let sign = if z.starts_with('-') { -1 } else { 1 };
            let (oh, om) = z[1..].split_once(':').unwrap_or((&z[1..], "0"));
            sign * (oh.parse::<i64>().ok()? * 60 + om.parse::<i64>().ok()?)
        }
    };
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 24 + h) * 60 + m - offset) * 60_000 + s * 1000 + millis)
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct JsonLine {
    pub key: String,
    /// Strings are quoted, so the expanded view reads as JSON.
    pub value: String,
    pub kind: FieldKind,
}

fn quoted(text: &str) -> String {
    Value::String(text.to_owned()).to_string()
}

/// The record as the daemon would print it, one entry per key: what a click on a line reveals.
pub fn json_lines(record: &LogRecord) -> Vec<JsonLine> {
    let line = |key: &str, value: String, kind| JsonLine {
        key: key.to_owned(),
        value,
        kind,
    };
    let mut lines = vec![
        line("time", quoted(&record.iso), FieldKind::Text),
        line(
            "level",
            quoted(match record.level {
                LogLevel::Debug => "debug",
                LogLevel::Info => "info",
                LogLevel::Warn => "warn",
                LogLevel::Error => "error",
            }),
            FieldKind::Text,
        ),
    ];
    if !record.component.is_empty() {
        lines.push(line(
            "component",
            quoted(&record.component),
            FieldKind::Text,
        ));
    }
    lines.push(line("msg", quoted(&record.msg), FieldKind::Text));
    for field in &record.fields {
        let value = if matches!(field.kind, FieldKind::Number | FieldKind::Bool) {
            field.value.clone()
        } else {
            quoted(&field.value)
        };
        lines.push(line(&field.key, value, field.kind));
    }
    lines
}

/// The output of one console command.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ExecFrame {
    Out { stderr: bool, data: String },
    Exit { code: i64, error: String },
    Error { status: u16, message: String },
    Raw(String),
}

pub fn parse_exec_frame(text: &str) -> ExecFrame {
    let Ok(Value::Object(frame)) = serde_json::from_str::<Value>(text) else {
        return ExecFrame::Raw(text.to_owned());
    };
    let raw = || ExecFrame::Raw(text.to_owned());
    let string = |key: &str| frame.get(key).and_then(Value::as_str).map(str::to_owned);
    match frame.get("type").and_then(Value::as_str) {
        Some("out") => match string("data") {
            Some(data) => ExecFrame::Out {
                stderr: frame.get("stream").and_then(Value::as_str) == Some("stderr"),
                data,
            },
            None => raw(),
        },
        Some("exit") => match frame.get("code").and_then(Value::as_i64) {
            Some(code) => ExecFrame::Exit {
                code,
                error: string("error").unwrap_or_default(),
            },
            None => raw(),
        },
        Some("error") => ExecFrame::Error {
            status: frame
                .get("status")
                .and_then(Value::as_u64)
                .and_then(|s| u16::try_from(s).ok())
                .unwrap_or(0),
            message: string("message").unwrap_or_default(),
        },
        _ => raw(),
    }
}

#[cfg(test)]
#[path = "frames_tests.rs"]
mod tests;
