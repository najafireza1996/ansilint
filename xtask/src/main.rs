//! Developer task runner for the ansilint workspace.
//!
//! It must not: contain product logic. This crate only automates checks,
//! generation and local developer workflows.
#![forbid(unsafe_code)]
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "xtask reports progress and failures to the developer"
)]

mod check;
mod commands;
mod commits;
mod generate;
mod workspace;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match commands::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error:#}");
            ExitCode::FAILURE
        }
    }
}
