//! Post-processing: inline `# noqa`, ignore files and configuration lists.

use std::path::Path;

use ansilint_config::{Config, IgnoreFile};
use ansilint_core::{Finding, Severity, Tag};
use ansilint_yaml::SourceFile;

use crate::PositionedFinding;

const NON_SKIPPABLE: &[&str] = &["parser-error", "load-failure", "internal-error", "schema"];

/// Configuration-dependent post-processing inputs.
pub(crate) struct PostContext<'a> {
    /// Effective configuration.
    pub config: &'a Config,
    /// Parsed ignore file, when one exists.
    pub ignore: Option<&'a IgnoreFile>,
}

/// Returns `false` when a finding should be dropped; otherwise adjusts severity.
#[must_use]
pub(crate) fn keep(
    finding: &mut Finding,
    line_text: Option<&str>,
    context: &PostContext<'_>,
) -> bool {
    let skippable = !NON_SKIPPABLE.contains(&finding.tag.rule);
    if skippable && line_text.is_some_and(|line| noqa_matches(line, &finding.tag)) {
        return false;
    }
    if let Some(ignore) = context.ignore {
        let path = finding.path.to_string_lossy().replace('\\', "/");
        match ignore.disposition(&path, &finding.tag) {
            Some(true) => return false,
            Some(false) => finding.severity = Severity::Warning,
            None => {}
        }
    }
    if matches_any(&finding.tag, &context.config.skip_list) {
        return false;
    }
    if !context.config.tags.is_empty() && !tag_selected(&finding.tag, &context.config.tags) {
        return false;
    }
    if matches_any(&finding.tag, &context.config.warn_list) {
        finding.severity = Severity::Warning;
    }
    true
}

fn matches_any(tag: &Tag, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| tag.matches(pattern))
}

fn tag_selected(tag: &Tag, patterns: &[String]) -> bool {
    if matches_any(tag, patterns) {
        return true;
    }
    ansilint_rules::all().iter().any(|rule| {
        let meta = rule.meta();
        meta.id == tag.rule
            && meta
                .tags
                .iter()
                .any(|meta_tag| patterns.iter().any(|pattern| pattern == meta_tag))
    })
}

fn noqa_matches(line: &str, tag: &Tag) -> bool {
    let Some(index) = line.find("# noqa") else {
        return false;
    };
    let rest = line
        .get(index + "# noqa".len()..)
        .unwrap_or_default()
        .trim_start();
    let Some(rest) = rest.strip_prefix(':') else {
        return false;
    };
    rest.split(',').any(|pattern| tag.matches(pattern.trim()))
}

pub(crate) fn process(
    findings: Vec<Finding>,
    source: Option<&SourceFile>,
    output_path: &Path,
    config: &Config,
    ignore: Option<&IgnoreFile>,
) -> Vec<PositionedFinding> {
    let context = PostContext { config, ignore };
    let mut results = Vec::new();
    for mut finding in findings {
        finding.path = output_path.to_path_buf();
        let (line, col, line_text) = match source {
            Some(source) => {
                let position = source
                    .line_index
                    .line_col(source.as_str(), finding.span.start);
                let index = usize::try_from(position.line.saturating_sub(1)).unwrap_or(usize::MAX);
                (
                    position.line,
                    position.col,
                    source.line_index.line_text(source.as_str(), index),
                )
            }
            None => (1, 1, None),
        };
        if keep(&mut finding, line_text, &context) {
            results.push(positioned(finding, line, col));
        }
    }
    results
}

fn positioned(finding: Finding, line: u32, col: u32) -> PositionedFinding {
    PositionedFinding { finding, line, col }
}

pub(crate) fn sort_and_dedupe(findings: &mut Vec<PositionedFinding>) {
    findings.sort_by(|left, right| {
        left.finding
            .path
            .cmp(&right.finding.path)
            .then(left.line.cmp(&right.line))
            .then(left.col.cmp(&right.col))
            .then(left.finding.tag.cmp(&right.finding.tag))
            .then(left.finding.message.cmp(&right.finding.message))
    });
    findings.dedup_by(|left, right| {
        left.line == right.line && left.col == right.col && left.finding == right.finding
    });
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, reason = "tests use fixed fixtures")]

    use ansilint_config::{Config, IgnoreFile};
    use ansilint_core::{Finding, Severity, Span, Tag};

    use super::{keep, PostContext};

    fn finding(tag: &'static str, severity: Severity) -> Finding {
        Finding::new(
            Tag::new(tag),
            "message",
            std::path::PathBuf::from("a.yml"),
            Span::new(0, 1),
            severity,
        )
    }

    #[test]
    fn noqa_drops_the_matching_tag() {
        let config = Config::default();
        let context = PostContext {
            config: &config,
            ignore: None,
        };
        let mut warning = Finding::new(
            Tag::with_sub("yaml", "truthy"),
            "message",
            std::path::PathBuf::from("a.yml"),
            Span::new(0, 1),
            Severity::Warning,
        );
        assert!(!keep(
            &mut warning,
            Some("a: 1  # noqa: yaml[truthy]"),
            &context
        ));
        let mut other = Finding::new(
            Tag::with_sub("yaml", "truthy"),
            "message",
            std::path::PathBuf::from("a.yml"),
            Span::new(0, 1),
            Severity::Warning,
        );
        assert!(keep(&mut other, Some("a: 1  # noqa: name"), &context));
    }

    #[test]
    fn parser_errors_are_not_skippable() {
        let config = Config::default();
        let context = PostContext {
            config: &config,
            ignore: None,
        };
        let mut error = finding("parser-error", Severity::Error);
        assert!(keep(
            &mut error,
            Some("bad  # noqa: parser-error"),
            &context
        ));
    }

    #[test]
    fn ignore_entries_skip_or_downgrade() {
        let config = Config::default();
        let ignore = IgnoreFile::parse("a.yml yaml skip\nb.yml name\n").expect("ignore file");
        let context = PostContext {
            config: &config,
            ignore: Some(&ignore),
        };
        let mut skipped = finding("yaml", Severity::Error);
        assert!(!keep(&mut skipped, None, &context));
        let mut downgraded = Finding::new(
            Tag::with_sub("name", "missing"),
            "message",
            std::path::PathBuf::from("b.yml"),
            Span::new(0, 1),
            Severity::Error,
        );
        assert!(keep(&mut downgraded, None, &context));
        assert_eq!(downgraded.severity, Severity::Warning);
    }

    #[test]
    fn tags_select_by_rule_metadata() {
        let config = Config {
            tags: vec!["syntax".to_string()],
            ..Config::default()
        };
        let context = PostContext {
            config: &config,
            ignore: None,
        };
        let mut parser_error = finding("parser-error", Severity::Error);
        assert!(keep(&mut parser_error, None, &context));
        let mut warning = finding("warning", Severity::Warning);
        assert!(!keep(&mut warning, None, &context));
    }

    #[test]
    fn skip_and_warn_lists_apply() {
        let config = Config {
            skip_list: vec!["yaml".to_string()],
            warn_list: vec!["name".to_string()],
            ..Config::default()
        };
        let context = PostContext {
            config: &config,
            ignore: None,
        };
        let mut skipped = finding("yaml", Severity::Warning);
        assert!(!keep(&mut skipped, None, &context));
        let mut warned = finding("name", Severity::Error);
        assert!(keep(&mut warned, None, &context));
        assert_eq!(warned.severity, Severity::Warning);
    }
}
