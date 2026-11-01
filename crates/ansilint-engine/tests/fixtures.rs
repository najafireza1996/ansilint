//! H-C9: every rule fixture runs through the engine and must match its
//! checked-in `expected/<kind>-<case>.json`. Regenerate with
//! `ANSILINT_BLESS=1 cargo test -p ansilint-engine --all-features --test fixtures`
//! and review the diff like any snapshot (ENG-38).

#![allow(
    clippy::expect_used,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use std::path::{Path, PathBuf};

use ansilint_engine::{lint, LintOptions};
use ansilint_testing::fixture_cases;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn findings_json(input: &Path) -> String {
    let mut options = LintOptions::new(workspace_root());
    options.paths = vec![input.to_path_buf()];
    let findings: Vec<serde_json::Value> = lint(&options)
        .expect("lint fixture")
        .findings
        .iter()
        .map(|positioned| {
            serde_json::json!({
                "tag": positioned.finding.tag.to_string(),
                "line": positioned.line,
                "col": positioned.col,
                "severity": format!("{:?}", positioned.finding.severity),
                "message": positioned.finding.message,
            })
        })
        .collect();
    let mut json = serde_json::to_string_pretty(&findings).expect("serialize");
    json.push('\n');
    json
}

#[test]
fn every_rule_fixture_matches_its_expected_findings() {
    let bless = std::env::var_os("ANSILINT_BLESS").is_some();
    let cases = fixture_cases(&workspace_root().join("tests/fixtures")).expect("fixtures");
    assert!(!cases.is_empty(), "no rule fixtures found");
    let mut mismatches = Vec::new();
    for case in cases {
        // The panicking probe rule exists only with this feature (H-B4).
        if case.rule == "internal-error" && !cfg!(feature = "test-panic-rule") {
            continue;
        }
        let actual = findings_json(&case.input);
        if bless {
            std::fs::create_dir_all(case.expected.parent().expect("parent")).expect("mkdir");
            std::fs::write(&case.expected, &actual).expect("bless");
            continue;
        }
        let expected = std::fs::read_to_string(&case.expected).unwrap_or_default();
        if expected != actual {
            mismatches.push(format!(
                "{}\n--- expected ({})\n{expected}+++ actual\n{actual}",
                case.input.display(),
                case.expected.display()
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
