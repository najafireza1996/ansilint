//! Engine-level errors.

use std::io;
use std::path::PathBuf;

use ansilint_config::{ConfigError, IgnoreError};
use thiserror::Error;

/// A failure that prevents linting from starting or completing.
#[derive(Debug, Error)]
pub enum EngineError {
    /// A configuration or ignore file could not be loaded.
    #[error("failed to load {path}: {reason}")]
    Load {
        /// Path that could not be loaded.
        path: PathBuf,
        /// Why loading failed.
        reason: String,
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
