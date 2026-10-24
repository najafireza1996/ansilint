//! Configuration parse errors.

use ansilint_core::Span;
use thiserror::Error;

/// A configuration value that cannot be parsed.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{message} at {span}")]
pub struct ConfigError {
    /// Location in the configuration file.
    pub span: Span,
    /// Human-readable explanation.
    pub message: String,
}

impl ConfigError {
    /// Creates an error at a source span.
    #[must_use]
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}
