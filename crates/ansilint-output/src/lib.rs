//! Output formatters that render findings into a caller-provided sink.
//!
//! It must not: decide exit codes, discover files, or read global state.
#![forbid(unsafe_code)]
