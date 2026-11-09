use super::*;

#[test]
fn every_command_brings_the_log_back_to_its_newest_line_and_a_blank_line_does_not() {
    let mut rig = live();
    let before = rig.c.state.tail;
    rig.submit("list");
    assert_eq!(rig.c.state.tail, before + 1);
    rig.submit("  ");
    assert_eq!(rig.c.state.tail, before + 1);
}
