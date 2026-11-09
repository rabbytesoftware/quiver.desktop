use super::*;
use serde_json::json;

fn log(frame: &Value) -> LogRecord {
    match parse_log_frame(&frame.to_string()) {
        LogFrame::Log(r) => r,
        other => panic!("expected a log frame, got {other:?}"),
    }
}

#[test]
fn a_log_frame_decodes_with_its_fields_styled_by_type_key_and_shape() {
    let r = log(
        &json!({"type":"log","seq":412,"time":"2026-10-04T14:02:14.390123Z","level":"warn",
            "component":"release","msg":"channel lookup slow",
            "fields":{"ns":"github.com/char2cs/crowbar","took":"1.8s","retry":1,"ok":true,"err":"boom","name":"x"}}),
    );
    assert_eq!(
        (r.seq, r.level, r.component.as_str()),
        (412, LogLevel::Warn, "release")
    );
    let kind = |key: &str| r.fields.iter().find(|f| f.key == key).map(|f| f.kind);
    assert_eq!(kind("ns"), Some(FieldKind::Path));
    assert_eq!(kind("took"), Some(FieldKind::Duration));
    assert_eq!(kind("retry"), Some(FieldKind::Number));
    assert_eq!(kind("ok"), Some(FieldKind::Bool));
    assert_eq!(kind("err"), Some(FieldKind::Error));
    assert_eq!(kind("name"), Some(FieldKind::Text));
}

#[test]
fn durations_and_paths_are_told_apart_from_prose() {
    for yes in ["1.8s", "250ms", "1h30m", "12µs", "3ns"] {
        assert!(is_duration(yes), "{yes}");
    }
    for no in ["", "s", "1", "1.s", "5 s", "1d", "abc"] {
        assert!(!is_duration(no), "{no}");
    }
    for yes in ["/etc/x", "~/x", "C:\\x", "d:/x", "github.com/a/b"] {
        assert!(is_path(yes), "{yes}");
    }
    for no in ["etc/x", "C:x", "gitlab.com/a", ""] {
        assert!(!is_path(no), "{no}");
    }
}

#[test]
fn levels_are_normalised_and_unknown_ones_read_as_info() {
    let level = |v: Value| to_level(Some(&v));
    assert_eq!(level(json!("TRACE")), LogLevel::Debug);
    assert_eq!(level(json!("Warning")), LogLevel::Warn);
    for e in ["fatal", "panic", "error"] {
        assert_eq!(level(json!(e)), LogLevel::Error);
    }
    assert_eq!(level(json!("nope")), LogLevel::Info);
    assert_eq!(level(json!(3)), LogLevel::Info);
    assert_eq!(to_level(None), LogLevel::Info);
}

