//! The upstream `warning` meta-rule for non-fatal notices.

use ansilint_core::{Finding, Severity, Span, Tag};
use ansilint_model::FileKind;

use crate::meta::Tier;
use crate::rule::{FileCx, Rule};
use crate::RuleMeta;
use ansilint_config::Profile;

const META: RuleMeta = RuleMeta {
    id: "warning",
    summary: "Non-fatal warning emitted by the linter",
    tags: &["warning"],
    profile: Profile::Min,
    opt_in: false,
    tier: Tier::Static,
    fixable: false,
    applies_to: &[],
};

/// Emits a warning for empty playbooks.
pub struct WarningRule;

impl Rule for WarningRule {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_file(&self, cx: &FileCx<'_>, out: &mut Vec<Finding>) {
        if cx.kind != FileKind::Playbook {
            return;
        }
        let plays = cx.built.model.plays();
        let empty = plays.iter().all(|play| {
            play.pre_tasks.is_empty()
                && play.tasks.is_empty()
                && play.post_tasks.is_empty()
                && play.handlers.is_empty()
                && play.roles.is_empty()
        });
        if !empty {
            return;
        }
        let span = plays.first().map_or_else(
            || {
                cx.documents
                    .first()
                    .map_or(Span::empty(0), |document| document.root.span())
            },
            |play| play.span,
        );
        out.push(Finding::new(
            Tag::new("warning"),
            "empty playbook",
            cx.source.path.clone(),
            span,
            Severity::Warning,
        ));
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::assert_is_empty,
        clippy::expect_used,
        clippy::indexing_slicing,
        reason = "tests use fixed fixtures and panic on broken assertions"
    )]

    use ansilint_config::Config;
    use ansilint_model::FileKind;
    use ansilint_yaml::SourceFile;

    use super::WarningRule;
    use crate::rule::{FileCx, Rule};

    fn findings(text: &str) -> Vec<ansilint_core::Finding> {
        let source = SourceFile::new("test.yml", text);
        let documents = ansilint_yaml::parse(&source).expect("valid YAML");
        let built = ansilint_model::build(&documents, FileKind::Playbook);
        let config = Config::default();
        let cx = FileCx {
            source: &source,
            documents: &documents,
            kind: FileKind::Playbook,
            built: &built,
            config: &config,
        };
        let mut out = Vec::new();
        WarningRule.check_file(&cx, &mut out);
        out
    }

    #[test]
    fn warns_for_an_empty_playbook() {
        let found = findings("- hosts: all\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].tag.rule, "warning");
    }

    #[test]
    fn ignores_playbooks_with_tasks() {
        let found = findings(
            "- hosts: all\n  tasks:\n    - name: hello\n      ansible.builtin.debug:\n        msg: hello\n",
        );
        assert!(found.is_empty());
    }
}
