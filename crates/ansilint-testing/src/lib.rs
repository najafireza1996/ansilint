//! Dev-only fixture builders, in-memory fakes and contract-test macros.
//!
//! It must not: be a normal dependency of a shipped crate.
#![forbid(unsafe_code)]

/// Generates contract tests for a rule implementation (ENG-12).
///
/// The macro checks the invariants every [`Rule`] must satisfy: a non-empty id,
/// summary and tag list, no duplicate tags, and no whitespace in the id. Call it
/// from an integration test where the `Rule` trait is in scope.
///
/// [`Rule`]: https://docs.rs/ansilint-rules
#[macro_export]
macro_rules! rule_contract_tests {
    ($name:ident, $rule:expr) => {
        #[test]
        #[allow(
            clippy::assert_is_empty,
            clippy::assertions_on_constants,
            reason = "contract assertions intentionally check fixed invariants"
        )]
        fn $name() {
            let rule = $rule;
            let meta = rule.meta();
            assert!(!meta.id.is_empty(), "rule id must not be empty");
            assert!(!meta.summary.is_empty(), "rule summary must not be empty");
            assert!(!meta.tags.is_empty(), "rule tags must not be empty");
            assert!(
                !meta.id.contains(char::is_whitespace),
                "rule id must not contain whitespace"
            );
            let mut tags = meta.tags.to_vec();
            tags.sort_unstable();
            tags.dedup();
            assert_eq!(
                tags.len(),
                meta.tags.len(),
                "rule tags must not contain duplicates"
            );
        }
    };
}
