//! Dev-only fixture builders, in-memory fakes and contract-test macros.
//!
//! It must not: be a normal dependency of a shipped crate.
#![forbid(unsafe_code)]

/// Generates the shared contract tests for an implementation of a rule,
/// formatter, module index or schema provider (ENG-12).
///
/// The macro is intentionally empty in Phase 0; each suite lands with its first
/// real implementation.
#[macro_export]
macro_rules! contract_tests {
    ($($tokens:tt)*) => {};
}
