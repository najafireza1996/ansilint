//! Integration tests for the `ansilint` binary.

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn version_flag_prints_the_package_version() {
    Command::cargo_bin("ansilint")
        .expect("the ansilint binary is built for integration tests")
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}
