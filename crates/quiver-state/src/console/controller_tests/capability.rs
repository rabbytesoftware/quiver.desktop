use super::*;

#[test]
fn asks_once_the_connection_is_ready_and_learns_the_capability_and_the_commands() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    rig.learn(supported());
    assert_eq!(rig.c.state.connection_id.as_deref(), Some("local"));
    assert_eq!(rig.c.state.support, Support::Supported);
    assert_eq!(rig.c.state.commands, table());
    assert!(rig.c.state.commands_loaded);
}

#[test]
fn does_not_ask_a_daemon_that_is_not_ready() {
    let mut rig = Rig::default();
    rig.sync("local", false);
    assert_eq!(rig.versions_asked, 0);
    assert_eq!(rig.c.state.support, Support::Unknown);
}

#[test]
fn asks_again_when_a_daemon_that_was_down_comes_back() {
    let mut rig = Rig::default();
    rig.sync("local", false);
    rig.sync("local", true);
    rig.sync("local", true);
    assert_eq!(rig.versions_asked, 1);
    rig.sync("local", false);
    rig.sync("local", true);
    assert_eq!(rig.versions_asked, 2);
}

#[test]
fn reads_an_older_daemon_as_unsupported_and_does_not_fetch_commands_it_does_not_have() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    rig.learn(legacy());
    assert_eq!(rig.c.state.support, Support::Unsupported);
    assert_eq!(rig.commands_asked, 0);
}

#[test]
fn stays_unknown_not_unsupported_when_the_daemon_cannot_be_reached() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    let epoch = rig.c.epoch;
    assert!(rig.c.on_versions(epoch, None).is_empty());
    assert_eq!(rig.c.state.support, Support::Unknown);
    assert!(rig.c.state.versions.is_none());
}

#[test]
fn works_without_help_and_completion_when_the_command_table_cannot_be_fetched() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    let epoch = rig.c.epoch;
    let fx = rig.c.on_versions(epoch, Some(supported()));
    rig.apply(fx);
    rig.c.on_commands(epoch, None, false);
    assert_eq!(rig.c.state.support, Support::Supported);
    assert!(!rig.c.state.commands_loaded);
}

#[test]
fn ignores_a_slow_answer_for_a_connection_that_is_no_longer_shown() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    let stale = rig.c.epoch;
    rig.sync("remote", true);
    rig.learn(legacy());
    assert!(rig.c.on_versions(stale, Some(supported())).is_empty());
    assert_eq!(rig.c.state.connection_id.as_deref(), Some("remote"));
    assert_eq!(rig.c.state.support, Support::Unsupported);
}
