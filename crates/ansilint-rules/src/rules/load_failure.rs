//! The `load-failure` rule metadata; findings are emitted by the engine.

use ansilint_config::Profile;

use crate::meta::Tier;
use crate::rule::Rule;
use crate::RuleMeta;

const META: RuleMeta = RuleMeta {
    id: "load-failure",
    summary: "A file could not be read as UTF-8 text",
    tags: &["syntax"],
    profile: Profile::Min,
    opt_in: false,
    tier: Tier::Static,
    fixable: false,
    applies_to: &[],
};

/// Metadata marker for load failures emitted by the engine.
pub struct LoadFailureRule;

impl Rule for LoadFailureRule {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }
}
