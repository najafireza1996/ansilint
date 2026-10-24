//! End-to-end tests for the lint pipeline.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use std::fs;

use ansilint_config::ConfigOverrides;
use ansilint_engine::{generate_ignore, lint, LintOptions};
use ansilint_output::Format;

fn write(path: &std::path::Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write fixture");
}

fn options(directory: &std::path::Path) -> LintOptions {
    LintOptions::new(directory)
}

#[test]
fn warns_about_an_empty_playbook() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    let result = lint(&options(directory.path())).expect("lint");
    assert_eq!(result.files_linted, 1);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].finding.tag.rule, "warning");
    assert_eq!(result.findings[0].line, 1);
    assert_eq!(
        result.findings[0].finding.path,
        std::path::PathBuf::from("site.yml")
    );
}

#[test]
fn reports_parser_errors_with_a_position() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("bad.yml"), "a: [1,\n");
    let result = lint(&options(directory.path())).expect("lint");
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].finding.tag.rule, "parser-error");
    assert!(result.has_fatal(false));
}

#[test]
fn noqa_silences_skippable_findings() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site.yml"),
        "- hosts: all  # noqa: warning\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
}

#[test]
fn ignore_file_can_skip_a_finding() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    write(
        &directory.path().join(".ansible-lint-ignore"),
        "site.yml warning skip\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
}

#[test]
fn warn_list_downgrades_errors() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("bad.yml"), "a: [1,\n");
    let mut options = options(directory.path());
    options.overrides = ConfigOverrides {
        warn_list: vec!["parser-error".to_string()],
        ..ConfigOverrides::default()
    };
    let result = lint(&options).expect("lint");
    assert!(!result.has_fatal(false));
    assert!(result.has_fatal(true));
}

#[test]
fn renders_pep8_and_json_output() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    let result = lint(&options(directory.path())).expect("lint");
    let mut pep8 = Vec::new();
    result.render(Format::Pep8, &mut pep8).expect("render pep8");
    let pep8 = String::from_utf8(pep8).expect("UTF-8");
    assert!(pep8.contains("site.yml:1: warning: empty playbook"));
    let mut json = Vec::new();
    result.render(Format::Json, &mut json).expect("render json");
    let json = String::from_utf8(json).expect("UTF-8");
    assert!(json.contains("\"tag\":\"warning\""));
}

#[test]
fn generates_an_ignore_file() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    let result = lint(&options(directory.path())).expect("lint");
    let ignore = directory.path().join(".ansible-lint-ignore");
    generate_ignore(&result, &ignore).expect("generate ignore");
    let contents = fs::read_to_string(ignore).expect("read ignore");
    assert!(contents.contains("site.yml warning"));
}

#[test]
fn discovers_configuration_in_a_parent_directory() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join(".ansible-lint"),
        "warn_list: [parser-error]\n",
    );
    write(&directory.path().join("work/bad.yml"), "a: [1,\n");
    let options = LintOptions {
        cwd: directory.path().join("work"),
        ..LintOptions::new(directory.path().join("work"))
    };
    let result = lint(&options).expect("lint");
    assert_eq!(result.findings.len(), 1);
    assert_eq!(
        result.findings[0].finding.severity,
        ansilint_core::Severity::Warning
    );
    assert!(!result.has_fatal(false));
}
