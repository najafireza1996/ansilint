//! Test-only rule that panics, to prove panic isolation end to end (H-B4).
//! Compiled only with the `test-panic-rule` feature; never shipped.

use ansilint_config::Profile;
use ansilint_core::Finding;

use crate::meta::Tier;
use crate::rule::{FileCx, Rule};
use crate::RuleMeta;

/// Files containing this text make the probe panic; nothing else does, so
/// `--all-features` test runs stay unaffected.
const MARKER: &str = "# ansilint-test-panic";

const META: RuleMeta = RuleMeta {
    id: "test-panic-probe",
    summary: "Panics on files marked for the panic-isolation test",
    tags: &["internal"],
    profile: Profile::Min,
    opt_in: false,
    tier: Tier::Static,
    fixable: false,
    applies_to: &[],
};

/// Panics on files containing [`MARKER`].
pub(crate) struct PanicProbeRule;

impl Rule for PanicProbeRule {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_file(&self, cx: &FileCx<'_>, _out: &mut Vec<Finding>) {
        assert!(
            !cx.source.as_str().contains(MARKER),
            "panic probe triggered"
        );
    }
}
