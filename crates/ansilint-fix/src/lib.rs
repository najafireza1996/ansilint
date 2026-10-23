//! Span-based edit planning, conflict resolution and diff generation.
//!
//! It must not: write files; applying edits belongs to the engine.
#![forbid(unsafe_code)]
