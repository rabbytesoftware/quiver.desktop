/// Everything that can go wrong while driving the app.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bridge i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("bridge json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bridge returned an error ({code}): {message}")]
    Rpc { code: i64, message: String },
    #[error("bridge closed the connection")]
    Closed,
    #[error("timed out waiting for {0}")]
    Timeout(String),
    #[error("app did not start: {0}")]
    Launch(String),
}

pub type Result<T> = std::result::Result<T, Error>;
