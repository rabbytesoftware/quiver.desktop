//! The one error type of this crate.

/// Everything that can go wrong talking to quiver.core or managing connections.
///
/// The transport variants ([`Error::Connect`], [`Error::Protocol`]) keep the
/// split the Tauri app made: a failure to reach the peer at all versus a
/// failure once a conversation had started.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("connect failed: {0}")]
    Connect(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    /// The reachability probe failed; the message already says why.
    #[error("{0}")]
    Health(String),
    /// Redeeming a device-pairing code failed; the message already says why.
    #[error("{0}")]
    Pairing(String),
    #[error("cannot remove local connection")]
    CannotRemoveLocal,
    #[error("cannot rename local connection")]
    CannotRenameLocal,
    #[error("connection {0} not found")]
    ConnectionNotFound(String),
    /// Reading or writing saved connections or the device id failed.
    #[error("{0}")]
    Settings(String),
    /// Reading or writing a bearer token in the secret store failed.
    #[error("{0}")]
    Secret(String),
    /// The local quiver.core daemon could not be spawned or did not become ready.
    #[error("{0}")]
    Daemon(String),
    #[error("ws connection {0} is closed")]
    WsClosed(String),
    #[error("no open ws connection {0}")]
    NoSuchWs(String),
    /// Opening the WebSocket route failed.
    #[error("open ws {path}: {source}")]
    OpenWs { path: String, source: Box<Error> },
    #[error("command is longer than {0} bytes")]
    LineTooLong(usize),
    /// quiver.core answered a self-announce call with an error status.
    #[error("{message} (status {status})")]
    Api { status: u16, message: String },
}

pub type Result<T> = std::result::Result<T, Error>;
