//! Rule trait, rule registry and the rule implementations.
//!
//! It must not: perform I/O or mutate the model it inspects.
#![forbid(unsafe_code)]

mod meta;
mod registry;
mod rule;
mod rules;
mod runner;

pub use meta::{RuleMeta, Tier};
pub use registry::all;
pub use rule::{FileCx, Rule};
pub use rules::{InternalErrorRule, LoadFailureRule, ParserErrorRule, WarningRule};
pub use runner::run;
