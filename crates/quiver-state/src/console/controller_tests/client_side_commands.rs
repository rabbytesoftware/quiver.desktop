use super::*;

#[test]
fn clear_empties_the_buffer_without_asking_the_daemon() {
    let mut rig = live();
    rig.submit("list");
    rig.submit("clear");
    assert!(rig.c.state.entries.is_empty());
    assert_eq!(rig.execs.len(), 1);
    assert_eq!(rig.c.state.history, ["list", "clear"]);
}

#[test]
fn help_lists_the_daemons_own_table_not_a_list_of_its_own() {
    let mut rig = live();
    rig.submit("help");
    assert_eq!(rig.execs.len(), 0);
    assert_eq!(
        rig.texts(),
        [
            "$ help",
            "install <namespace>  install an arrow",
            "list                 list arrows"
        ]
    );
}

#[test]
fn help_fetches_the_table_when_it_was_not_loaded() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    rig.submit("help");
    assert_eq!(rig.commands_asked, 1);
    let epoch = rig.c.epoch;
    rig.c.on_commands(epoch, Some(table()), true);
    assert_eq!(rig.texts().len(), 3);
}

#[test]
fn help_says_so_when_the_table_cannot_be_had() {
    let mut rig = Rig::default();
    rig.sync("local", true);
    rig.submit("help");
    let epoch = rig.c.epoch;
    rig.c.on_commands(epoch, None, true);
    assert_eq!(
        rig.texts().last().map(String::as_str),
        Some("HelpUnavailable")
    );
}

#[test]
fn help_with_an_argument_goes_to_the_daemon_only_a_bare_help_is_ours() {
    let mut rig = live();
    rig.submit("help install");
    assert_eq!(rig.execs.len(), 1);
}
