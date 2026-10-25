//! Fixture-driven regression tests for the min-profile rules.

#![allow(
    clippy::assert_is_empty,
    clippy::expect_used,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use std::path::{Path, PathBuf};

use ansilint_engine::{lint, LintOptions};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn fixture(rule: &str, kind: &str, name: &str) -> PathBuf {
    workspace_root()
        .join("tests/fixtures")
        .join(rule)
        .join(kind)
        .join(name)
}

fn findings_for(path: &Path) -> Vec<String> {
    let mut options = LintOptions::new(workspace_root());
    options.paths = vec![path.to_path_buf()];
    lint(&options)
        .expect("lint fixture")
        .findings
        .into_iter()
        .map(|finding| finding.finding.tag.to_string())
        .collect()
}

#[test]
fn parser_error_fixtures() {
    assert_eq!(
        findings_for(&fixture("parser-error", "fail", "bad.yml")),
        ["parser-error"]
    );
    assert!(findings_for(&fixture("parser-error", "pass", "ok.yml")).is_empty());
    assert_eq!(
        findings_for(&fixture("parser-error", "noqa", "bad.yml")),
        ["parser-error"]
    );
}

#[test]
fn load_failure_fixtures() {
    assert_eq!(
        findings_for(&fixture("load-failure", "fail", "non-utf8.yml")),
        ["load-failure"]
    );
    assert!(findings_for(&fixture("load-failure", "pass", "ok.yml")).is_empty());
    assert_eq!(
        findings_for(&fixture("load-failure", "noqa", "non-utf8.yml")),
        ["load-failure"]
    );
}

#[test]
fn warning_fixtures() {
    assert_eq!(
        findings_for(&fixture("warning", "fail", "empty-playbook.yml")),
        ["warning"]
    );
    assert!(findings_for(&fixture("warning", "pass", "with-task.yml")).is_empty());
    assert!(findings_for(&fixture("warning", "noqa", "empty-with-noqa.yml")).is_empty());
}
