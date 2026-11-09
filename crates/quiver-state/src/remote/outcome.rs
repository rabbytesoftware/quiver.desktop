use crate::domain::ConnectionStatus;
use crate::i18n::{en::remote, Key, Params, Translator};

/// What a connection mutation ended in; the toast each has is written once, here, for the manage screen,
/// the row menu and the command palette alike.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnectionOutcome {
    /// `status` is the one the switch left behind: the Rust side calls `start()` before the switch returns,
    /// so it is already current.
    Connected {
        status: ConnectionStatus,
    },
    ConnectFailed,
    Added,
    AddFailed,
    Renamed,
    RenameFailed,
    Removed,
    RemoveFailed,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Toast {
    pub id: u64,
    pub message: String,
}

pub fn toast_key(outcome: ConnectionOutcome) -> Key {
    match outcome {
        ConnectionOutcome::Connected {
            status: ConnectionStatus::Disconnected,
        } => remote::TOAST_DISCONNECTED,
        ConnectionOutcome::Connected { .. } => remote::TOAST_CONNECTED,
        ConnectionOutcome::ConnectFailed => remote::TOAST_CONNECT_FAILED,
        ConnectionOutcome::Added => remote::TOAST_ADDED,
        ConnectionOutcome::AddFailed => remote::TOAST_ADD_FAILED,
        ConnectionOutcome::Renamed => remote::TOAST_RENAMED,
        ConnectionOutcome::RenameFailed => remote::TOAST_RENAME_FAILED,
        ConnectionOutcome::Removed => remote::TOAST_REMOVED,
        ConnectionOutcome::RemoveFailed => remote::TOAST_REMOVE_FAILED,
    }
}

/// The toast text for `outcome` about the connection called `name`. A success also closes the dialog it
/// came from; a failure leaves it open (see [`ConnectionOutcome::closes_dialog`]).
pub fn toast_for(tr: &Translator, outcome: ConnectionOutcome, name: &str) -> String {
    tr.t_with(toast_key(outcome), &Params::new().with("name", name))
}

impl ConnectionOutcome {
    pub fn closes_dialog(self) -> bool {
        matches!(self, Self::Added | Self::Renamed | Self::Removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switch_toasts_what_the_status_became() {
        let tr = Translator::default();
        let up = ConnectionOutcome::Connected {
            status: ConnectionStatus::Ready,
        };
        let down = ConnectionOutcome::Connected {
            status: ConnectionStatus::Disconnected,
        };
        assert_eq!(toast_for(&tr, up, "Home"), "Connected to Home");
        assert_eq!(toast_for(&tr, down, "Home"), "Couldn't reach Home");
        assert_eq!(
            toast_for(&tr, ConnectionOutcome::ConnectFailed, "Home"),
            "Couldn't switch to Home"
        );
    }

    #[test]
    fn every_outcome_has_its_own_message_and_only_successes_close_a_dialog() {
        let tr = Translator::default();
        let all = [
            ConnectionOutcome::Added,
            ConnectionOutcome::AddFailed,
            ConnectionOutcome::Renamed,
            ConnectionOutcome::RenameFailed,
            ConnectionOutcome::Removed,
            ConnectionOutcome::RemoveFailed,
        ];
        let messages: Vec<_> = all.iter().map(|o| toast_for(&tr, *o, "X")).collect();
        let mut unique = messages.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), messages.len());
        assert!(messages.iter().all(|m| m.contains('X')));
        let closes: Vec<_> = all.iter().map(|o| o.closes_dialog()).collect();
        assert_eq!(closes, [true, false, true, false, true, false]);
    }
}
