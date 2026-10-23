//! Conventional Commit validation.

use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::workspace::root;

const CONVENTIONAL_TYPES: &[&str] = &[
    "build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style", "test",
];

pub(crate) fn check_commits(args: &[String]) -> Result<()> {
    let range = parse_commit_range(args)?;
    let mut command = Command::new("git");
    command.args(["log", "--format=%s"]);
    if let Some(range) = &range {
        command.arg(range);
    } else {
        command.arg("-1");
    }
    let output = command
        .current_dir(root())
        .output()
        .context("failed to run `git log`")?;
    if !output.status.success() {
        bail!(
            "`git log` failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let subjects = String::from_utf8_lossy(&output.stdout);
    let mut failures = Vec::new();
    for subject in subjects.lines().filter(|line| !line.trim().is_empty()) {
        if let Err(error) = validate_commit_subject(subject) {
            failures.push(format!("{subject}: {error}"));
        }
    }
    if failures.is_empty() {
        println!("commit messages: ok");
        return Ok(());
    }
    for failure in &failures {
        eprintln!("commit message: {failure}");
    }
    bail!(
        "{} commit message(s) do not follow Conventional Commits",
        failures.len()
    )
}

fn parse_commit_range(args: &[String]) -> Result<Option<String>> {
    match args {
        [] => Ok(None),
        [flag, range] if flag.as_str() == "--range" => Ok(Some(range.clone())),
        _ => bail!("usage: cargo xtask check-commits [--range <rev-range>]"),
    }
}

fn validate_commit_subject(subject: &str) -> Result<(), String> {
    let Some((header, description)) = subject.split_once(':') else {
        return Err("missing `type: description`".to_string());
    };
    if description.trim().is_empty() {
        return Err("empty description".to_string());
    }
    let kind = header.split(['(', '!']).next().unwrap_or_default();
    if !CONVENTIONAL_TYPES.contains(&kind) {
        return Err(format!("unknown type `{kind}`"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_commit_range, validate_commit_subject};

    #[test]
    fn accepts_conventional_subjects() {
        assert!(validate_commit_subject("feat(parser): add spans").is_ok());
        assert!(validate_commit_subject("fix!: drop bad behavior").is_ok());
    }

    #[test]
    fn rejects_unconventional_subjects() {
        assert!(validate_commit_subject("Add spans").is_err());
        assert!(validate_commit_subject("feat: ").is_err());
        assert!(validate_commit_subject("nope: do something").is_err());
    }

    #[test]
    fn parses_an_optional_range() {
        assert_eq!(parse_commit_range(&[]).expect("no arguments"), None);
        let arguments = vec!["--range".to_string(), "a..b".to_string()];
        assert_eq!(
            parse_commit_range(&arguments).expect("valid arguments"),
            Some("a..b".to_string())
        );
    }

    #[test]
    fn rejects_bad_range_arguments() {
        let arguments = vec!["--range".to_string()];
        assert!(parse_commit_range(&arguments).is_err());
    }
}
