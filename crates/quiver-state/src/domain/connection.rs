use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionKind {
    Local,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub kind: ConnectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub api_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Starting,
    Ready,
    Disconnected,
}

pub const LOCAL_CONNECTION_ID: &str = "local";

pub fn local_connection() -> ConnectionConfig {
    ConnectionConfig {
        id: LOCAL_CONNECTION_ID.into(),
        name: "Local".into(),
        kind: ConnectionKind::Local,
        url: None,
        api_version: "v0".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_connection_has_correct_id_and_kind() {
        let c = local_connection();
        assert_eq!(c.id, LOCAL_CONNECTION_ID);
        assert_eq!(c.kind, ConnectionKind::Local);
        assert!(c.url.is_none());
        assert_eq!(c.api_version, "v0");
    }
}
