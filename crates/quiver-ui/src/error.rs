//! The crate error type.

/// Everything that can fail while building or painting a component.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A vector path (icon, wavy line) could not be tessellated.
    #[error("could not build a vector path: {0}")]
    Path(String),
}

pub type Result<T> = std::result::Result<T, Error>;
