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

/// Every rule fixture input must have a checked-in expected-findings file
/// (H-C9), so a fixture can never sit unexecuted.
pub(super) fn check_fixture_expectations(root: &Path) -> Result<Vec<String>> {
    let fixtures = root.join("tests/fixtures");
    if !fixtures.is_dir() {
        return Ok(Vec::new());
    }
    let mut failures = Vec::new();
    for rule_dir in read_entries(&fixtures)?
        .into_iter()
        .map(|entry| entry.path())
    {
        let rule = rule_dir
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default();
        if !rule_dir.is_dir() || rule.starts_with('_') {
            continue;
        }
        for kind in ["fail", "pass", "noqa"] {
            let kind_dir = rule_dir.join(kind);
            if !kind_dir.is_dir() {
                continue;
            }
            for input in read_entries(&kind_dir)?
                .into_iter()
                .map(|entry| entry.path())
            {
                let stem = input
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .unwrap_or_default();
                let expected = format!("expected/{kind}-{stem}.json");
                if !rule_dir.join(&expected).is_file() {
                    failures.push(format!(
                        "tests/fixtures/{rule}/{kind}/{} has no {expected}",
                        input
                            .file_name()
                            .and_then(OsStr::to_str)
                            .unwrap_or_default()
                    ));
                }
            }
        }
    }
    Ok(failures)
}

/// Every relative link in `docs/rules/README.md` must point at an existing
/// file (H-C16); generated links once used Rust file stems instead of ids.
pub(super) fn check_rule_doc_links(root: &Path) -> Result<Vec<String>> {
    let index = root.join("docs/rules/README.md");
    if !index.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&index)
        .with_context(|| format!("failed to read {}", index.display()))?;
    let mut failures = Vec::new();
    for (number, line) in text.lines().enumerate() {
        for target in line
            .split("](")
            .skip(1)
            .filter_map(|rest| rest.split(')').next())
        {
            if target.contains("://") || target.starts_with('#') {
                continue;
            }
            let path = target.split('#').next().unwrap_or(target);
            if !root.join("docs/rules").join(path).is_file() {
                failures.push(format!(
                    "docs/rules/README.md:{}: link target `{target}` does not exist",
                    number + 1
                ));
            }
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
