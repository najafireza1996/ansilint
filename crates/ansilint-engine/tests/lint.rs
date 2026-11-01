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
fn ignore_file_matches_nested_paths() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("sub/site.yml"), "- hosts: all\n");
    write(
        &directory.path().join(".ansible-lint-ignore"),
        "sub/site.yml warning skip\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
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
fn excludes_the_default_cache_directory() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    write(&directory.path().join(".cache/bad.yml"), "a: [1,\n");
    let result = lint(&options(directory.path())).expect("lint");
    assert_eq!(result.files_linted, 1);
}

#[test]
fn strips_a_bom_from_configuration_and_ignore_files() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    write(
        &directory.path().join(".ansible-lint"),
        "\u{feff}exclude_paths: []\n",
    );
    write(
        &directory.path().join(".ansible-lint-ignore"),
        "\u{feff}site.yml warning skip\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
}

#[test]
fn discovers_ignore_file_next_to_a_dot_config_directory() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    write(
        &directory.path().join(".config/ansible-lint.yml"),
        "exclude_paths: []\n",
    );
    write(
        &directory.path().join(".config/ansible-lint-ignore.txt"),
        "site.yml warning skip\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
}

#[test]
fn can_disable_default_rules() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    write(
        &directory.path().join(".ansible-lint"),
        "use_default_rules: false\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
}

#[test]
fn tag_selectors_that_match_no_rule_produce_a_notice() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    write(
        &directory.path().join(".ansible-lint"),
        "tags: [syntax, no-such-tag]\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert!(result.findings.is_empty());
    assert_eq!(
        result.warnings,
        ["tag selector `no-such-tag` matches no rule"]
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
fn configuration_keys_without_an_effect_yet_produce_a_notice() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    write(
        &directory.path().join(".ansible-lint"),
        "sarif_file: out.sarif\nstrict: false\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert_eq!(
        result.warnings,
        ["configuration key `sarif_file` is accepted but not implemented yet"]
    );
}

#[test]
fn a_target_directory_is_linted_like_any_other() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("target/site.yml"), "- hosts: all\n");
    let result = lint(&options(directory.path())).expect("lint");
    assert_eq!(result.files_linted, 1);
}

#[test]
fn crlf_configuration_and_ignore_files_are_understood() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    write(&directory.path().join("other.yml"), "- hosts: all\n");
    write(
        &directory.path().join(".ansible-lint"),
        "exclude_paths:\r\n  - other.yml\r\n",
    );
    write(
        &directory.path().join(".ansible-lint-ignore"),
        "site.yml warning skip\r\n",
    );
    let result = lint(&options(directory.path())).expect("lint");
    assert_eq!(result.files_linted, 1);
    assert!(result.findings.is_empty());
}

#[test]
fn configuration_files_go_through_the_shared_loader() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    fs::write(directory.path().join(".ansible-lint"), [0xFF, 0xFE, b'\n']).expect("write");
    let error = lint(&options(directory.path())).expect_err("invalid UTF-8 config");
    assert!(error.to_string().contains("not valid UTF-8"), "{error}");
    assert!(error.to_string().contains(".ansible-lint"), "{error}");

    fs::write(directory.path().join(".ansible-lint"), "skip_list: []\n").expect("write");
    let mut small = options(directory.path());
    small.max_file_bytes = 4;
    let error = lint(&small).expect_err("oversized config");
    assert!(error.to_string().contains("byte limit"), "{error}");
}

#[test]
fn relative_config_path_resolves_exclude_paths_from_the_config_location() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("lint.yml"),
        "exclude_paths: [work/skip]\n",
    );
    write(&directory.path().join("work/site.yml"), "- hosts: all\n");
    write(&directory.path().join("work/skip/bad.yml"), "a: [1,\n");
    let mut options = LintOptions::new(directory.path().join("work"));
    options.config_path = Some("../lint.yml".into());
    let result = lint(&options).expect("lint");
    assert_eq!(result.files_linted, 1);
    // No git repository: the project root is the cwd (`work/`).
    assert_eq!(
        result.findings[0].finding.path,
        std::path::PathBuf::from("site.yml")
    );
}

#[test]
fn files_outside_the_project_root_are_reported_relative_not_absolute() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("project/.git/HEAD"), "ref: x\n");
    write(&directory.path().join("outside/site.yml"), "- hosts: all\n");
    let mut options = LintOptions::new(directory.path().join("project"));
    options.paths = vec![directory.path().join("outside/site.yml")];
    let result = lint(&options).expect("lint");
    assert_eq!(
        result.findings[0].finding.path,
        std::path::PathBuf::from("../outside/site.yml")
    );
}

#[test]
fn finds_the_ignore_file_in_every_documented_layout() {
    // (config file or "", ignore file, cwd relative to the root)
    let layouts = [
        (".ansible-lint", ".ansible-lint-ignore", ""),
        (".ansible-lint.yml", ".ansible-lint-ignore", ""),
        (
            ".config/ansible-lint.yml",
            ".config/ansible-lint-ignore.txt",
            "",
        ),
        (".config/ansible-lint.yaml", ".ansible-lint-ignore", ""),
        ("", ".ansible-lint-ignore", ""),
        ("", ".config/ansible-lint-ignore.txt", ""),
        ("sub/.ansible-lint", "sub/.ansible-lint-ignore", "sub"),
    ];
    let found: Vec<bool> = layouts
        .iter()
        .map(|(config, ignore, cwd)| {
            let directory = tempfile::tempdir().expect("temp dir");
            let root = directory.path();
            write(&root.join(".git/HEAD"), "ref: x\n");
            if !config.is_empty() {
                write(&root.join(config), "skip_list: []\n");
            }
            write(&root.join(ignore), "x.yml warning\n");
            let result = lint(&LintOptions::new(root.join(cwd))).expect("lint");
            result.ignore_file_path == Some(root.join(ignore))
        })
        .collect();
    assert_eq!(found, [true; 7], "layouts: {layouts:?}");
}
