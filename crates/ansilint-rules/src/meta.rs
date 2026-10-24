//! Rule metadata used by listings, profiles and parity generation.

use ansilint_config::Profile;
use ansilint_model::FileKind;

/// What a rule needs to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Pure YAML/text.
    Static,
    /// Needs the module index.
    NeedsIndex,
    /// Needs JSON schemas.
    NeedsSchema,
    /// Needs the Jinja scanner.
    NeedsJinja,
    /// Requires running Ansible and is not planned.
    Runtime,
}

/// Static metadata for one rule.
#[derive(Clone, Copy, Debug)]
pub struct RuleMeta {
    /// Stable rule id.
    pub id: &'static str,
    /// One-line summary.
    pub summary: &'static str,
    /// Upstream tags.
    pub tags: &'static [&'static str],
    /// Minimum profile that includes the rule.
    pub profile: Profile,
    /// Whether the rule is only active via `enable_list`.
    pub opt_in: bool,
    /// What the rule needs.
    pub tier: Tier,
    /// Whether the rule can emit fixes.
    pub fixable: bool,
    /// File kinds the rule applies to; empty means every kind.
    pub applies_to: &'static [FileKind],
}

impl RuleMeta {
    /// Returns `true` when the rule applies to `kind`.
    #[must_use]
    pub fn applies(&self, kind: FileKind) -> bool {
        self.applies_to.is_empty() || self.applies_to.contains(&kind)
    }
}
