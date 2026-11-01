//! H-B4: a panicking rule becomes an `internal-error` finding and the run
//! continues. CI runs this against the release profile:
//! `cargo test --release -p ansilint-cli --features test-panic-rule --test panic_isolation`.

#![cfg(feature = "test-panic-rule")]
#![allow(
    clippy::expect_used,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use std::fs;

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn a_panicking_rule_is_reported_and_the_run_continues() {
    let directory = tempfile::tempdir().expect("temp dir");
    fs::write(
        directory.path().join("boom.yml"),
        "# ansilint-test-panic\nkey: value\n",
    )
    .expect("write boom.yml");
    fs::write(directory.path().join("site.yml"), "- hosts: all\n").expect("write site.yml");
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .args(["boom.yml", "site.yml"])
        .assert()
        .code(2)
        .stdout(contains(
            "internal-error: rule `test-panic-probe` panicked: panic probe triggered",
        ))
        .stdout(contains("boom.yml:"))
        .stdout(contains("site.yml:1: warning: empty playbook"));
}
