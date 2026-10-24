//! Lossless-enough YAML AST with source spans, comments and multi-document support.
//!
//! It must not: interpret Ansible semantics or perform filesystem I/O.
#![forbid(unsafe_code)]

mod ast;
mod comments;
mod error;
mod parser;

pub use ast::{
    Alias, Comment, Document, Mapping, MappingEntry, Node, Scalar, ScalarStyle, Sequence,
    SourceFile,
};
pub use error::YamlError;
pub use parser::parse;
