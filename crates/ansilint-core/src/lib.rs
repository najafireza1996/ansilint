//! Shared value types: spans, positions, line indices, findings, tags and fixes.
//!
//! It must not: parse YAML, read files, or know anything about Ansible semantics.
#![forbid(unsafe_code)]

mod finding;
mod fix;
mod line_index;
mod span;

pub use finding::{Finding, Severity, Tag};
pub use fix::{Applicability, Edit, Fix};
pub use line_index::{LineCol, LineIndex};
pub use span::Span;
