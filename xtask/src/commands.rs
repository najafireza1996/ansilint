//! Implementation of the `cargo xtask` subcommands.

use std::fs;
use std::path::Path;
use std::process::Command;

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
    run_tool(&[
        "llvm-cov",
        "--workspace",
        "--all-features",
        "--ignore-filename-regex",
        "xtask/",
        "--fail-under-lines",
        "70",
    ])?;
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
