use crate::core_api::ApiError;

/// The one error type of this crate.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error("could not decode {what}: {message}")]
    Decode { what: &'static str, message: String },
    #[error("backend: {0}")]
    Backend(String),
}

impl Error {
    pub(crate) fn decode(what: &'static str, err: serde_json::Error) -> Self {
        Error::Decode {
            what,
            message: err.to_string(),
        }
    }

    /// Whether this is an API 404.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::Api(e) if e.is_not_found())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
