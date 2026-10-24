//! Output formatters that render findings into a caller-provided sink.
//!
//! It must not: decide exit codes, discover files, or read global state.
#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::path::Path;

use ansilint_core::{Finding, Severity};
use serde_json::json;
use thiserror::Error;

/// Formats implemented in Phase 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// `path:line: tag: message`
    Pep8,
    /// A JSON array of findings.
    Json,
    /// No output.
    Quiet,
}

impl Format {
    /// Parses a CLI format name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "pep8" => Some(Self::Pep8),
            "json" => Some(Self::Json),
            "quiet" => Some(Self::Quiet),
            _ => None,
        }
    }

    /// Returns the CLI name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pep8 => "pep8",
            Self::Json => "json",
            Self::Quiet => "quiet",
        }
    }
}

/// A finding with resolved display positions.
#[derive(Clone, Copy, Debug)]
pub struct DisplayFinding<'a> {
    /// The original finding.
    pub finding: &'a Finding,
    /// One-based line.
    pub line: u32,
    /// One-based column in Unicode scalar values.
    pub col: u32,
}

/// Output errors.
#[derive(Debug, Error)]
pub enum OutputError {
    /// Writing to the sink failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// JSON serialization failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Renders findings in the selected format.
///
/// # Errors
///
/// Returns [`OutputError`] when the sink or JSON serializer fails.
pub fn render(
    format: Format,
    findings: &[DisplayFinding<'_>],
    out: &mut impl Write,
) -> Result<(), OutputError> {
    match format {
        Format::Pep8 => render_pep8(findings, out),
        Format::Json => render_json(findings, out),
        Format::Quiet => Ok(()),
    }
}

/// Renders the default machine-friendly `path:line: tag: message` format.
///
/// # Errors
///
/// Returns an I/O error when writing to the sink fails.
pub fn render_pep8(
    findings: &[DisplayFinding<'_>],
    out: &mut impl Write,
) -> Result<(), OutputError> {
    for display in findings {
        writeln!(
            out,
            "{}:{}: {}: {}",
            path_string(&display.finding.path),
            display.line,
            display.finding.tag,
            display.finding.message
        )?;
    }
    Ok(())
}

/// Renders a stable JSON array of findings.
///
/// # Errors
///
/// Returns an error when writing or serializing fails.
pub fn render_json(
    findings: &[DisplayFinding<'_>],
    out: &mut impl Write,
) -> Result<(), OutputError> {
    let values: Vec<serde_json::Value> = findings
        .iter()
        .map(|display| {
            json!({
                "path": path_string(&display.finding.path),
                "line": display.line,
                "column": display.col,
                "tag": display.finding.tag.to_string(),
                "message": display.finding.message,
                "severity": severity_name(display.finding.severity),
            })
        })
        .collect();
    serde_json::to_writer(&mut *out, &values)?;
    writeln!(out)?;
    Ok(())
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::assert_is_empty,
        clippy::expect_used,
        reason = "tests use fixed fixtures and panic on broken assertions"
    )]

    use ansilint_core::{Finding, Severity, Span, Tag};

    use super::{render, DisplayFinding, Format};

    fn finding(tag: Tag, message: &str, severity: Severity) -> Finding {
        Finding::new(
            tag,
            message,
            std::path::PathBuf::from("playbooks/site.yml"),
            Span::new(10, 20),
            severity,
        )
    }

    fn display(finding: &Finding, line: u32, col: u32) -> DisplayFinding<'_> {
        DisplayFinding { finding, line, col }
    }

    #[test]
    fn pep8_is_stable() {
        let first = finding(Tag::new("parser-error"), "bad yaml", Severity::Error);
        let second = finding(
            Tag::with_sub("yaml", "truthy"),
            "use true",
            Severity::Warning,
        );
        let findings = [display(&first, 3, 5), display(&second, 7, 1)];
        let mut buffer = Vec::new();
        render(Format::Pep8, &findings, &mut buffer).expect("render");
        let out = String::from_utf8(buffer).expect("UTF-8 output");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn json_is_stable() {
        let first = finding(Tag::new("parser-error"), "bad yaml", Severity::Error);
        let findings = [display(&first, 3, 5)];
        let mut buffer = Vec::new();
        render(Format::Json, &findings, &mut buffer).expect("render");
        let out = String::from_utf8(buffer).expect("UTF-8 output");
        insta::assert_snapshot!(out);
    }

    #[test]
    fn quiet_writes_nothing() {
        let first = finding(Tag::new("warning"), "empty", Severity::Warning);
        let findings = [display(&first, 1, 1)];
        let mut out = Vec::new();
        render(Format::Quiet, &findings, &mut out).expect("render");
        assert!(out.is_empty());
    }
}