#[test]
fn ready_frames_carry_the_reset_flag_only_when_true() {
    assert_eq!(
        parse_log_frame(r#"{"type":"ready","seq":3,"reset":true}"#),
        LogFrame::Ready {
            seq: 3,
            reset: true
        }
    );
    assert_eq!(
        parse_log_frame(r#"{"type":"ready","seq":3}"#),
        LogFrame::Ready {
            seq: 3,
            reset: false
        }
    );
    assert_eq!(
        parse_log_frame(r#"{"type":"ready","seq":3,"reset":"yes"}"#),
        LogFrame::Ready {
            seq: 3,
            reset: false
        }
    );
}

#[test]
fn anything_not_understood_becomes_raw_text_and_never_an_error() {
    for text in [
        "not json",
        "[1,2]",
        "42",
        r#"{"type":"ready"}"#,
        r#"{"type":"log","seq":1}"#,
        r#"{"type":"log","msg":"x"}"#,
        r#"{"type":"gap"}"#,
        r#"{"nope":1}"#,
    ] {
        assert_eq!(
            parse_log_frame(text),
            LogFrame::Raw(text.to_owned()),
            "{text}"
        );
    }
}

#[test]
fn a_missing_time_or_component_defaults_to_empty_and_object_fields_are_stringified() {
    let r = log(&json!({"type":"log","seq":1,"msg":"m","fields":{"o":{"a":1},"n":null}}));
    assert_eq!((r.iso.as_str(), r.component.as_str()), ("", ""));
    let value = |k: &str| {
        r.fields
            .iter()
            .find(|f| f.key == k)
            .map(|f| f.value.clone())
    };
    assert_eq!(value("o").as_deref(), Some(r#"{"a":1}"#));
    assert_eq!(value("n").as_deref(), Some("null"));
}

#[test]
fn the_time_renders_in_the_viewers_zone_and_falls_back_to_the_text() {
    assert_eq!(
        format_log_time("2026-10-04T14:02:14.390123Z", 0),
        "14:02:14.390"
    );
    assert_eq!(
        format_log_time("2026-10-04T14:02:14.5Z", 120),
        "16:02:14.500"
    );
    assert_eq!(format_log_time("2026-10-04T23:30:00Z", 90), "01:00:00.000");
    assert_eq!(
        format_log_time("2026-10-04T14:02:14+02:00", 0),
        "12:02:14.000"
    );
    assert_eq!(format_log_time("yesterday", 0), "yesterday");
    assert_eq!(format_log_time("", 0), "");
}

#[test]
fn rfc3339_parsing_matches_known_epochs() {
    assert_eq!(parse_rfc3339_ms("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(
        parse_rfc3339_ms("2000-03-01T00:00:00Z"),
        Some(951_868_800_000)
    );
    assert_eq!(parse_rfc3339_ms("2026-13-01T00:00:00Z"), None);
    assert_eq!(parse_rfc3339_ms("2026-01-01T25:00:00Z"), None);
}

#[test]
fn the_expanded_view_reads_as_json() {
    let r = log(
        &json!({"type":"log","seq":1,"time":"t","level":"info","component":"c","msg":"hi",
            "fields":{"b":false,"n":2,"s":"x"}}),
    );
    let lines = json_lines(&r);
    let pairs: Vec<_> = lines
        .iter()
        .map(|l| (l.key.as_str(), l.value.as_str()))
        .collect();
    assert_eq!(
        pairs,
        [
            ("time", "\"t\""),
            ("level", "\"info\""),
            ("component", "\"c\""),
            ("msg", "\"hi\""),
            ("b", "false"),
            ("n", "2"),
            ("s", "\"x\"")
        ]
    );
    let bare = LogRecord {
        component: String::new(),
        fields: vec![],
        ..r
    };
    assert!(!json_lines(&bare).iter().any(|l| l.key == "component"));
}

#[test]
fn exec_frames_decode_and_degrade_to_raw() {
    assert_eq!(
        parse_exec_frame(r#"{"type":"out","stream":"stderr","data":"x\n"}"#),
        ExecFrame::Out {
            stderr: true,
            data: "x\n".into()
        }
    );
    assert_eq!(
        parse_exec_frame(r#"{"type":"out","data":"y"}"#),
        ExecFrame::Out {
            stderr: false,
            data: "y".into()
        }
    );
    assert_eq!(
        parse_exec_frame(r#"{"type":"exit","code":2}"#),
        ExecFrame::Exit {
            code: 2,
            error: String::new()
        }
    );
    assert_eq!(
        parse_exec_frame(r#"{"type":"error","status":403,"message":"no"}"#),
        ExecFrame::Error {
            status: 403,
            message: "no".into()
        }
    );
    assert_eq!(
        parse_exec_frame(r#"{"type":"error"}"#),
        ExecFrame::Error {
            status: 0,
            message: String::new()
        }
    );
    for text in [
        "junk",
        "[1]",
        r#"{"type":"out"}"#,
        r#"{"type":"exit"}"#,
        r#"{"type":"?"}"#,
    ] {
        assert_eq!(
            parse_exec_frame(text),
            ExecFrame::Raw(text.to_owned()),
            "{text}"
        );
    }
}
