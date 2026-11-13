use super::*;

#[test]
fn cancelling_closes_every_running_command_once_and_says_so() {
    let mut rig = live();
    rig.submit("slow");
    rig.submit("slower");
    let effects = rig.c.cancel_running("Cancelled");
    assert_eq!(
        effects,
        [
            ConsoleEffect::CancelExec { run: 0 },
            ConsoleEffect::CancelExec { run: 1 }
        ]
    );
    assert_eq!(rig.c.state.running, 0);
    assert!(rig.texts().last().is_some_and(|t| t.contains("Cancelled")));
    assert!(
        rig.c.cancel_running("Cancelled").is_empty(),
        "nothing left to cancel"
    );
}

#[test]
fn late_frames_of_a_cancelled_run_are_dropped() {
    let mut rig = live();
    rig.submit("slow");
    rig.c.cancel_running("Cancelled");
    rig.exec(0, out("too late"));
    assert!(!rig.texts().iter().any(|t| t == "too late"));
}
