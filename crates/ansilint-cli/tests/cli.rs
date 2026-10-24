//! Integration tests for the `ansilint` binary.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use std::fs;

use assert_cmd::Command;
use predicates::str::contains;

fn write(path: &std::path::Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write fixture");
}

#[test]
fn version_flag_prints_the_package_version() {
    Command::cargo_bin("ansilint")
        .expect("the ansilint binary is built for integration tests")
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn parser_errors_exit_with_two() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("bad.yml"), "a: [1,\n");
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .arg("bad.yml")
        .assert()
        .code(2)
        .stdout(contains("parser-error"));
}

#[test]
fn warnings_pass_unless_strict() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .arg("site.yml")
        .assert()
        .code(0)
        .stdout(contains("warning: empty playbook"));
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .args(["site.yml", "--strict"])
        .assert()
        .code(2);
}

#[test]
fn json_output_contains_tags() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .args(["site.yml", "-f", "json"])
        .assert()
        .success()
        .stdout(contains("\"tag\":\"warning\""));
}

#[test]
fn noqa_silences_findings() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site.yml"),
        "- hosts: all  # noqa: warning\n",
    );
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .arg("site.yml")
        .assert()
        .success()
        .stdout("");
}

#[test]
fn lists_rules_and_tags() {
    Command::cargo_bin("ansilint")
        .expect("binary")
        .arg("--list-rules")
        .assert()
        .success()
        .stdout(contains("warning"));
    Command::cargo_bin("ansilint")
        .expect("binary")
        .arg("--list-tags")
        .assert()
        .success()
        .stdout(contains("syntax"));
}

#[test]
fn generates_an_ignore_file() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("site.yml"), "- hosts: all\n");
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .args(["site.yml", "--generate-ignore"])
        .assert()
        .success();
    let contents =
        fs::read_to_string(directory.path().join(".ansible-lint-ignore")).expect("ignore file");
    assert!(contents.contains("site.yml warning"));
}
