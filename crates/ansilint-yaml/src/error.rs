//! YAML parsing errors with source spans.

use ansilint_core::Span;
use saphyr_parser::ScanError;
use thiserror::Error;

/// A YAML parse failure.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{message} at {span}")]
pub struct YamlError {
    /// Location of the failure.
    pub span: Span,
    /// Scanner or parser message.
    pub message: String,
}

impl YamlError {
    /// Converts a scanner error into a span-carrying error.
    #[must_use]
    pub fn from_scan(error: &ScanError) -> Self {
        let offset = u32::try_from(error.marker().index()).unwrap_or(u32::MAX);
        Self {
            span: Span::empty(offset),
            message: error.info().to_string(),
        }
    }

    pub(crate) fn unexpected(span: Span, event: &str) -> Self {
        Self {
            span,
            message: format!("unexpected YAML event {event}"),
        }
    }

    pub(crate) fn eof() -> Self {
        Self {
            span: Span::empty(0),
            message: "unexpected end of YAML input".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use saphyr_parser::{Marker, ScanError};

    use super::YamlError;
    use ansilint_core::Span;

    #[test]
    fn scan_errors_keep_the_marker_offset() {
        let scan = ScanError::new(Marker::new(5, 1, 6), "boom".to_string());
        let error = YamlError::from_scan(&scan);
        assert_eq!(error.span, Span::empty(5));
        assert_eq!(error.message, "boom");
        assert!(error.to_string().contains("boom"));
    }

    #[test]
    fn synthetic_errors_are_displayed() {
        let unexpected = YamlError::unexpected(Span::new(1, 2), "MappingEnd");
        assert!(unexpected.to_string().contains("MappingEnd"));
        let eof = YamlError::eof();
        assert!(eof.to_string().contains("end of YAML"));
    }
}
