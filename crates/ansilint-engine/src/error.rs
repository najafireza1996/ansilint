//! Engine-level errors.

use std::io;
use std::path::PathBuf;

use ansilint_config::{ConfigError, IgnoreError};
use thiserror::Error;

/// A failure that prevents linting from starting or completing.
#[derive(Debug, Error)]
pub enum EngineError {
    /// Reading a file failed.
    #[error("failed to read {path}: {source}")]
    Read {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// Writing a file failed.
    #[error("failed to write {path}: {source}")]
    Write {
        /// Path that could not be written.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// Configuration parsing failed.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// The ignore file is malformed.
    #[error(transparent)]
    Ignore(#[from] IgnoreError),
    /// Directory walking failed.
    #[error("failed to walk files: {0}")]
    Walk(#[from] ignore::Error),
}
