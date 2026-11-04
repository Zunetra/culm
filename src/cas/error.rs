// === Errors

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CasError {
    #[error("cannot find a cache directory for this platform")]
    NoCacheDir,

    #[error("{}: {message}", .path.display())]
    Io { path: PathBuf, message: String },

    #[error("object `{hash}` is not in the store")]
    Missing { hash: String },

    #[error("object `{hash}` is corrupted: stored bytes hash to a different digest")]
    Corrupted { hash: String },

    #[error("{}: {message}", .path.display())]
    Malformed { path: PathBuf, message: String },
}

impl CasError {
    /// The telemetry error code: which failure, never its text.
    pub fn code(&self) -> &'static str {
        return match self {
            Self::NoCacheDir { .. } => "cas_no_cache_dir",
            Self::Io { .. } => "cas_io",
            Self::Missing { .. } => "cas_missing",
            Self::Corrupted { .. } => "cas_corrupted",
            Self::Malformed { .. } => "cas_malformed",
        };
    }
}
