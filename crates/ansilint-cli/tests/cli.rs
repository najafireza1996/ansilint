//! Integration tests for the `ansilint` binary.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use std::fs;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
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
    write(
        &directory.path().join("site-playbook.yml"),
        "# nothing to do\n",
    );
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .arg("site-playbook.yml")
        .assert()
        .code(0)
        .stdout(contains("warning: empty playbook"));
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .args(["site-playbook.yml", "--strict"])
        .assert()
        .code(2);
}

#[test]
fn json_output_contains_tags() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site-playbook.yml"),
        "# nothing to do\n",
    );
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .args(["site-playbook.yml", "-f", "json"])
        .assert()
        .success()
        .stdout(contains("\"tag\":\"warning\""));
}

#[test]
fn noqa_silences_findings() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site-playbook.yml"),
        "# noqa: warning\n",
    );
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .arg("site-playbook.yml")
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
    write(
        &directory.path().join("site-playbook.yml"),
        "# nothing to do\n",
    );
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .args(["site-playbook.yml", "--generate-ignore"])
        .assert()
        .success();
    let contents =
        fs::read_to_string(directory.path().join(".ansible-lint-ignore")).expect("ignore file");
    assert!(contents.contains("site-playbook.yml warning"));
}

// Exit codes (H-E1), verified against upstream ansible-lint at commit
// 67a48717843c4d5d37681294cb5c9967781b0a31 (constants.RC, app.report_outcome).

#[test]
fn clean_files_exit_with_zero() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site-playbook.yml"),
        "- hosts: all\n  tasks:\n    - name: ping\n      ansible.builtin.ping:\n",
    );
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .assert()
        .code(0);
}

#[test]
fn usage_errors_exit_with_two_like_argparse() {
    Command::cargo_bin("ansilint")
        .expect("binary")
        .arg("--not-a-real-flag")
        .assert()
        .code(2);
    Command::cargo_bin("ansilint")
        .expect("binary")
        .args(["-f", "no-such-format"])
        .assert()
        .code(2);
}

#[test]
fn invalid_configuration_exits_with_three() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site-playbook.yml"),
        "# nothing to do\n",
    );
    write(&directory.path().join(".ansible-lint"), "strict: maybe\n");
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .assert()
        .code(3);
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .args(["-c", "missing.yml"])
        .assert()
        .code(3);
}

#[test]
fn no_files_matched_exits_with_five() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(&directory.path().join("README.md"), "# not yaml\n");
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .assert()
        .code(5);
}

// Unix only: Windows directory permissions cannot be revoked with the
// standard library. Exit 1 on every platform is covered by the `run` unit test
// `write_failure_maps_to_exit_code_one` (src/tests.rs).
#[cfg(unix)]
#[test]
fn unexpected_io_failure_exits_with_one() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().expect("temp dir");
    let locked = directory.path().join("locked");
    write(&locked.join("site-playbook.yml"), "# nothing to do\n");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("chmod");
    let output = Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .output()
        .expect("run");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("chmod back");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
}

// Unix only: sending Ctrl-C to a child on Windows needs the Win32
// `GenerateConsoleCtrlEvent` API (`unsafe` or a new dependency, both ruled
// out). Windows behavior is documented in docs/CLI.md.
#[cfg(unix)]
#[test]
fn interrupt_terminates_with_sigint_which_shells_report_as_130() {
    use std::os::unix::process::ExitStatusExt;
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("big.yml"),
        &"- {name: x, ansible.builtin.debug: {msg: y}}\n".repeat(100_000),
    );
    let binary = assert_cmd::cargo::cargo_bin("ansilint");
    let mut child = std::process::Command::new(binary)
        .current_dir(directory.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn");
    let killed = std::process::Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("kill");
    assert!(killed.success());
    let status = child.wait().expect("wait");
    assert_eq!(status.signal(), Some(2), "{status:?}");
}

#[test]
fn output_is_identical_across_thread_counts() {
    let directory = tempfile::tempdir().expect("temp dir");
    for index in 0..10 {
        write(
            &directory.path().join(format!("empty_{index}.yml")),
            "# nothing to do\n",
        );
        write(
            &directory.path().join(format!("bad_{index}.yml")),
            "a: [1,\n",
        );
    }
    let run = |threads: &str| {
        Command::cargo_bin("ansilint")
            .expect("binary")
            .current_dir(directory.path())
            .env("RAYON_NUM_THREADS", threads)
            .args(["-f", "json"])
            .output()
            .expect("run")
    };
    let one = run("1");
    let four = run("4");
    assert_eq!(one.stdout, four.stdout);
    assert_eq!(one.status.code(), four.status.code());
}

#[test]
fn quiet_configuration_suppresses_warnings() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site-playbook.yml"),
        "# nothing to do\n",
    );
    write(&directory.path().join(".ansible-lint"), "quiet: true\n");
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .arg("site-playbook.yml")
        .assert()
        .success()
        .stderr("");
}

#[test]
fn tags_flag_keeps_only_matching_rules() {
    let directory = tempfile::tempdir().expect("temp dir");
    write(
        &directory.path().join("site-playbook.yml"),
        "# nothing to do\n",
    );
    write(&directory.path().join("bad.yml"), "a: [1,\n");
    Command::cargo_bin("ansilint")
        .expect("binary")
        .current_dir(directory.path())
        .args(["-t", "syntax", "site-playbook.yml", "bad.yml"])
        .assert()
        .code(2)
        .stdout(contains("parser-error"))
        .stdout(contains("empty playbook").not());
}
