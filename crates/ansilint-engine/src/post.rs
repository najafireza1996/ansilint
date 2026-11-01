//! Post-processing: inline `# noqa`, ignore files and configuration lists.

use std::path::Path;

use ansilint_config::{Config, IgnoreFile};
use ansilint_core::{Finding, Severity, Tag};
use ansilint_yaml::{Document, SourceFile};

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
///
/// Non-skippable findings (H-D2) are never dropped by `# noqa`, `skip_list`,
/// an ignore-file `skip` entry or a `tags` filter; ignore entries and
/// `warn_list` may still downgrade them. Severity only ever goes down (H-D3).
#[must_use]
pub(crate) fn keep(
    finding: &mut Finding,
    line_comment: Option<&str>,
    context: &PostContext<'_>,
) -> bool {
    let skippable = !NON_SKIPPABLE.contains(&finding.tag.rule);
    if skippable && line_comment.is_some_and(|comment| noqa_matches(comment, &finding.tag)) {
        return false;
    }
    if let Some(ignore) = context.ignore {
        let path = finding.path.to_string_lossy().replace('\\', "/");
        match ignore.disposition(&path, &finding.tag) {
            Some(true) if skippable => return false,
            Some(_) => downgrade(finding),
            None => {}
        }
    }
    if skippable && matches_any(&finding.tag, &context.config.skip_list) {
        return false;
    }
    if skippable
        && !context.config.tags.is_empty()
        && !tag_selected(&finding.tag, &context.config.tags)
    {
        return false;
    }
    if matches_any(&finding.tag, &context.config.warn_list) {
        downgrade(finding);
    }
    true
}

/// Lowers a finding to at most a warning; `Info` stays `Info`.
fn downgrade(finding: &mut Finding) {
    finding.severity = finding.severity.min(Severity::Warning);
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

/// Notices for `tags` selectors that no registered rule can satisfy, so a
/// typo never silently yields an empty report (H-B1).
pub(crate) fn unmatched_tag_notices(patterns: &[String]) -> Vec<String> {
    patterns
        .iter()
        .map(|pattern| pattern.trim())
        .filter(|pattern| {
            let rule_id = pattern.split('[').next().unwrap_or(pattern);
            !ansilint_rules::all().iter().any(|rule| {
                let meta = rule.meta();
                meta.id == rule_id || meta.tags.contains(pattern)
            })
        })
        .map(|pattern| format!("tag selector `{pattern}` matches no rule"))
        .collect()
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

/// `documents` supply the comment tokens for `# noqa` (H-D1): text that only
/// looks like a comment inside a scalar never suppresses a finding.
pub(crate) fn process(
    findings: Vec<Finding>,
    source: Option<&SourceFile>,
    documents: &[Document],
    output_path: &Path,
    context: &PostContext<'_>,
) -> Vec<PositionedFinding> {
    let mut results = Vec::new();
    for mut finding in findings {
        finding.path = output_path.to_path_buf();
        let (line, col, line_comment) = match source {
            Some(source) => {
                let position = source
                    .line_index
                    .line_col(source.as_str(), finding.span.start);
                let index = usize::try_from(position.line.saturating_sub(1)).unwrap_or(usize::MAX);
                (
                    position.line,
                    position.col,
                    documents
                        .iter()
                        .find_map(|document| document.comments_on_line(index).next())
                        .map(|comment| comment.text.as_str()),
                )
            }
            None => (1, 1, None),
        };
        if keep(&mut finding, line_comment, context) {
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
    fn non_skippable_findings_survive_every_drop_request() {
        let ignore = IgnoreFile::parse("a.yml parser-error skip\n").expect("ignore file");
        let config = Config {
            skip_list: vec!["load-failure".to_string()],
            tags: vec!["warning".to_string()],
            ..Config::default()
        };
        let context = PostContext {
            config: &config,
            ignore: Some(&ignore),
        };
        let mut parser_error = finding("parser-error", Severity::Error);
        assert!(keep(&mut parser_error, None, &context), "ignore skip");
        assert_eq!(parser_error.severity, Severity::Warning, "but downgraded");
        let mut load_failure = finding("load-failure", Severity::Error);
        assert!(keep(&mut load_failure, None, &context), "skip_list / tags");
        let mut internal = finding("internal-error", Severity::Error);
        assert!(keep(&mut internal, None, &context), "tags filter");
    }

    #[test]
    fn severity_is_only_ever_lowered() {
        let ignore = IgnoreFile::parse("a.yml name\n").expect("ignore file");
        let config = Config {
            warn_list: vec!["yaml".to_string()],
            ..Config::default()
        };
        let context = PostContext {
            config: &config,
            ignore: Some(&ignore),
        };
        let mut warned = finding("yaml", Severity::Info);
        assert!(keep(&mut warned, None, &context));
        assert_eq!(warned.severity, Severity::Info, "warn_list must not raise");
        let mut ignored = finding("name", Severity::Info);
        assert!(keep(&mut ignored, None, &context));
        assert_eq!(
            ignored.severity,
            Severity::Info,
            "ignore file must not raise"
        );
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
