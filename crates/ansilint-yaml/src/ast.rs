//! The span-carrying YAML AST.

use std::path::PathBuf;
use std::sync::Arc;

use ansilint_core::{LineIndex, Span};

/// A source file held in memory exactly once.
#[derive(Clone, Debug)]
pub struct SourceFile {
    /// Path as given by the caller.
    pub path: PathBuf,
    /// The full UTF-8 source.
    pub text: Arc<str>,
    /// Byte-to-line index for the source.
    pub line_index: LineIndex,
}

impl SourceFile {
    /// Creates a source file and indexes its lines.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, text: impl Into<Arc<str>>) -> Self {
        let text = text.into();
        let line_index = LineIndex::new(&text);
        Self {
            path: path.into(),
            text,
            line_index,
        }
    }

    /// Returns the source as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// The style a scalar was written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScalarStyle {
    /// Unquoted.
    Plain,
    /// `'...'`.
    SingleQuoted,
    /// `"..."`.
    DoubleQuoted,
    /// `|` block scalar.
    Literal,
    /// `>` block scalar.
    Folded,
}

/// A scalar value with its decoded text and source span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scalar {
    /// Decoded value.
    pub value: String,
    /// Original style.
    pub style: ScalarStyle,
    /// Span of the scalar token in the source.
    pub span: Span,
    /// Anchor id when the scalar is anchored; `None` when it has no anchor.
    pub anchor: Option<usize>,
}

/// A YAML alias (`*name`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alias {
    /// Anchor id this alias refers to.
    pub anchor: usize,
    /// Span of the alias token.
    pub span: Span,
}

/// A YAML sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sequence {
    /// Items in source order.
    pub items: Vec<Node>,
    /// Span covering the sequence.
    pub span: Span,
    /// Anchor id when the sequence is anchored.
    pub anchor: Option<usize>,
}

/// A mapping entry; keys are nodes because YAML allows complex keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MappingEntry {
    /// Key node.
    pub key: Node,
    /// Value node.
    pub value: Node,
    /// Span of the key token.
    pub key_span: Span,
    /// Span of the value token.
    pub value_span: Span,
}

/// A YAML mapping that preserves source order and duplicate keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapping {
    /// Entries in source order, duplicates included.
    pub entries: Vec<MappingEntry>,
    /// Span covering the mapping.
    pub span: Span,
    /// Anchor id when the mapping is anchored.
    pub anchor: Option<usize>,
}

/// A YAML node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    /// A non-null scalar.
    Scalar(Scalar),
    /// A sequence.
    Sequence(Sequence),
    /// A mapping.
    Mapping(Mapping),
    /// An alias reference.
    Alias(Alias),
    /// An explicit or implicit null.
    Null {
        /// Span of the null token (can be empty).
        span: Span,
    },
}

impl Node {
    /// Returns the node's span.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Scalar(scalar) => scalar.span,
            Self::Sequence(sequence) => sequence.span,
            Self::Mapping(mapping) => mapping.span,
            Self::Alias(alias) => alias.span,
            Self::Null { span } => *span,
        }
    }

    /// Returns the scalar text, if this node is a non-null scalar.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Scalar(scalar) => Some(&scalar.value),
            _ => None,
        }
    }

    /// Returns the mapping, if this node is one.
    #[must_use]
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Self::Mapping(mapping) => Some(mapping),
            _ => None,
        }
    }

    /// Returns the sequence, if this node is one.
    #[must_use]
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self {
            Self::Sequence(sequence) => Some(sequence),
            _ => None,
        }
    }

    /// Returns the alias, if this node is one.
    #[must_use]
    pub fn as_alias(&self) -> Option<&Alias> {
        match self {
            Self::Alias(alias) => Some(alias),
            _ => None,
        }
    }

    /// Returns `true` for null nodes.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null { .. })
    }
}

impl Mapping {
    /// Returns the first value for a scalar key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.entries
            .iter()
            .find(|entry| entry.key.as_str() == Some(key))
            .map(|entry| &entry.value)
    }

    /// Returns every entry with a scalar key equal to `key`, including duplicates.
    pub fn get_all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a MappingEntry> {
        self.entries
            .iter()
            .filter(move |entry| entry.key.as_str() == Some(key))
    }
}

/// A comment attached to a source line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    /// Span from `#` to the end of the line.
    pub span: Span,
    /// Raw comment text including `#`.
    pub text: String,
    /// Zero-based line number.
    pub line: usize,
}

impl Comment {
    /// Returns the text after `#`, trimmed.
    #[must_use]
    pub fn content(&self) -> &str {
        self.text
            .strip_prefix('#')
            .map_or(self.text.as_str(), str::trim)
    }
}

/// One YAML document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    /// Root node.
    pub root: Node,
    /// Comments in the whole source, shared by every document.
    pub comments: Vec<Comment>,
    /// Span of an explicit `---` marker.
    pub doc_start: Option<Span>,
    /// Span of an explicit `...` marker.
    pub doc_end: Option<Span>,
    /// Whether the document started with `---`.
    pub explicit_start: bool,
}

impl Document {
    /// Returns comments on a zero-based line.
    pub fn comments_on_line(&self, line: usize) -> impl Iterator<Item = &Comment> {
        self.comments
            .iter()
            .filter(move |comment| comment.line == line)
    }
}
