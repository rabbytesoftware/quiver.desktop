//! The one error type of the services layer.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Platform(#[from] quiver_platform::Error),
    #[error("arrow app page: {0}")]
    Helper(String),
    #[error("could not start the async runtime: {0}")]
    Runtime(String),
}

pub type Result<T> = std::result::Result<T, Error>;
