use super::*;

#[test]
fn echoes_the_line_sends_it_exactly_as_typed_and_remembers_it() {
    let mut rig = live();
    rig.submit("  install   github.com/a/b  ");
    assert_eq!(rig.execs[0].1, "install   github.com/a/b");
    assert_eq!(rig.texts(), ["$ install   github.com/a/b"]);
    assert_eq!(rig.c.state.history, ["install   github.com/a/b"]);
    assert!(matches!(
        rig.c.state.entries[0].body,
        NewEntry::Cmd {
            at: 1_791_122_529_000,
            ..
        }
    ));
}

#[test]
fn ignores_a_blank_line() {
    let mut rig = live();
    rig.submit("   ");
    assert!(
        rig.execs.is_empty() && rig.c.state.entries.is_empty() && rig.c.state.history.is_empty()
    );
}

#[test]
fn shows_output_a_line_at_a_time_and_a_clean_exit_adds_nothing() {
    let mut rig = live();
    rig.submit("list");
    rig.exec(0, out("a\nb"));
    rig.exec(0, out("c\n"));
    rig.exec(0, exit(0));
    assert_eq!(rig.texts(), ["$ list", "a", "bc"]);
    assert_eq!(rig.c.state.running, 0);
}

#[test]
fn flushes_an_unterminated_last_line_when_the_command_exits() {
    let mut rig = live();
    rig.submit("list");
    rig.exec(0, out("tail"));
    rig.exec(0, exit(0));
    assert_eq!(rig.texts().last().map(String::as_str), Some("tail"));
}

#[test]
fn says_something_for_a_failure() {
    let mut rig = live();
    rig.submit("list");
    rig.exec(0, json!({"type":"exit","code":3,"error":"unreachable"}));
    assert_eq!(
        rig.texts().last().map(String::as_str),
        Some(r#"Exit { code: 3, error: "unreachable" }"#)
    );
}

#[test]
fn shows_a_line_the_daemon_rejects_as_an_error_note_in_its_own_words() {
    let mut rig = live();
    rig.submit("rm -rf $HOME");
    rig.exec(
        0,
        json!({"type":"error","status":400,"message":"quoting is not supported"}),
    );
    assert_eq!(rig.c.state.running, 0);
    assert!(rig
        .texts()
        .last()
        .is_some_and(|t| t.contains("Refused") && t.contains("quoting is not supported")));
}

#[test]
fn reports_a_failure_to_reach_the_daemon_as_plain_text() {
    let mut rig = live();
    rig.submit("list");
    rig.exec(
        0,
        json!({"type":"error","status":0,"message":"connection refused"}),
    );
    assert_eq!(
        rig.texts().last().map(String::as_str),
        Some(r#"Text("connection refused")"#)
    );
}

#[test]
fn shows_a_frame_it_cannot_decode_as_raw_text_and_ignores_frames_after_the_end() {
    let mut rig = live();
    rig.submit("list");
    rig.c.exec_frame(rig.execs[0].0, "garbage");
    rig.exec(0, exit(0));
    let after = rig.c.state.entries.len();
    rig.exec(0, out("late\n"));
    assert_eq!(rig.c.state.entries.len(), after);
    assert!(rig.texts().contains(&"raw:garbage".to_owned()));
}

#[test]
fn counts_several_runs_at_once() {
    let mut rig = live();
    rig.submit("a");
    rig.submit("b");
    assert_eq!(rig.c.state.running, 2);
    rig.exec(0, exit(0));
    assert_eq!(rig.c.state.running, 1);
    rig.exec(1, exit(0));
    assert_eq!(rig.c.state.running, 0);
}

#[test]
fn a_connection_that_ends_before_an_exit_frame_stops_counting_as_running() {
    let mut rig = live();
    rig.submit("a");
    rig.exec(0, out("partial"));
    rig.c.exec_closed(rig.execs[0].0);
    assert_eq!(rig.c.state.running, 0);
    assert_eq!(rig.texts().last().map(String::as_str), Some("partial"));
}

#[test]
fn refuses_to_run_on_a_daemon_known_to_have_no_console_and_says_so() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    rig.learn(legacy());
    rig.submit("list");
    assert!(rig.execs.is_empty());
    assert_eq!(rig.texts().last().map(String::as_str), Some("Unsupported"));
}

#[test]
fn still_tries_when_it_has_not_heard_from_the_daemon_yet() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    rig.submit("list");
    assert_eq!(rig.execs.len(), 1);
}
