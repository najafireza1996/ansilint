//! Workspace fitness functions from Section 12 of the specification.
//!
//! Each check is a pure function of the workspace tree (plus `cargo metadata`)
//! that returns human-readable failures. The purpose is to make architecture
//! and hygiene rules fail loudly instead of living only in review comments.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

mod dependencies;
mod files;
mod hygiene;
mod rules;

#[cfg(test)]
mod tests;

use dependencies::{check_dependency_edges, check_pure_crates};
use hygiene::{
    check_adrs, check_crate_docs, check_file_sizes, check_forbidden_names, check_markers,
    check_panic_strategy,
};
use rules::{check_rule_id_matches, check_rules_consistency};

/// Collected results of all workspace checks.
#[derive(Debug, Default)]
pub(crate) struct Report {
    /// Problems that make `cargo xtask check` fail.
    pub(crate) failures: Vec<String>,
    /// Advisory findings that do not fail the build.
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Architecture {
    edges: BTreeMap<String, Vec<String>>,
    pure_crates: PureCrates,
}

#[derive(Debug, Deserialize)]
struct PureCrates {
    list: Vec<String>,
}

/// Runs every workspace check and returns the report.
pub(crate) fn run_checks(root: &Path) -> Result<Report> {
    let architecture = load_architecture(root)?;
    let mut report = Report::default();

    report
        .failures
        .extend(check_dependency_edges(root, &architecture)?);
    report
        .failures
        .extend(check_pure_crates(root, &architecture.pure_crates.list)?);
    report.failures.extend(check_forbidden_names(root)?);

    let (warnings, failures) = check_file_sizes(root)?;
    report.warnings.extend(warnings);
    report.failures.extend(failures);

    report.failures.extend(check_markers(root)?);
    report.failures.extend(check_panic_strategy(root)?);
    report.failures.extend(check_rules_consistency(root)?);
    report.failures.extend(check_rule_id_matches(root)?);
    report.failures.extend(check_adrs(root)?);
    report.failures.extend(check_crate_docs(root)?);

    Ok(report)
}

/// Reads `architecture.toml` from the workspace root.
pub(crate) fn load_architecture(root: &Path) -> Result<Architecture> {
    let path = root.join("architecture.toml");
    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}
