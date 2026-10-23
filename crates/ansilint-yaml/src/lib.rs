//! Lossless-enough YAML AST with source spans, comments and multi-document support.
//!
//! It must not: interpret Ansible semantics or perform filesystem I/O.
#![forbid(unsafe_code)]
