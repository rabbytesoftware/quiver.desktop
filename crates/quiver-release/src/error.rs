use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

/// Why a resolution failed, in terms the UI can turn into a sentence.
///
/// [`Error::kind`] is a stable machine-readable tag the frontend maps to a
/// translated message; [`Error::detail`] is the technical text, shown only
/// where technical text already belongs (the same problem dialog that shows a
/// failed step's own error verbatim). Serialises as `{ kind, detail }`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The machine could not reach GitHub at all: DNS, TLS, connect or timeout.
    #[error("{0}")]
    Offline(String),
    /// GitHub answered, and refused on quota: the release page throttles a
    /// client that asks too often, with a 403 or 429.
    #[error("{0}")]
    RateLimited(String),
    /// The repository publishes no release GitHub will name.
    #[error("{0}")]
    NoRelease(String),
    /// Any other non-success status.
    #[error("{0}")]
    Http(String),
    /// A 2xx whose body was not a release page.
    #[error("{0}")]
    Malformed(String),
    /// The release exists but carries nothing installable on this platform.
    #[error("{0}")]
    NoAsset(String),
    /// This build is running somewhere the manifest has no bundle for.
    #[error("{0}")]
    UnsupportedPlatform(String),
}

/// The stable `Error::kind()` tags.
pub mod kind {
    pub const OFFLINE: &str = "offline";
    pub const RATE_LIMITED: &str = "rate_limited";
    pub const NO_RELEASE: &str = "no_release";
    pub const HTTP: &str = "http";
    pub const MALFORMED: &str = "malformed";
    pub const NO_ASSET: &str = "no_asset";
    pub const UNSUPPORTED_PLATFORM: &str = "unsupported_platform";
}

impl Error {
    /// The stable tag the UI branches on.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Offline(_) => kind::OFFLINE,
            Self::RateLimited(_) => kind::RATE_LIMITED,
            Self::NoRelease(_) => kind::NO_RELEASE,
            Self::Http(_) => kind::HTTP,
            Self::Malformed(_) => kind::MALFORMED,
            Self::NoAsset(_) => kind::NO_ASSET,
            Self::UnsupportedPlatform(_) => kind::UNSUPPORTED_PLATFORM,
        }
    }

    /// The technical text.
    pub fn detail(&self) -> &str {
        match self {
            Self::Offline(d)
            | Self::RateLimited(d)
            | Self::NoRelease(d)
            | Self::Http(d)
            | Self::Malformed(d)
            | Self::NoAsset(d)
            | Self::UnsupportedPlatform(d) => d,
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("Error", 2)?;
        out.serialize_field("kind", self.kind())?;
        out.serialize_field("detail", self.detail())?;
        out.end()
    }
}
