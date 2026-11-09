use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    /// The daemon (or the transport to it) failed; the text is shown to the page.
    #[error("{0}")]
    Upstream(String),
    #[error("unknown arrow host {0}")]
    UnknownHost(String),
    #[error("unsafe websocket path")]
    UnsafeWsPath,
    #[error("ipc line: {0}")]
    Ipc(#[from] serde_json::Error),
}
