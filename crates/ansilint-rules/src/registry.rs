//! The static rule registry.

use crate::rule::Rule;
use crate::rules::{InternalErrorRule, LoadFailureRule, ParserErrorRule, WarningRule};

macro_rules! register_rules {
    ($($(#[$attr:meta])* $rule:path),* $(,)?) => {
        static RULES: &[&dyn Rule] = &[$( $(#[$attr])* &$rule ),*];
    };
}

register_rules![
    InternalErrorRule,
    LoadFailureRule,
    ParserErrorRule,
    WarningRule,
    #[cfg(feature = "test-panic-rule")]
    crate::panic_probe::PanicProbeRule,
];

/// Returns every registered rule in deterministic order.
#[must_use]
pub fn all() -> &'static [&'static dyn Rule] {
    RULES
}
