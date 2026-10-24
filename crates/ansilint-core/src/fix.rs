//! Span-based text edits and fixes.

use crate::Span;

/// A non-overlapping replacement of a byte range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// The range to replace.
    pub span: Span,
    /// The replacement text.
    pub replacement: String,
}

/// Whether a fix can be applied without an explicit opt-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applicability {
    /// Safe to apply with `--fix`.
    Safe,
    /// Requires `--fix=unsafe` (not part of 1.0).
    Unsafe,
}

/// A machine-applicable fix described as text edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    /// Short description shown by editors and `--diff`.
    pub description: String,
    /// Non-overlapping edits, applied back-to-front.
    pub edits: Vec<Edit>,
    /// Safety classification.
    pub applicability: Applicability,
}

impl Fix {
    /// Creates a safe fix from a description and edits.
    #[must_use]
    pub fn safe(description: impl Into<String>, edits: Vec<Edit>) -> Self {
        Self {
            description: description.into(),
            edits,
            applicability: Applicability::Safe,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Applicability, Edit, Fix};
    use crate::Span;

    #[test]
    fn safe_fixes_are_marked_safe() {
        let fix = Fix::safe(
            "replace the module name",
            vec![Edit {
                span: Span::new(0, 5),
                replacement: "ansible.builtin.copy".to_string(),
            }],
        );
        assert_eq!(fix.applicability, Applicability::Safe);
        assert_eq!(fix.edits.len(), 1);
    }
}
