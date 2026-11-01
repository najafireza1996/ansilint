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

impl IgnoreEntry {
    /// Renders `<path> <tag>[ skip]` so that [`IgnoreFile::parse`] reads it
    /// back unchanged (H-C6). A path containing whitespace, `#`, `"` or `\\` is
    /// double-quoted with `\\` escapes; upstream does not define quoting.
    #[must_use]
    pub fn to_line(&self) -> String {
        let needs_quotes = self
            .path
            .chars()
            .any(|character| character.is_whitespace() || matches!(character, '#' | '"' | '\\'));
        let path = if needs_quotes {
            let mut quoted = String::with_capacity(self.path.len().saturating_add(2));
            quoted.push('"');
            for character in self.path.chars() {
                if matches!(character, '"' | '\\') {
                    quoted.push('\\');
                }
                quoted.push(character);
            }
            quoted.push('"');
            quoted
        } else {
            self.path.clone()
        };
        let skip = if self.skip { " skip" } else { "" };
        format!("{path} {}{skip}", self.tag)
    }
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
            let content = raw.trim();
            if content.is_empty() || content.starts_with('#') {
                continue;
            }
            let (path, rest) = split_path(line, content)?;
            let mut fields = rest
                .split('#')
                .next()
                .unwrap_or_default()
                .split_whitespace();
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
                path,
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

/// Splits the leading path (plain, or double-quoted with `\\` escapes) from
/// the rest of the line. A plain path ends at whitespace or `#`.
fn split_path(line: usize, content: &str) -> Result<(String, &str), IgnoreError> {
    let Some(quoted) = content.strip_prefix('"') else {
        let end = content
            .find(|character: char| character.is_whitespace() || character == '#')
            .unwrap_or(content.len());
        let (path, rest) = content.split_at(end);
        return Ok((path.to_string(), rest));
    };
    let mut path = String::new();
    let mut characters = quoted.char_indices();
    while let Some((index, character)) = characters.next() {
        match character {
            '\\' => {
                let (_, escaped) = characters
                    .next()
                    .ok_or_else(|| IgnoreError::new(line, "dangling `\\` in quoted path"))?;
                path.push(escaped);
            }
            '"' => {
                return Ok((
                    path,
                    quoted.get(index.saturating_add(1)..).unwrap_or_default(),
                ))
            }
            other => path.push(other),
        }
    }
    Err(IgnoreError::new(line, "unterminated quoted path"))
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

    use super::{IgnoreEntry, IgnoreFile};
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

    fn entry(path: &str, tag: &str, skip: bool) -> IgnoreEntry {
        IgnoreEntry {
            path: path.to_string(),
            tag: tag.to_string(),
            skip,
            line: 0,
        }
    }

    fn round_trip(entry: &IgnoreEntry) -> IgnoreEntry {
        let file = IgnoreFile::parse(&format!("{}  # note\n", entry.to_line())).expect("parses");
        assert_eq!(file.entries.len(), 1, "line: {}", entry.to_line());
        file.entries[0].clone()
    }

    #[test]
    fn round_trips_paths_with_spaces_hashes_and_quotes() {
        for path in [
            "my dir/a.yml",
            "a#b.yml",
            "q\"uote.yml",
            "back\\slash.yml",
            "plain.yml",
        ] {
            let original = entry(path, "yaml[truthy]", true);
            assert_eq!(round_trip(&original), original);
        }
        assert_eq!(
            entry("plain.yml", "name", false).to_line(),
            "plain.yml name"
        );
    }

    proptest::proptest! {
        #[test]
        fn parse_inverts_to_line(
            path in "[a-zA-Z0-9 ./_#\"\\\\-]{1,30}",
            tag in "[a-z-]{1,10}(\\[[a-z-]{1,8}\\])?",
            skip in proptest::bool::ANY,
        ) {
            proptest::prop_assume!(!path.trim().is_empty());
            let original = entry(&path, &tag, skip);
            proptest::prop_assert_eq!(round_trip(&original), original);
        }
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(IgnoreFile::parse("a.yml\n").is_err());
        assert!(IgnoreFile::parse("a.yml yaml maybe\n").is_err());
        assert!(IgnoreFile::parse("a.yml yaml skip extra\n").is_err());
    }
}
