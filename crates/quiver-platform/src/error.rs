pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("settings file {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("settings file {path} is not valid JSON: {source}")]
    Json {
        path: std::path::PathBuf,
        source: serde_json::Error,
    },
    #[error("setting `{key}`: {source}")]
    Value {
        key: String,
        source: serde_json::Error,
    },
    #[error("secret store: {0}")]
    Secret(String),
    #[error("no home directory could be determined for this user")]
    NoHome,
}
