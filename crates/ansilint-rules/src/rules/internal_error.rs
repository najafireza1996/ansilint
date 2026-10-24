//! The `internal-error` rule metadata; findings are emitted by the runner.

use ansilint_config::Profile;

use crate::meta::Tier;
use crate::rule::Rule;
use crate::RuleMeta;

const META: RuleMeta = RuleMeta {
    id: "internal-error",
    summary: "A rule panicked or failed internally",
    tags: &["internal"],
    profile: Profile::Min,
    opt_in: false,
    tier: Tier::Static,
    fixable: false,
    applies_to: &[],
};

/// Metadata marker for internal errors emitted by the runner.
pub struct InternalErrorRule;

impl Rule for InternalErrorRule {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }
}
