use super::*;

#[test]
fn stops_the_stream_cancels_runs_and_drops_what_is_queued() {
    let mut rig = live();
    rig.deliver(0, ready(0));
    rig.deliver(0, log(1, "queued"));
    rig.submit("list");
    let fx = rig.c.dispose();
    rig.apply(fx);
    assert!(rig.sockets[0].2);
    assert!(rig.execs[0].2);
    assert!(!rig.timers.contains(&TimerKind::Flush));
    assert_eq!(rig.c.state.running, 0);
    rig.flush();
    assert!(log_msgs(&rig).is_empty());
}
