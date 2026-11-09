use crate::domain::{ConnectionConfig, ConnectionKind, ConnectionStatus};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConnectionRowView {
    pub id: String,
    pub name: String,
    pub kind: ConnectionKind,
    pub subtitle: Option<String>,
    pub is_local: bool,
    pub is_remote: bool,
    pub is_active: bool,
    /// Only the active connection has a live status -- a saved-but-inactive connection is not dialled,
    /// so there is nothing to report.
    pub status: Option<ConnectionStatus>,
    pub show_connect: bool,
    /// Local can never be renamed or removed, but it can still be switched away from -- so its menu
    /// button only shows once it is inactive.
    pub show_menu_button: bool,
}

/// Local is hidden until a remote exists: with nothing to switch between there is nothing to show.
pub fn connection_rows(
    connections: &[ConnectionConfig],
    active_id: &str,
    status: ConnectionStatus,
) -> Vec<ConnectionRowView> {
    let has_remote = connections.iter().any(|c| c.kind != ConnectionKind::Local);
    connections
        .iter()
        .filter(|c| has_remote || c.kind != ConnectionKind::Local)
        .map(|c| {
            let is_local = c.kind == ConnectionKind::Local;
            let is_active = c.id == active_id;
            let show_connect = !is_active;
            ConnectionRowView {
                id: c.id.clone(),
                name: c.name.clone(),
                kind: c.kind,
                subtitle: if is_local { None } else { c.url.clone() },
                is_local,
                is_remote: !is_local,
                is_active,
                status: is_active.then_some(status),
                show_connect,
                show_menu_button: show_connect || !is_local,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::local_connection;

    fn remote() -> ConnectionConfig {
        ConnectionConfig {
            id: "r1".into(),
            name: "Home".into(),
            kind: ConnectionKind::Remote,
            url: Some("https://home".into()),
            api_version: "v0".into(),
        }
    }

    #[test]
    fn local_alone_shows_no_rows() {
        assert!(
            connection_rows(&[local_connection()], "local", ConnectionStatus::Ready).is_empty()
        );
    }

    #[test]
    fn local_active_has_a_status_and_no_menu_and_the_remote_can_be_connected() {
        let rows = connection_rows(
            &[local_connection(), remote()],
            "local",
            ConnectionStatus::Ready,
        );
        let (local, remote) = (&rows[0], &rows[1]);
        assert_eq!(local.status, Some(ConnectionStatus::Ready));
        assert!(!local.show_connect && !local.show_menu_button);
        assert_eq!(local.subtitle, None);
        assert_eq!(remote.status, None);
        assert!(remote.show_connect && remote.show_menu_button && remote.is_remote);
        assert_eq!(remote.subtitle.as_deref(), Some("https://home"));
    }

    #[test]
    fn local_regains_its_menu_once_inactive_and_an_active_remote_keeps_its_own() {
        let rows = connection_rows(
            &[local_connection(), remote()],
            "r1",
            ConnectionStatus::Disconnected,
        );
        assert!(rows[0].show_menu_button && rows[0].show_connect);
        assert!(rows[1].is_active && rows[1].show_menu_button && !rows[1].show_connect);
        assert_eq!(rows[1].status, Some(ConnectionStatus::Disconnected));
    }
}
