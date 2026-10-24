//! Findings, severities and tags.

use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;

use crate::{Fix, Span};

/// How a finding affects the exit code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational, never fails the run.
    Info,
    /// A warning: reported, and fatal only in strict mode or by configuration.
    Warning,
    /// An error: fails the run.
    Error,
}

/// An upstream-compatible rule tag: `rule-id` or `rule-id[sub-tag]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tag {
    /// The stable rule identifier, e.g. `fqcn`.
    pub rule: &'static str,
    /// The optional sub-tag, e.g. `action-core` in `fqcn[action-core]`.
    pub sub: Option<Cow<'static, str>>,
}

impl Tag {
    /// Creates a tag without a sub-tag.
    #[must_use]
    pub const fn new(rule: &'static str) -> Self {
        Self { rule, sub: None }
    }

    /// Creates a tag with a sub-tag.
    #[must_use]
    pub fn with_sub(rule: &'static str, sub: impl Into<Cow<'static, str>>) -> Self {
        Self {
            rule,
            sub: Some(sub.into()),
        }
    }

    /// Returns `true` when a `# noqa` / ignore-file pattern applies to this tag.
    ///
    /// A bare rule identifier matches every sub-tag; `rule[sub]` matches exactly.
    #[must_use]
    pub fn matches(&self, pattern: &str) -> bool {
        let pattern = pattern.trim();
        if let Some((rule, rest)) = pattern.split_once('[') {
            let Some(sub) = rest.strip_suffix(']') else {
                return false;
            };
            self.rule == rule && self.sub.as_deref() == Some(sub)
        } else {
            self.rule == pattern
        }
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.sub {
            Some(sub) => write!(formatter, "{}[{}]", self.rule, sub),
            None => formatter.write_str(self.rule),
        }
    }
}

/// A diagnostic produced for a source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The rule tag.
    pub tag: Tag,
    /// Human-readable message.
    pub message: String,
    /// File path, as given by the caller.
    pub path: PathBuf,
    /// Location of the problem.
    pub span: Span,
    /// Severity before profile and configuration mapping.
    pub severity: Severity,
    /// Optional machine-applicable fix.
    pub fix: Option<Fix>,
    /// Optional extra context, not part of default output.
    pub details: Option<String>,
}

impl Finding {
    /// Creates a finding without a fix or details.
    #[must_use]
    pub fn new(
        tag: Tag,
        message: impl Into<String>,
        path: PathBuf,
        span: Span,
        severity: Severity,
    ) -> Self {
        Self {
            tag,
            message: message.into(),
            path,
            span,
            severity,
            fix: None,
            details: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Severity, Tag};

    #[test]
    fn bare_rule_patterns_match_every_sub_tag() {
        let tag = Tag::with_sub("yaml", "truthy");
        assert!(tag.matches("yaml"));
        assert!(!tag.matches("yaml[comments]"));
        assert!(tag.matches("yaml[truthy]"));
    }

    #[test]
    fn tags_render_like_upstream() {
        assert_eq!(Tag::new("no-tabs").to_string(), "no-tabs");
        assert_eq!(
            Tag::with_sub("fqcn", "action-core").to_string(),
            "fqcn[action-core]"
        );
    }

    #[test]
    fn malformed_patterns_do_not_match() {
        assert!(!Tag::new("yaml").matches("yaml[unclosed"));
        assert!(!Tag::new("yaml").matches("yaml["));
    }

    #[test]
    fn severity_is_ordered_by_fatality() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }
}
