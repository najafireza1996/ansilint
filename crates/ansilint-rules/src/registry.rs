//! The static rule registry.

use crate::rule::Rule;
use crate::rules::{InternalErrorRule, LoadFailureRule, ParserErrorRule, WarningRule};

macro_rules! register_rules {
    ($($rule:ident),* $(,)?) => {
        static RULES: &[&dyn Rule] = &[$( &$rule ),*];
    };
}

register_rules![
    InternalErrorRule,
    LoadFailureRule,
    ParserErrorRule,
    WarningRule
];

/// Returns every registered rule in deterministic order.
#[must_use]
pub fn all() -> &'static [&'static dyn Rule] {
    RULES
}
