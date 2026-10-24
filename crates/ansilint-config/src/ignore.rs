//! Parsing of `.ansible-lint-ignore` files.

use ansilint_core::Tag;
use thiserror::Error;

/// Ignore-file names, in discovery order.
pub const IGNORE_FILE_NAMES: &[&str] = &[".ansible-lint-ignore", ".config/ansible-lint-ignore.txt"];

/// One ignore-file line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnoreEntry {
    /// Path exactly as written in the file.
    pub path: String,
    /// Tag pattern: `rule` or `rule[sub]`.
    pub tag: String,
    /// When true the finding disappears; otherwise it is downgraded to a warning.
    pub skip: bool,
    /// Zero-based line number.
    pub line: usize,
}

/// The parsed ignore file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IgnoreFile {
    /// Entries in file order.
    pub entries: Vec<IgnoreEntry>,
}

impl IgnoreFile {
    /// Parses ignore-file contents.
    ///
    /// # Errors
    ///
    /// Returns an error for a line that is not `<path> <tag> [skip]`.
    pub fn parse(text: &str) -> Result<Self, IgnoreError> {
        let mut entries = Vec::new();
        for (line, raw) in text.lines().enumerate() {
            let content = raw.split('#').next().unwrap_or_default().trim();
            if content.is_empty() {
                continue;
            }
            let mut fields = content.split_whitespace();
            let Some(path) = fields.next() else {
                continue;
            };
            let Some(tag) = fields.next() else {
                return Err(IgnoreError::new(line, "missing tag"));
            };
            let skip = match fields.next() {
                Some("skip") => true,
                Some(other) => {
                    return Err(IgnoreError::new(
                        line,
                        format!("unexpected token `{other}`; expected `skip`"),
                    ));
                }
                None => false,
            };
            if fields.next().is_some() {
                return Err(IgnoreError::new(line, "too many fields"));
            }
            entries.push(IgnoreEntry {
                path: path.to_string(),
                tag: tag.to_string(),
                skip,
                line,
            });
        }
        Ok(Self { entries })
    }

    /// Returns `Some(true)` for skip, `Some(false)` for downgrade, `None` if no
    /// entry applies.
    #[must_use]
    pub fn disposition(&self, path: &str, tag: &Tag) -> Option<bool> {
        let mut downgrade = false;
        for entry in &self.entries {
            if entry.path != path || !tag.matches(&entry.tag) {
                continue;
            }
            if entry.skip {
                return Some(true);
            }
            downgrade = true;
        }
        downgrade.then_some(false)
    }
}

/// An invalid ignore-file line.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("ignore file line {}: {message}", line + 1)]
pub struct IgnoreError {
    /// Zero-based line number.
    pub line: usize,
    /// Explanation.
    pub message: String,
}

impl IgnoreError {
    /// Creates an error for a line.
    #[must_use]
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        reason = "tests use fixed fixtures and panic on broken assertions"
    )]

    use super::IgnoreFile;
    use ansilint_core::Tag;

    #[test]
    fn parses_skip_and_downgrade_entries() {
        let file = IgnoreFile::parse(
            "playbooks/a.yml yaml[truthy]  # comment\nroles/x/tasks/main.yml name skip\n",
        )
        .expect("valid ignore file");
        assert_eq!(file.entries.len(), 2);
        assert!(!file.entries[0].skip);
        assert!(file.entries[1].skip);
    }

    #[test]
    fn skip_wins_over_downgrade() {
        let file = IgnoreFile::parse("a.yml yaml\na.yml yaml skip\n").expect("valid");
        assert_eq!(
            file.disposition("a.yml", &Tag::with_sub("yaml", "truthy")),
            Some(true)
        );
    }

    #[test]
    fn bare_rule_matches_sub_tags() {
        let file = IgnoreFile::parse("a.yml yaml skip\n").expect("valid");
        assert_eq!(
            file.disposition("a.yml", &Tag::with_sub("yaml", "line-length")),
            Some(true)
        );
        assert_eq!(file.disposition("b.yml", &Tag::new("yaml")), None);
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(IgnoreFile::parse("a.yml\n").is_err());
        assert!(IgnoreFile::parse("a.yml yaml maybe\n").is_err());
        assert!(IgnoreFile::parse("a.yml yaml skip extra\n").is_err());
    }
}
