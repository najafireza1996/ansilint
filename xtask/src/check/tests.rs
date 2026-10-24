//! Deliberate-violation tests for the workspace checks.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::dependencies::{check_dependency_edges, check_pure_crates};
use super::hygiene::{check_forbidden_names, check_markers, check_panic_strategy};
use super::load_architecture;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, contents).expect("write fixture");
}

fn workspace() -> TempDir {
    let temp = tempfile::tempdir().expect("create temp workspace");
    write(
        &temp.path().join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/*\"]\n",
    );
    write(&temp.path().join("docs/TECH_DEBT.md"), "# Technical debt\n");
    temp
}

fn crate_manifest(name: &str, dependency: Option<&str>) -> String {
    let mut manifest =
        format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n");
    if let Some(dependency) = dependency {
        let _ = write!(
            manifest,
            "\n[dependencies]\n{dependency} = {{ path = \"../{dependency}\" }}\n"
        );
    }
    manifest
}

#[test]
fn rejects_a_forbidden_dependency_edge() {
    let temp = workspace();
    let root = temp.path();
    write(
        &root.join("crates/ansilint-core/Cargo.toml"),
        &crate_manifest("ansilint-core", Some("ansilint-yaml")),
    );
    write(&root.join("crates/ansilint-core/src/lib.rs"), "//! core\n");
    write(
        &root.join("crates/ansilint-yaml/Cargo.toml"),
        &crate_manifest("ansilint-yaml", None),
    );
    write(&root.join("crates/ansilint-yaml/src/lib.rs"), "//! yaml\n");
    write(
        &root.join("architecture.toml"),
        "[edges]\nansilint-core = []\nansilint-yaml = [\"ansilint-core\"]\n\n[pure_crates]\nlist = []\n",
    );

    let failures =
        check_dependency_edges(root, &load_architecture(root).expect("load architecture"))
            .expect("run edge check");
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("not an allowed edge")),
        "expected forbidden-edge failure, got {failures:?}"
    );
}

#[test]
fn rejects_a_utils_module() {
    let temp = workspace();
    let root = temp.path();
    write(
        &root.join("crates/ansilint-core/src/utils.rs"),
        "//! helpers\n",
    );
    let failures = check_forbidden_names(root).expect("run name check");
    assert!(
        failures.iter().any(|failure| failure.contains("utils")),
        "expected forbidden-name failure, got {failures:?}"
    );
}

#[test]
fn rejects_an_untracked_pending_marker() {
    let temp = workspace();
    let root = temp.path();
    let pending = concat!("TO", "DO");
    let source = format!("//! core\n// {pending}: later\n");
    write(&root.join("crates/ansilint-core/src/lib.rs"), &source);
    let failures = check_markers(root).expect("run marker check");
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("must be written as")),
        "expected marker failure, got {failures:?}"
    );
}

#[test]
fn rejects_io_inside_a_pure_crate() {
    let temp = workspace();
    let root = temp.path();
    write(
        &root.join("crates/ansilint-core/clippy.toml"),
        "disallowed-methods = []\ndisallowed-types = []\n",
    );
    write(
        &root.join("crates/ansilint-core/src/lib.rs"),
        "//! core\nfn read() { let _ = std::fs::read(\"x\"); }\n",
    );
    let failures =
        check_pure_crates(root, &["ansilint-core".to_string()]).expect("run pure-crate check");
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("uses `std::fs::`")),
        "expected I/O failure, got {failures:?}"
    );
}

#[test]
fn rejects_panic_abort_profiles() {
    let temp = workspace();
    let root = temp.path();
    write(
        &root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = []\n\n[profile.release]\npanic = \"abort\"\n",
    );
    let failures = check_panic_strategy(root).expect("run panic check");
    assert!(
        failures.iter().any(|failure| failure.contains("panic")),
        "expected panic=abort failure, got {failures:?}"
    );
}
