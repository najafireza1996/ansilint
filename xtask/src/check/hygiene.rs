//! Hygiene checks: forbidden names, file sizes, markers, ADRs and crate docs.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::files::{display_path, read_entries, walk_files};

const FILE_SIZE_WARN: usize = 400;
const FILE_SIZE_FAIL: usize = 800;
const FORBIDDEN_NAMES: &[&str] = &[
    "utils", "util", "helpers", "common", "misc", "base", "manager", "stuff",
];

const PENDING: &str = concat!("TO", "DO");
const BROKEN_NOTE: &str = concat!("FIX", "ME");
const TRIPLE_X: &str = concat!("X", "XX");
const SMELL: &str = concat!("HA", "CK");
const IGNORE_ATTR: &str = concat!("#[ign", "ore");
const GENERATED: &str = concat!("@", "generated");

/// Fails when a profile disables unwinding, which would break `internal-error`.
pub(super) fn check_panic_strategy(root: &Path) -> Result<Vec<String>> {
    let mut failures = Vec::new();
    for path in walk_files(root, "toml")? {
        if path.file_name().is_none_or(|name| name != "Cargo.toml") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        for (index, line) in text.lines().enumerate() {
            let compact: String = line
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect();
            if compact.starts_with("panic=\"abort\"") {
                failures.push(format!(
                    "{}:{}: `panic = \"abort\"` disables catch_unwind and internal-error",
                    display_path(root, &path),
                    index + 1
                ));
            }
        }
    }
    Ok(failures)
}

pub(super) fn check_forbidden_names(root: &Path) -> Result<Vec<String>> {
    let mut failures = Vec::new();
    for base in ["crates", "xtask", "benches", "fuzz"] {
        let directory = root.join(base);
        if directory.is_dir() {
            scan_forbidden_names(root, &directory, &mut failures)?;
        }
    }
    Ok(failures)
}

