//! The `parser-error` rule metadata; findings are emitted by the engine.

use ansilint_config::Profile;

use crate::meta::Tier;
use crate::rule::Rule;
use crate::RuleMeta;

const META: RuleMeta = RuleMeta {
    id: "parser-error",
    summary: "The YAML parser could not parse a file",
    tags: &["syntax"],
    profile: Profile::Min,
    opt_in: false,
    tier: Tier::Static,
    fixable: false,
    applies_to: &[],
};

/// Metadata marker for parser errors emitted by the engine.
pub struct ParserErrorRule;

impl Rule for ParserErrorRule {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }
}
