use super::*;

#[test]
fn opens_only_while_the_console_is_open_the_daemon_supports_it_and_it_is_ready() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    rig.learn(supported());
    assert!(rig.sockets.is_empty());
    rig.open(true);
    assert_eq!(rig.sockets.len(), 1);
    assert!(rig.sockets[0].1.contains("/v0/console/logs"));
    rig.open(false);
    assert!(rig.sockets[0].2);
}

#[test]
fn does_not_open_for_an_unsupported_daemon_even_when_the_console_is_open() {
    let mut rig = Rig::default();
    rig.open(true);
    rig.sync("local", true);
    rig.learn(legacy());
    assert!(rig.sockets.is_empty());
}

#[test]
fn opens_as_soon_as_support_is_learned_when_the_console_was_already_open() {
    let mut rig = Rig::default();
    rig.open(true);
    rig.sync("local", true);
    assert!(rig.sockets.is_empty());
    rig.learn(supported());
    assert_eq!(rig.sockets.len(), 1);
}

#[test]
fn closes_when_the_daemon_stops_being_ready_and_reopens_from_the_cursor_when_it_is() {
    let mut rig = live();
    rig.deliver(0, log(7, "hi"));
    rig.deliver(0, ready(7));
    rig.flush();
    rig.sync("local", false);
    assert!(rig.sockets[0].2);
    rig.sync("local", true);
    assert_eq!(rig.sockets.len(), 2);
    assert!(rig.sockets[1].1.contains("since=7"));
}

#[test]
fn folds_frames_into_the_buffer_in_batches() {
    let mut rig = live();
    rig.deliver(0, log(1, "a"));
    rig.deliver(0, log(2, "b"));
    rig.deliver(0, ready(2));
    assert!(rig.c.state.entries.is_empty());
    assert_eq!(
        rig.timers
            .iter()
            .filter(|t| **t == TimerKind::Flush)
            .count(),
        1,
        "one timer for the batch"
    );
    rig.flush();
    assert_eq!(rig.c.state.entries.len(), 2);
    assert_eq!(rig.c.state.cursor, Some(2));
}

#[test]
fn holds_a_replay_back_until_its_ready_so_a_reset_can_replace_what_is_shown() {
    let mut rig = live();
    rig.deliver(0, log(1, "a"));
    rig.flush();
    assert!(rig.c.state.entries.is_empty());
    rig.deliver(0, ready(1));
    rig.flush();
    assert_eq!(rig.c.state.entries.len(), 1);
}

#[test]
fn passes_live_frames_straight_through_once_the_replay_is_over() {
    let mut rig = live();
    rig.deliver(0, ready(0));
    rig.deliver(0, log(1, "live"));
    rig.flush();
    assert_eq!(log_msgs(&rig), ["live"]);
}

#[test]
fn does_not_need_the_socket_to_have_reported_open_before_frames_arrive() {
    let mut rig = live();
    rig.deliver(0, log(1, "a"));
    rig.deliver(0, ready(1));
    rig.flush();
    assert_eq!(rig.c.state.entries.len(), 1);
}

#[test]
fn flushes_what_is_pending_when_the_console_closes() {
    let mut rig = live();
    rig.deliver(0, ready(0));
    rig.deliver(0, log(1, "a"));
    rig.open(false);
    assert_eq!(rig.c.state.entries.len(), 1);
    assert!(!rig.timers.contains(&TimerKind::Flush));
}

#[test]
fn replaces_the_shown_log_with_the_replay_when_the_daemon_says_it_restarted() {
    let mut rig = live();
    rig.deliver(0, log(40, "old"));
    rig.deliver(0, ready(40));
    rig.flush();
    assert_eq!(rig.c.state.cursor, Some(40));
    rig.drop_socket(0);
    rig.backoff();
    assert!(rig.sockets[1].1.contains("since=40"));
    rig.deliver(1, log(1, "new"));
    rig.deliver(1, json!({"type":"ready","seq":1,"reset":true}));
    rig.flush();
    assert_eq!(log_msgs(&rig), ["new"]);
    assert!(rig.texts().iter().any(|t| t == "Restarted"));
    assert_eq!(rig.c.state.cursor, Some(1));
}

#[test]
fn reconnects_from_the_last_line_shown_and_shows_each_line_once() {
    let mut rig = live();
    rig.deliver(0, log(1, "a"));
    rig.deliver(0, log(2, "b"));
    rig.deliver(0, ready(2));
    rig.flush();
    rig.drop_socket(0);
    assert_eq!(rig.c.state.stream, StreamState::Reconnecting);
    rig.backoff();
    assert_eq!(rig.sockets.len(), 2);
    assert!(rig.sockets[1].1.contains("since=2"));
    rig.deliver(1, log(2, "b"));
    rig.deliver(1, log(3, "c"));
    rig.deliver(1, ready(3));
    rig.flush();
    assert_eq!(log_msgs(&rig), ["a", "b", "c"]);
    assert_eq!(rig.c.state.cursor, Some(3));
}

#[test]
fn forgets_a_replay_that_was_cut_off_by_a_dropped_connection() {
    let mut rig = live();
    rig.deliver(0, log(1, "half"));
    rig.drop_socket(0);
    rig.backoff();
    rig.deliver(1, log(1, "whole"));
    rig.deliver(1, ready(1));
    rig.flush();
    assert_eq!(log_msgs(&rig), ["whole"]);
}

#[test]
fn ignores_frames_from_a_socket_that_is_no_longer_current() {
    let mut rig = live();
    rig.drop_socket(0);
    rig.backoff();
    rig.deliver(0, log(9, "stale"));
    rig.deliver(1, ready(0));
    rig.flush();
    assert!(rig.c.state.entries.is_empty());
}