fn scan_forbidden_names(root: &Path, directory: &Path, failures: &mut Vec<String>) -> Result<()> {
    for entry in read_entries(directory)? {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if super::files::should_skip_dir(&path) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if FORBIDDEN_NAMES.contains(&name.as_str()) {
                failures.push(format!(
                    "{}: forbidden dumping-ground directory name `{name}`",
                    display_path(root, &path)
                ));
            }
            scan_forbidden_names(root, &path, failures)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or_default();
            if FORBIDDEN_NAMES.contains(&stem) {
                failures.push(format!(
                    "{}: forbidden dumping-ground module name `{stem}`",
                    display_path(root, &path)
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn check_file_sizes(root: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let mut warnings = Vec::new();
    let mut failures = Vec::new();
    for base in ["crates", "xtask"] {
        for path in walk_files(&root.join(base), "rs")? {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            if text.contains(GENERATED) {
                continue;
            }
            let lines = text.lines().count();
            let display = display_path(root, &path);
            if lines > FILE_SIZE_FAIL {
                failures.push(format!(
                    "{display}: {lines} lines exceeds the {FILE_SIZE_FAIL}-line hard limit"
                ));
            } else if lines > FILE_SIZE_WARN {
                warnings.push(format!(
                    "{display}: {lines} lines exceeds the {FILE_SIZE_WARN}-line soft limit"
                ));
            }
        }
    }
    Ok((warnings, failures))
}

pub(super) fn check_markers(root: &Path) -> Result<Vec<String>> {
    let mut failures = Vec::new();
    let tech_debt_path = root.join("docs/TECH_DEBT.md");
    let tech_debt = fs::read_to_string(&tech_debt_path).unwrap_or_default();

    for base in ["crates", "xtask"] {
        for path in walk_files(&root.join(base), "rs")? {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let display = display_path(root, &path);
            for (number, line) in text.lines().enumerate() {
                let location = format!("{display}:{}", number + 1);
                if line.contains(BROKEN_NOTE) || line.contains(TRIPLE_X) || line.contains(SMELL) {
                    failures.push(format!(
                        "{location}: banned marker; use {PENDING}(#<issue>) and document it"
                    ));
                }
                check_pending_markers(line, &location, &tech_debt, &mut failures);
                if line.contains(IGNORE_ATTR) && !has_issue_reference(line) {
                    failures.push(format!("{location}: ignored test needs an issue reference"));
                }
            }
        }
    }
    Ok(failures)
}

fn check_pending_markers(line: &str, location: &str, tech_debt: &str, failures: &mut Vec<String>) {
    let mut remainder = line;
    while let Some(position) = remainder.find(PENDING) {
        let after = remainder
            .get(position + PENDING.len()..)
            .unwrap_or_default();
        let Some(rest) = after.strip_prefix("(#") else {
            failures.push(format!(
                "{location}: {PENDING} must be written as {PENDING}(#<issue>)"
            ));
            remainder = after;
            continue;
        };
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            failures.push(format!(
                "{location}: {PENDING}(#...) needs a numeric issue number"
            ));
        } else if !tech_debt.contains(&format!("#{digits}")) {
            failures.push(format!(
                "{location}: {PENDING}(#{digits}) has no entry in docs/TECH_DEBT.md"
            ));
        }
        remainder = rest;
    }
}

fn has_issue_reference(line: &str) -> bool {
    line.contains("http")
        || line.char_indices().any(|(index, character)| {
            character == '#'
                && line.get(index + 1..).is_some_and(|rest| {
                    rest.chars()
                        .next()
                        .is_some_and(|next| next.is_ascii_digit())
                })
        })
}

pub(super) fn check_adrs(root: &Path) -> Result<Vec<String>> {
    let adr_dir = root.join("docs/adr");
    if !adr_dir.is_dir() {
        return Ok(vec!["docs/adr/ does not exist".to_string()]);
    }
    let mut failures = Vec::new();
    let template = adr_dir.join("0000-template.md");
    if !template.is_file() {
        failures.push("docs/adr/0000-template.md is missing".to_string());
    } else if let Ok(text) = fs::read_to_string(&template) {
        failures.extend(missing_adr_headings(&template, &text));
    }

    let mut numbers = Vec::new();
    for entry in read_entries(&adr_dir)? {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "md")
            && path
                .file_name()
                .is_some_and(|name| name != "0000-template.md")
        {
            match adr_number(&path) {
                Some(number) => numbers.push(number),
                None => failures.push(format!(
                    "{}: ADR file name must start with a four-digit number",
                    display_path(root, &path)
                )),
            }
            if let Ok(text) = fs::read_to_string(&path) {
                failures.extend(missing_adr_headings(&path, &text));
            }
        }
    }
    numbers.sort_unstable();
    for (expected, actual) in (1_u32..).zip(numbers.iter().copied()) {
        if expected != actual {
            failures.push(format!(
                "docs/adr: expected ADR {expected:04} but found {actual:04}"
            ));
            break;
        }
    }
    Ok(failures)
}

fn missing_adr_headings(path: &Path, text: &str) -> Vec<String> {
    [
        "## Context",
        "## Decision",
        "## Consequences",
        "## Alternatives considered",
    ]
    .into_iter()
    .filter(|heading| !text.contains(heading))
    .map(|heading| format!("{}: ADR is missing `{heading}`", path.display()))
    .collect()
}

fn adr_number(path: &Path) -> Option<u32> {
    let stem = path.file_stem().and_then(OsStr::to_str)?;
    let prefix = stem.split('-').next()?;
    if prefix.len() != 4 {
        return None;
    }
    prefix.parse().ok()
}

pub(super) fn check_crate_docs(root: &Path) -> Result<Vec<String>> {
    let mut failures = Vec::new();
    let mut crate_dirs = Vec::new();
    for entry in read_entries(&root.join("crates"))? {
        if entry.file_type()?.is_dir() {
            crate_dirs.push(entry.path());
        }
    }
    crate_dirs.push(root.join("xtask"));
    crate_dirs.sort();

    for directory in crate_dirs {
        match crate_entry_point(&directory) {
            Some(path) => {
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                let documented = text
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .is_some_and(|line| line.trim_start().starts_with("//!"));
                if !documented {
                    failures.push(format!(
                        "{}: crate-level `//!` documentation is missing",
                        display_path(root, &path)
                    ));
                }
            }
            None => failures.push(format!(
                "{}: crate has no src/lib.rs or src/main.rs",
                display_path(root, &directory)
            )),
        }
    }
    Ok(failures)
}

fn crate_entry_point(directory: &Path) -> Option<PathBuf> {
    let library = directory.join("src/lib.rs");
    if library.is_file() {
        return Some(library);
    }
    let binary = directory.join("src/main.rs");
    binary.is_file().then_some(binary)
}
