//! Merging the live active connection with the saved ones into one list.

use crate::types::ConnectionConfig;

/// One entry per known connection -- "local" plus every saved remote --
/// never a duplicate: a saved remote that happens to be `active_id` would
/// otherwise appear both as the active connection and again from its own
/// persisted entry, and "local" would vanish outright, since it is never
/// itself one of the persisted remotes. Whichever entry matches `active_id`
/// is taken from the live `active_config` rather than the persisted copy, so
/// a rename that has not round-tripped through storage yet still shows up --
/// this also covers a connection that is still active after being removed
/// from the saved list, which `remove_connection` allows.
pub(super) fn merge_connections(
    active_id: &str,
    active_config: ConnectionConfig,
    local_default: ConnectionConfig,
    remotes: Vec<ConnectionConfig>,
) -> Vec<ConnectionConfig> {
    let local_config = if active_id == "local" {
        active_config.clone()
    } else {
        local_default
    };

    let mut list = vec![local_config];
    let mut active_is_listed = active_id == "local";
    for remote in remotes {
        if remote.id == active_id {
            list.push(active_config.clone());
            active_is_listed = true;
        } else {
            list.push(remote);
        }
    }
    if !active_is_listed {
        list.push(active_config);
    }
    list
}

#[cfg(test)]
mod tests;
