//! H-A4: the permanent false-positive gate. Every file under
//! `tests/fixtures/_valid/` is valid input and must lint with zero findings.

#![allow(
    clippy::expect_used,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use std::path::Path;

use ansilint_engine::{lint, LintOptions};

const MIN_CORPUS_FILES: usize = 30;

#[test]
fn valid_corpus_lints_with_zero_findings() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/_valid")
        .canonicalize()
        .expect("tests/fixtures/_valid exists");
    let result = lint(&LintOptions::new(&corpus)).expect("lint corpus");
    assert!(
        result.files_linted >= MIN_CORPUS_FILES,
        "corpus shrank to {} files",
        result.files_linted
    );
    let findings: Vec<String> = result
        .findings
        .iter()
        .map(|positioned| {
            format!(
                "{}:{}:{} {} {}",
                positioned.finding.path.display(),
                positioned.line,
                positioned.col,
                positioned.finding.tag,
                positioned.finding.message
            )
        })
        .collect();
    assert!(
        findings.is_empty(),
        "false positives:\n{}",
        findings.join("\n")
    );
}
