//! The four connection mutations and what each one means for the screen (port of
//! `use-connection-actions.ts`): the outcome to toast, and for a switch the list update the daemon never
//! announces itself.

use super::add_form::AddSubmission;
use super::outcome::ConnectionOutcome;
use crate::connection::{Backend, ConnectionStore};
use crate::domain::ConnectionStatus;

/// The health probe before pairing (`/v0/health`, tokenless). Any failure reads as unreachable.
pub async fn is_reachable(backend: &dyn Backend, url: &str) -> bool {
    backend.check_remote_health(url).await.is_ok()
}

/// `switch_connection` does not itself emit `connection://changed`: the list is unchanged by a switch,
/// only which entry is active, so that is updated here rather than waiting for an event that never
/// comes. The backend has already started the new connection, so `status()` is current when the switch
/// returns.
pub async fn connect(
    backend: &dyn Backend,
    connections: &ConnectionStore,
    id: &str,
    status: impl FnOnce() -> ConnectionStatus,
) -> ConnectionOutcome {
    if backend.switch_connection(id).await.is_err() {
        return ConnectionOutcome::ConnectFailed;
    }
    let current = connections.snapshot();
    connections.set_from_event(current.connections, id);
    ConnectionOutcome::Connected { status: status() }
}

pub async fn add(backend: &dyn Backend, submission: &AddSubmission) -> ConnectionOutcome {
    match backend
        .add_connection(&submission.name, &submission.url, &submission.code)
        .await
    {
        Ok(_) => ConnectionOutcome::Added,
        Err(_) => ConnectionOutcome::AddFailed,
    }
}

pub async fn rename(backend: &dyn Backend, id: &str, name: &str) -> ConnectionOutcome {
    match backend.rename_connection(id, name).await {
        Ok(()) => ConnectionOutcome::Renamed,
        Err(_) => ConnectionOutcome::RenameFailed,
    }
}

pub async fn remove(backend: &dyn Backend, id: &str) -> ConnectionOutcome {
    match backend.remove_connection(id).await {
        Ok(()) => ConnectionOutcome::Removed,
        Err(_) => ConnectionOutcome::RemoveFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::FakeBackend;
    use crate::domain::local_connection;

    #[tokio::test]
    async fn a_switch_marks_the_new_connection_active_and_reports_the_status_it_ended_in() {
        let backend = FakeBackend::new();
        let store = ConnectionStore::default();
        store.set_from_event(vec![local_connection()], "local");
        let outcome = connect(&backend, &store, "r1", || ConnectionStatus::Disconnected).await;
        assert_eq!(
            outcome,
            ConnectionOutcome::Connected {
                status: ConnectionStatus::Disconnected
            }
        );
        assert_eq!(store.snapshot().active_id, "r1");
        assert_eq!(backend.call_count("switch_connection r1"), 1);
    }

    #[tokio::test]
    async fn add_rename_and_remove_report_their_success_outcomes() {
        let backend = FakeBackend::new();
        let sub = AddSubmission {
            name: "Home".into(),
            url: "http://h".into(),
            code: "123456".into(),
        };
        assert_eq!(add(&backend, &sub).await, ConnectionOutcome::Added);
        assert_eq!(rename(&backend, "a", "B").await, ConnectionOutcome::Renamed);
        assert_eq!(remove(&backend, "a").await, ConnectionOutcome::Removed);
        assert!(is_reachable(&backend, "http://h").await);
        assert_eq!(backend.call_count("add_connection Home http://h 123456"), 1);
    }
}
