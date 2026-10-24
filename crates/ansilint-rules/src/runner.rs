//! Rule execution with panic isolation.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};

use ansilint_core::{Finding, Severity, Tag};
use ansilint_model::{Block, FileModel, Play, TaskItem};

use crate::rule::{FileCx, Rule};

/// Runs rules against one file. A panicking rule becomes an `internal-error`.
#[must_use]
pub fn run(rules: &[&dyn Rule], cx: &FileCx<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for rule in rules {
        let mut rule_findings = Vec::new();
        let result = catch_unwind(AssertUnwindSafe(|| {
            visit_rule(*rule, cx, &mut rule_findings);
        }));
        if let Err(payload) = result {
            rule_findings.push(internal_error(cx, rule.meta().id, &payload));
        }
        findings.extend(rule_findings);
    }
    findings
}

fn visit_rule(rule: &dyn Rule, cx: &FileCx<'_>, out: &mut Vec<Finding>) {
    if !rule.meta().applies(cx.kind) {
        return;
    }
    rule.check_file(cx, out);
    rule.check_text(cx, out);
    match &cx.built.model {
        FileModel::Playbook(plays) => visit_plays(rule, cx, plays, out),
        FileModel::Tasks(items) | FileModel::Handlers(items) => visit_items(rule, cx, items, out),
        FileModel::Other | FileModel::Unknown => {}
    }
}

fn visit_plays(rule: &dyn Rule, cx: &FileCx<'_>, plays: &[Play], out: &mut Vec<Finding>) {
    for play in plays {
        rule.check_play(cx, play, out);
        visit_items(rule, cx, &play.pre_tasks, out);
        visit_items(rule, cx, &play.tasks, out);
        visit_items(rule, cx, &play.post_tasks, out);
        visit_items(rule, cx, &play.handlers, out);
    }
}

fn visit_items(rule: &dyn Rule, cx: &FileCx<'_>, items: &[TaskItem], out: &mut Vec<Finding>) {
    for item in items {
        match item {
            TaskItem::Task(task) | TaskItem::IncludeImport(task) => rule.check_task(cx, task, out),
            TaskItem::Block(block) => visit_block(rule, cx, block, out),
        }
    }
}

fn visit_block(rule: &dyn Rule, cx: &FileCx<'_>, block: &Block, out: &mut Vec<Finding>) {
    rule.check_block(cx, block, out);
    visit_items(rule, cx, &block.block, out);
    visit_items(rule, cx, &block.rescue, out);
    visit_items(rule, cx, &block.always, out);
}

fn internal_error(cx: &FileCx<'_>, rule: &str, payload: &Box<dyn Any + Send>) -> Finding {
    let detail = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("rule panicked");
    let span = cx
        .documents
        .first()
        .map_or(ansilint_core::Span::empty(0), |document| {
            document.root.span()
        });
    Finding::new(
        Tag::new("internal-error"),
        format!("rule `{rule}` panicked: {detail}"),
        cx.source.path.clone(),
        span,
        Severity::Error,
    )
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "tests use fixed fixtures and panic on broken assertions"
    )]

    use ansilint_config::Config;
    use ansilint_model::FileKind;
    use ansilint_yaml::SourceFile;

    use super::run;
    use crate::rule::{FileCx, Rule};
    use crate::{RuleMeta, Tier};

    struct PanicRule;

    static PANIC_META: RuleMeta = RuleMeta {
        id: "test-panic",
        summary: "panics on purpose",
        tags: &[],
        profile: ansilint_config::Profile::Min,
        opt_in: false,
        tier: Tier::Static,
        fixable: false,
        applies_to: &[],
    };

    impl Rule for PanicRule {
        fn meta(&self) -> &'static RuleMeta {
            &PANIC_META
        }

        fn check_file(&self, _cx: &FileCx<'_>, _out: &mut Vec<ansilint_core::Finding>) {
            std::panic::panic_any("boom");
        }
    }

    #[test]
    fn converts_panics_into_internal_errors() {
        let source = SourceFile::new("a.yml", "a: 1\n");
        let documents = ansilint_yaml::parse(&source).expect("valid YAML");
        let built = ansilint_model::build(&documents, FileKind::Yaml);
        let config = Config::default();
        let cx = FileCx {
            source: &source,
            documents: &documents,
            kind: FileKind::Yaml,
            built: &built,
            config: &config,
        };
        let findings = run(&[&PanicRule], &cx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].tag.rule, "internal-error");
        assert!(findings[0].message.contains("test-panic"));
    }
}

#[cfg(test)]
mod string_payload_tests {
    #![allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "tests use fixed fixtures and panic on broken assertions"
    )]

    use ansilint_config::Config;
    use ansilint_model::FileKind;
    use ansilint_yaml::SourceFile;

    use super::run;
    use crate::rule::{FileCx, Rule};
    use crate::{RuleMeta, Tier};

    struct StringPanicRule;

    static STRING_PANIC_META: RuleMeta = RuleMeta {
        id: "test-string-panic",
        summary: "panics with a String on purpose",
        tags: &[],
        profile: ansilint_config::Profile::Min,
        opt_in: false,
        tier: Tier::Static,
        fixable: false,
        applies_to: &[],
    };

    impl Rule for StringPanicRule {
        fn meta(&self) -> &'static RuleMeta {
            &STRING_PANIC_META
        }

        fn check_file(&self, _cx: &FileCx<'_>, _out: &mut Vec<ansilint_core::Finding>) {
            std::panic::panic_any(String::from("boom"));
        }
    }

    #[test]
    fn extracts_string_panic_payloads() {
        let source = SourceFile::new("a.yml", "a: 1\n");
        let documents = ansilint_yaml::parse(&source).expect("valid YAML");
        let built = ansilint_model::build(&documents, FileKind::Yaml);
        let config = Config::default();
        let cx = FileCx {
            source: &source,
            documents: &documents,
            kind: FileKind::Yaml,
            built: &built,
            config: &config,
        };
        let findings = run(&[&StringPanicRule], &cx);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("boom"));
    }
}
