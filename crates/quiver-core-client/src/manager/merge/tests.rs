use super::*;
use crate::types::ConnectionKind;

fn config(id: &str, name: &str) -> ConnectionConfig {
    ConnectionConfig {
        id: id.into(),
        name: name.into(),
        kind: ConnectionKind::Remote,
        url: Some(format!("http://{id}.example")),
        api_version: "v0".into(),
    }
}

/// Reproduces a real bug: with `active` = local and a saved remote, the
/// remote used to appear once (correctly) -- this pins that the fix did
/// not regress the unremarkable case.
#[test]
fn merge_connections_lists_local_once_alongside_every_saved_remote() {
    let local = config("local", "Local");
    let remotes = vec![config("home-lab", "Home Lab")];

    let list = merge_connections("local", local.clone(), local.clone(), remotes);

    let ids: Vec<&str> = list.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["local", "home-lab"]);
}

/// The actual bug: when the active connection IS a saved remote, the old
/// `[active] + remotes` concatenation listed that remote twice (once as
/// `active`, once again from its own persisted entry) and dropped
/// "local" entirely, since "local" is never itself a persisted remote.
#[test]
fn merge_connections_does_not_duplicate_the_active_remote_or_drop_local() {
    let local_default = config("local", "Local");
    let active_remote = config("home-lab", "Home Lab");
    let remotes = vec![active_remote.clone()];

    let list = merge_connections("home-lab", active_remote, local_default, remotes);

    let ids: Vec<&str> = list.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        ["local", "home-lab"],
        "must list each connection exactly once"
    );
}

/// The active connection's live config (e.g. a rename not yet persisted)
/// wins over the stale persisted copy for that same id.
#[test]
fn merge_connections_prefers_the_live_active_config_over_the_persisted_one() {
    let local_default = config("local", "Local");
    let live = config("home-lab", "Renamed");
    let stale = config("home-lab", "Home Lab");

    let list = merge_connections("home-lab", live, local_default, vec![stale]);

    assert_eq!(list[1].name, "Renamed");
}

/// A connection can still be active after `remove_connection` drops it
/// from the saved list (removal never touches `active`); it must keep
/// showing up rather than disappear from the merged list.
#[test]
fn merge_connections_keeps_an_active_connection_that_is_no_longer_saved() {
    let local_default = config("local", "Local");
    let active_remote = config("home-lab", "Home Lab");

    let list = merge_connections("home-lab", active_remote, local_default, vec![]);

    let ids: Vec<&str> = list.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["local", "home-lab"]);
}
