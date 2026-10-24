//! Implementation of the `cargo xtask` subcommands.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::check;
use crate::commits;
use crate::generate;
use crate::workspace;

/// Dispatches one xtask invocation.
pub(crate) fn run(args: &[String]) -> Result<()> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };
    match command {
        "check" => run_check(),
        "ci" => run_ci(),
        "setup" => setup(),
        "gen-parity" => generate::gen_parity(),
        "gen-rule-docs" => generate::gen_rule_docs(),
        "update-schemas" => generate::update_schemas(),
        "build-index" => generate::build_index(),
        "check-commits" => commits::check_commits(args.get(1..).unwrap_or_default()),
        "bench" => bench(),
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        unknown => bail!("unknown xtask command `{unknown}`; run `cargo xtask help`"),
    }
}

fn print_help() {
    println!("cargo xtask <command>");
    println!();
    println!("Commands:");
    println!("  check          run the workspace fitness functions");
    println!("  ci             run everything CI runs (single source of truth)");
    println!("  setup          verify the toolchain and install optional git hooks");
    println!("  gen-parity     regenerate docs/PARITY.md from parity.toml");
    println!("  gen-rule-docs  regenerate docs/rules/README.md");
    println!("  update-schemas refresh vendored JSON schemas (Phase 6)");
    println!("  build-index    build the embedded module index (Phase 5)");
    println!("  check-commits  validate Conventional Commits [--range <rev-range>]");
    println!("  bench          lint a synthetic 500-file repository and print the best time");
}

fn run_check() -> Result<()> {
    let report = check::run_checks(&workspace::root())?;
    for warning in &report.warnings {
        println!("xtask check: warning: {warning}");
    }
    if report.failures.is_empty() {
        println!("xtask check: ok");
        return Ok(());
    }
    for failure in &report.failures {
        eprintln!("xtask check: error: {failure}");
    }
    bail!("{} workspace check(s) failed", report.failures.len())
}

fn bench() -> Result<()> {
    let directory = tempfile::tempdir().context("create benchmark directory")?;
    let playbook = "- hosts: all\n  tasks:\n    - name: hello\n      ansible.builtin.debug:\n        msg: hello\n";
    for index in 0..500 {
        let path = directory.path().join(format!("play_{index}.yml"));
        fs::write(&path, playbook)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    let mut options = ansilint_engine::LintOptions::new(directory.path());
    options.paths = vec![directory.path().to_path_buf()];
    let mut best = Duration::MAX;
    for run in 0..6 {
        let start = Instant::now();
        let result = ansilint_engine::lint(&options)?;
        let elapsed = start.elapsed();
        if run == 0 {
            println!("files linted: {}", result.files_linted);
        } else {
            best = best.min(elapsed);
        }
    }
    println!("500-file lint: best of 5 = {} ms", best.as_millis());
    Ok(())
}

fn run_ci() -> Result<()> {
    run_check()?;
    run_cargo(&["fmt", "--all", "--", "--check"])?;
    run_cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])?;
    run_cargo(&["test", "--workspace", "--all-features"])?;
    run_cargo(&[
        "doc",
        "--workspace",
        "--no-deps",
        "--document-private-items",
    ])?;
    run_tool(&["deny", "check"])?;
    run_tool(&["machete"])?;
    // xtask is developer tooling and never shipped, so it is excluded from the gate.
    run_coverage()?;
    println!("xtask ci: ok");
    Ok(())
}

fn run_cargo(args: &[&str]) -> Result<()> {
    let status = cargo_command(args)
        .status()
        .with_context(|| format!("failed to run `cargo {}`", args.join(" ")))?;
    if !status.success() {
        bail!("`cargo {}` failed", args.join(" "));
    }
    Ok(())
}

fn run_coverage() -> Result<()> {
    let output = cargo_command(&[
        "llvm-cov",
        "--workspace",
        "--all-features",
        "--ignore-filename-regex",
        "xtask/",
        "--summary-only",
    ])
    .output()
    .context("failed to run `cargo llvm-cov`")?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        bail!("`cargo llvm-cov` failed");
    }
    let summary = String::from_utf8_lossy(&output.stdout);
    let failures = coverage_failures(&summary);
    if failures.is_empty() {
        return Ok(());
    }
    for failure in &failures {
        eprintln!("coverage: {failure}");
    }
    bail!("coverage gate failed for {} target(s)", failures.len())
}

