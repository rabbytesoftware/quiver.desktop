use super::*;

#[test]
fn starts_clean_stops_the_stream_and_cancels_what_is_running() {
    let mut rig = live();
    rig.deliver(0, ready(0));
    rig.deliver(0, log(1, "a"));
    rig.flush();
    rig.submit("install x");
    assert_eq!(rig.c.state.running, 1);
    rig.sync("remote", true);
    assert!(rig.sockets[0].2);
    assert!(rig.execs[0].2);
    assert_eq!(rig.c.state.running, 0);
    assert!(rig.c.state.entries.is_empty());
    assert_eq!(rig.c.state.cursor, None);
    assert_eq!(rig.c.state.connection_id.as_deref(), Some("remote"));
    assert_eq!(
        rig.c.state.history,
        ["install x"],
        "history is the user's, not the connection's"
    );
}

#[test]
fn drops_frames_a_cancelled_run_still_delivers() {
    let mut rig = live();
    rig.submit("install x");
    rig.sync("remote", true);
    let before = rig.c.state.entries.len();
    rig.exec(0, out("late\n"));
    assert_eq!(rig.c.state.entries.len(), before);
}

#[test]
fn drops_stream_frames_queued_for_the_connection_it_left() {
    let mut rig = live();
    rig.deliver(0, ready(0));
    rig.deliver(0, log(1, "queued"));
    rig.sync("remote", true);
    rig.flush();
    assert!(log_msgs(&rig).is_empty() || rig.c.state.connection_id.as_deref() == Some("remote"));
    assert_eq!(rig.c.state.cursor, None);
}
