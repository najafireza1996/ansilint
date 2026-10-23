//! Dependency-edge and pure-crate checks (ENG-01, ENG-02).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use super::files::{display_path, walk_files};
use super::Architecture;

const PURE_IO_PATTERNS: &[&str] = &[
    "std::fs::",
    "std::env::",
    "std::process::",
    "std::net::",
    "SystemTime::now",
];

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>,
}

fn load_metadata(root: &Path) -> Result<Metadata> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .current_dir(root)
        .output()
        .context("failed to run `cargo metadata`")?;
    if !output.status.success() {
        bail!(
            "`cargo metadata` failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).context("failed to parse `cargo metadata` output")
}

pub(super) fn check_dependency_edges(
    root: &Path,
    architecture: &Architecture,
) -> Result<Vec<String>> {
    let metadata = load_metadata(root)?;
    let members: BTreeSet<&str> = metadata
        .workspace_members
        .iter()
        .map(String::as_str)
        .collect();
    let packages: Vec<&Package> = metadata
        .packages
        .iter()
        .filter(|package| members.contains(package.id.as_str()))
        .collect();
    let names: BTreeSet<&str> = packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();

    let mut failures = Vec::new();
    for package in packages {
        if is_exempt_crate(&package.name) {
            continue;
        }
        let Some(allowed) = architecture.edges.get(&package.name) else {
            failures.push(format!(
                "crate `{}` is not declared in [edges] of architecture.toml",
                package.name
            ));
            continue;
        };
        for dependency in &package.dependencies {
            if !names.contains(dependency.name.as_str()) {
                continue;
            }
            if dependency.name == "ansilint-testing" {
                if dependency.kind.as_deref() != Some("dev") {
                    failures.push(format!(
                        "crate `{}` uses ansilint-testing outside [dev-dependencies]",
                        package.name
                    ));
                }
                continue;
            }
            if !allowed.contains(&dependency.name) {
                failures.push(format!(
                    "crate `{}` depends on `{}`, which is not an allowed edge in architecture.toml",
                    package.name, dependency.name
                ));
            }
        }
    }
    failures.extend(check_cycles(&architecture.edges));
    Ok(failures)
}

fn is_exempt_crate(name: &str) -> bool {
    matches!(name, "xtask" | "ansilint-testing")
}

fn check_cycles(edges: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut nodes: BTreeSet<&str> = edges.keys().map(String::as_str).collect();
    for dependencies in edges.values() {
        nodes.extend(dependencies.iter().map(String::as_str));
    }
    let mut indegree: BTreeMap<&str, usize> = nodes.iter().map(|node| (*node, 0)).collect();
    for dependencies in edges.values() {
        for dependency in dependencies {
            if let Some(count) = indegree.get_mut(dependency.as_str()) {
                *count += 1;
            }
        }
    }
    let mut queue: Vec<&str> = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(node, _)| *node)
        .collect();
    let mut visited = 0;
    while let Some(node) = queue.pop() {
        visited += 1;
        if let Some(dependencies) = edges.get(node) {
            for dependency in dependencies {
                if let Some(count) = indegree.get_mut(dependency.as_str()) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push(dependency.as_str());
                    }
                }
            }
        }
    }
    if visited == nodes.len() {
        Vec::new()
    } else {
        vec!["architecture.toml declares a dependency cycle".to_string()]
    }
}

pub(super) fn check_pure_crates(root: &Path, pure_crates: &[String]) -> Result<Vec<String>> {
    let mut failures = Vec::new();
    for name in pure_crates {
        let crate_dir = root.join("crates").join(name);
        if !crate_dir.is_dir() {
            failures.push(format!("pure crate `{name}` does not exist under crates/"));
            continue;
        }
        check_pure_crate_config(root, name, &crate_dir, &mut failures);
        for path in walk_files(&crate_dir, "rs")? {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            for pattern in PURE_IO_PATTERNS {
                if text.contains(pattern) {
                    failures.push(format!(
                        "{}: pure crate `{name}` uses `{pattern}`; I/O belongs to the engine",
                        display_path(root, &path)
                    ));
                }
            }
        }
    }
    Ok(failures)
}

fn check_pure_crate_config(root: &Path, name: &str, crate_dir: &Path, failures: &mut Vec<String>) {
    let config = crate_dir.join("clippy.toml");
    let path = display_path(root, &config);
    match fs::read_to_string(&config) {
        Ok(text) => {
            for needle in ["disallowed-methods", "disallowed-types"] {
                if !text.contains(needle) {
                    failures.push(format!(
                        "{path}: `{needle}` is missing from the pure crate's clippy.toml"
                    ));
                }
            }
        }
        Err(error) => failures.push(format!(
            "{path}: pure crate `{name}` needs a clippy.toml: {error}"
        )),
    }
}