fn coverage_failures(summary: &str) -> Vec<String> {
    let mut totals: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for line in summary.lines() {
        let Some((crate_name, lines, missed)) = coverage_row(line) else {
            continue;
        };
        let entry = totals.entry(crate_name).or_insert((0, 0));
        entry.0 = entry.0.saturating_add(lines);
        entry.1 = entry.1.saturating_add(missed);
    }
    let mut failures = Vec::new();
    let mut overall = (0_u64, 0_u64);
    for (crate_name, (lines, missed)) in &totals {
        overall.0 = overall.0.saturating_add(*lines);
        overall.1 = overall.1.saturating_add(*missed);
        if *lines == 0 {
            continue;
        }
        let threshold = crate_threshold(crate_name);
        let Some(percent) = lines
            .saturating_sub(*missed)
            .saturating_mul(100)
            .checked_div(*lines)
        else {
            continue;
        };
        println!("coverage: {crate_name}: {percent}% (threshold {threshold}%)");
        if percent < threshold {
            failures.push(format!(
                "{crate_name}: {percent}% < {threshold}% ({} of {lines} lines)",
                lines.saturating_sub(*missed)
            ));
        }
    }
    let overall_percent = overall
        .0
        .saturating_sub(overall.1)
        .saturating_mul(100)
        .checked_div(overall.0);
    if let Some(percent) = overall_percent {
        if percent < 70 {
            failures.push(format!("workspace: {percent}% < 70%"));
        }
    }
    failures
}

fn coverage_row(line: &str) -> Option<(String, u64, u64)> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let path = fields.first()?;
    let crate_name = path
        .split('/')
        .find(|part| part.starts_with("ansilint-"))?
        .to_string();
    let lines = fields.get(7)?.parse::<u64>().ok()?;
    let missed = fields.get(8)?.parse::<u64>().ok()?;
    Some((crate_name, lines, missed))
}

fn crate_threshold(crate_name: &str) -> u64 {
    if matches!(
        crate_name,
        "ansilint-core"
            | "ansilint-yaml"
            | "ansilint-model"
            | "ansilint-config"
            | "ansilint-rules"
            | "ansilint-fix"
    ) {
        85
    } else {
        70
    }
}

fn run_tool(args: &[&str]) -> Result<()> {
    let status = cargo_command(args)
        .status()
        .with_context(|| format!("failed to run `cargo {}`", args.join(" ")))?;
    if !status.success() {
        let tool = args.first().copied().unwrap_or_default();
        bail!("`cargo {tool}` failed; install it with `cargo install cargo-{tool}`");
    }
    Ok(())
}

fn cargo_command(args: &[&str]) -> Command {
    let mut command = Command::new("cargo");
    command.args(args).current_dir(workspace::root());
    // `cargo run` injects its own package variables into our process. Nested
    // cargo subcommands must see a clean environment (cargo-machete, for
    // example, misreads CARGO_PKG_NAME as a path argument).
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if name.starts_with("CARGO_PKG_")
            || matches!(
                name.as_ref(),
                "CARGO_MANIFEST_DIR"
                    | "CARGO_MANIFEST_PATH"
                    | "CARGO_PRIMARY_PACKAGE"
                    | "RUST_RECURSION_COUNT"
            )
        {
            command.env_remove(&key);
        }
    }
    command
}

fn setup() -> Result<()> {
    let root = workspace::root();
    print_version("rustc", &["--version"])?;
    print_version("cargo", &["--version"])?;
    install_git_hook(&root)?;
    report_missing_tools();
    Ok(())
}

fn print_version(tool: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(tool)
        .args(args)
        .output()
        .with_context(|| format!("failed to run `{tool}`"))?;
    if !output.status.success() {
        bail!("`{tool}` exited with a failure");
    }
    println!("{}", String::from_utf8_lossy(&output.stdout).trim());
    Ok(())
}

fn install_git_hook(root: &Path) -> Result<()> {
    let hooks = root.join(".git/hooks");
    if !hooks.is_dir() {
        println!("no .git/hooks directory; skipping git hook installation");
        return Ok(());
    }
    let hook = hooks.join("pre-commit");
    if hook.exists() {
        println!("pre-commit hook already exists; leaving it untouched");
        return Ok(());
    }
    let script = "#!/bin/sh\nset -eu\ncargo fmt --all -- --check\ncargo clippy --workspace --all-targets -- -D warnings\n";
    fs::write(&hook, script).with_context(|| format!("failed to write {}", hook.display()))?;
    set_executable(&hook)?;
    println!("installed .git/hooks/pre-commit");
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("failed to mark {} executable", path.display()))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn report_missing_tools() {
    let tools = [
        ("cargo-deny", "deny"),
        ("cargo-machete", "machete"),
        ("cargo-llvm-cov", "llvm-cov"),
    ];
    for (package, subcommand) in tools {
        let available = cargo_command(&[subcommand, "--version"])
            .output()
            .is_ok_and(|output| output.status.success());
        if !available {
            println!("missing optional tool: {package} (install with `cargo install {package}`)");
        }
    }
}
