//! End-to-end tests for the lint pipeline.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use ansilint_config::ConfigOverrides;
use ansilint_engine::{lint, LintOptions};
use ansilint_output::Format;

use ansilint_testing::write_file as write;

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
fn handles_uppercase_extensions_by_sniffing() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("SITE.YML"), "- hosts: all\n");
    let result = lint(&options(directory.path())).expect("lint");
    assert_eq!(result.files_linted, 1);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].finding.tag.rule, "warning");
}

#[test]
fn does_not_treat_generic_yaml_lists_as_tasks() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("data.yml"), "- foo: 1\n  bar: 2\n");
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
}

#[test]
fn a_playbook_of_imports_is_not_empty() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site.yml"),
        "- import_playbook: web.yml\n- ansible.builtin.import_playbook: db.yml\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
}

#[test]
fn model_issues_are_not_surfaced_as_warnings() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("roles/x/tasks/main.yml"),
        "- name: bad\n  ansible.builtin.copy: {src: a}\n  ansible.builtin.file: {path: b}\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
}

#[test]
fn model_issues_in_playbooks_and_handlers_never_become_findings() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site.yml"),
        "- hosts: all\n  tasks: not-a-list\n  roles: also-not-a-list\n  handlers:\n    - 42\n",
    );
    write(
        &directory.path().join("roles/x/handlers/main.yml"),
        "- just a string\n- name: no action here\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    let tags: Vec<String> = result
        .findings
        .iter()
        .map(|positioned| positioned.finding.tag.to_string())
        .collect();
    assert_eq!(
        tags,
        ["warning"],
        "only the empty-playbook warning may fire"
    );
}

#[test]
fn hundred_thousand_deep_nesting_is_a_parser_error_not_a_crash() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("deep.yml"),
        &format!("{}x\n", "- ".repeat(100_000)),
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].finding.tag.rule, "parser-error");
    assert!(result.findings[0].finding.message.contains("nesting depth"));
}

#[test]
fn noqa_inside_a_scalar_does_not_suppress() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site.yml"),
        "- {hosts: \"a # noqa: warning, x\"}\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert_eq!(result.findings.len(), 1, "the quoted text is not a comment");
}
