//! Rule registry, parity, documentation and fixture consistency checks.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::files::{display_path, read_entries, walk_files};

pub(super) fn check_rules_consistency(root: &Path) -> Result<Vec<String>> {
    let rules_dir = root.join("crates/ansilint-rules/src/rules");
    let rule_ids = rule_ids(&rules_dir)?;
    let parity_path = root.join("parity.toml");
    let parity_text = fs::read_to_string(&parity_path)
        .with_context(|| format!("failed to read {}", parity_path.display()))?;
    let parity: toml::Value = toml::from_str(&parity_text)
        .with_context(|| format!("failed to parse {}", parity_path.display()))?;
    let parity_ids: BTreeSet<String> = parity
        .get("rules")
        .and_then(toml::Value::as_table)
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default();

    let mut failures = Vec::new();
    for id in &rule_ids {
        if !parity_ids.contains(id) {
            failures.push(format!(
                "rule `{id}` is missing from [rules] in parity.toml"
            ));
        }
    }
    for id in &parity_ids {
        if !rule_ids.contains(id) {
            failures.push(format!(
                "parity.toml declares rule `{id}` but no rule file exists"
            ));
        }
    }
    for id in &rule_ids {
        failures.extend(check_rule_assets(root, id)?);
    }
    Ok(failures)
}

fn check_rule_assets(root: &Path, id: &str) -> Result<Vec<String>> {
    let mut failures = Vec::new();
    let doc = root.join("docs/rules").join(format!("{id}.md"));
    if !doc.is_file() {
        failures.push(format!("rule `{id}` has no docs/rules/{id}.md"));
    }
    for kind in ["fail", "pass", "noqa"] {
        let directory = root.join("tests/fixtures").join(id).join(kind);
        if !has_yaml(&directory)? {
            failures.push(format!(
                "rule `{id}` needs at least one fixture in tests/fixtures/{id}/{kind}/"
            ));
        }
    }
    Ok(failures)
}

pub(super) fn check_rule_id_matches(root: &Path) -> Result<Vec<String>> {
    let rules_dir = root.join("crates/ansilint-rules/src/rules");
    let mut failures = Vec::new();
    for path in walk_files(&rules_dir, "rs")? {
        if path.file_stem().is_some_and(|stem| stem == "mod") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        for (number, line) in text.lines().enumerate() {
            if line.contains("match") && line.contains(".id()") {
                failures.push(format!(
                    "{}:{}: dispatch on a rule id outside the registry",
                    display_path(root, &path),
                    number + 1
                ));
            }
        }
    }
    Ok(failures)
}

fn rule_ids(rules_dir: &Path) -> Result<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for path in walk_files(rules_dir, "rs")? {
        let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or_default();
        if stem != "mod" {
            ids.insert(stem.replace('_', "-"));
        }
    }
    Ok(ids)
}

fn has_yaml(directory: &Path) -> Result<bool> {
    if !directory.is_dir() {
        return Ok(false);
    }
    for entry in read_entries(directory)? {
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "yml" || extension == "yaml")
        {
            return Ok(true);
        }
    }
    Ok(false)
}
