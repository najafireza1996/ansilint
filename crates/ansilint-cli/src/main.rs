//! The `ansilint` command-line binary.
//!
//! It must not: implement linting logic itself; it parses arguments and delegates
//! to `ansilint-engine` (and `ansilint-lsp` once the language server exists).
#![forbid(unsafe_code)]

use clap::Parser;

/// A fast, single-binary Ansible linter that needs no Python.
#[derive(Debug, Parser)]
#[command(name = "ansilint", version, about, long_about = None)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
