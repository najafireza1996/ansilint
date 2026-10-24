//! Comment discovery over source lines.
//!
//! This is a line-oriented scanner, not a full YAML lexer: a `#` starts a
//! comment when it is the first non-space character or follows whitespace
//! outside single/double quotes. A `#` inside a literal or folded block scalar
//! can be misinterpreted; that limitation is documented in `docs/OPEN_QUESTIONS.md`.

use ansilint_core::{LineIndex, Span};

use crate::Comment;

pub(crate) fn scan_comments(text: &str, line_index: &LineIndex) -> Vec<Comment> {
    let mut comments = Vec::new();
    for line in 0..line_index.line_count() {
        let Some(line_text) = line_index.line_text(text, line) else {
            continue;
        };
        let Some(offset) = comment_offset(line_text) else {
            continue;
        };
        let Some(line_span) = line_index.line_span(line) else {
            continue;
        };
        let Some(relative) = u32::try_from(offset).ok() else {
            continue;
        };
        let start = line_span.start.saturating_add(relative);
        let text = line_text.get(offset..).unwrap_or_default().to_string();
        comments.push(Comment {
            span: Span::new(start, line_span.end),
            text,
            line,
        });
    }
    comments
}

fn comment_offset(line: &str) -> Option<usize> {
    let mut single = false;
    let mut double = false;
    let mut escaped = false;
    let mut previous: Option<char> = None;
    for (index, character) in line.char_indices() {
        if double {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                double = false;
            }
        } else if single {
            if character == '\'' {
                single = false;
            }
        } else {
            match character {
                '"' => double = true,
                '\'' => single = true,
                '#' if previous.is_none_or(char::is_whitespace) => return Some(index),
                _ => {}
            }
        }
        previous = Some(character);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::comment_offset;

    #[test]
    fn finds_inline_and_full_line_comments() {
        assert_eq!(comment_offset("a: b # note"), Some(5));
        assert_eq!(comment_offset("   # note"), Some(3));
        assert_eq!(comment_offset("# note"), Some(0));
    }

    #[test]
    fn ignores_hashes_inside_quotes() {
        assert_eq!(comment_offset("a: \"b # c\""), None);
        assert_eq!(comment_offset("a: 'b # c'"), None);
        assert_eq!(comment_offset("a: \"b \\\" # c\""), None);
    }

    #[test]
    fn ignores_hashes_without_leading_space() {
        assert_eq!(comment_offset("a: b#c"), None);
    }
}
