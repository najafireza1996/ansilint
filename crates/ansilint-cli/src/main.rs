//! The `ansilint` command-line binary.
//!
//! It must not: implement linting logic itself; it parses arguments and delegates
//! to `ansilint-engine` (and `ansilint-lsp` once the language server exists).
#![forbid(unsafe_code)]
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the CLI writes user-facing output"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ansilint_engine::{generate_ignore, lint, ConfigOverrides, EngineError, LintOptions, Profile};
use ansilint_output::Format;
use anyhow::Result;
use clap::Parser;

/// A fast, single-binary Ansible linter that needs no Python.
#[expect(
    clippy::struct_excessive_bools,
    reason = "CLI flags are naturally boolean switches"
)]
#[derive(Debug, Parser)]
#[command(name = "ansilint", version, about, long_about = None)]
struct Cli {
    /// Files or directories to lint.
    #[arg(value_name = "PATHS")]
    paths: Vec<PathBuf>,
    /// Explicit configuration file.
    #[arg(short = 'c', long = "config-file", value_name = "FILE")]
    config_file: Option<PathBuf>,
    /// Minimum profile.
    #[arg(short = 'p', long = "profile", value_name = "PROFILE", value_parser = parse_profile)]
    profile: Option<Profile>,
    /// Tags to hide.
    #[arg(
        short = 'x',
        long = "skip-list",
        value_delimiter = ',',
        value_name = "RULES"
    )]
    skip_list: Vec<String>,
    /// Tags to report as warnings.
    #[arg(
        short = 'w',
        long = "warn-list",
        value_delimiter = ',',
        value_name = "RULES"
    )]
    warn_list: Vec<String>,
    /// Only report rules whose id, `rule[sub]` tag or rule tag matches.
    #[arg(short = 't', long = "tags", value_delimiter = ',', value_name = "TAGS")]
    tags: Vec<String>,
    /// Opt-in rules to enable.
    #[arg(long = "enable-list", value_delimiter = ',', value_name = "RULES")]
    enable_list: Vec<String>,
    /// Paths to exclude, relative to the current directory.
    #[arg(long = "exclude", value_name = "PATH")]
    exclude: Vec<PathBuf>,
    /// Treat warnings as failures.
    #[arg(long = "strict")]
    strict: bool,
    /// Suppress non-finding output.
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
    /// Increase verbosity.
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbosity: u8,
    /// Output format.
    #[arg(
        short = 'f',
        long = "format",
        default_value = "pep8",
        value_name = "FORMAT",
        value_parser = parse_format
    )]
    format: Format,
    /// Accepted for compatibility; linting is always offline.
    #[arg(long = "offline")]
    offline: bool,
    /// Disable ANSI colors (no colors are emitted in Phase 1).
    #[arg(long = "nocolor")]
    nocolor: bool,
    /// Rewrite the ignore file from current findings.
    #[arg(long = "generate-ignore")]
    generate_ignore: bool,
    /// List rules.
    #[arg(short = 'L', long = "list-rules")]
    list_rules: bool,
    /// List tags.
    #[arg(short = 'T', long = "list-tags")]
    list_tags: bool,
    /// List profiles.
    #[arg(short = 'P', long = "list-profiles")]
    list_profiles: bool,
}

/// Exit codes, verified against upstream ansible-lint `constants.RC` at commit
/// 67a48717843c4d5d37681294cb5c9967781b0a31 (H-E1). Usage errors exit 2 like
/// upstream's `argparse`; Ctrl-C keeps the default SIGINT termination, which
/// shells report as 130. `4` (lock timeout) and `8` (fixed) are not emitted.
const VIOLATIONS_FOUND: u8 = 2;
const INVALID_CONFIG: u8 = 3;
const NO_FILES_MATCHED: u8 = 5;
/// Our own unexpected failure outside rule isolation (upstream: a traceback).
const INTERNAL_FAILURE: u8 = 1;

fn main() -> ExitCode {
    // clap exits 2 on usage errors and 0 for --help/--version.
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ansilint: {error:#}");
            ExitCode::from(failure_code(&error))
        }
    }
}

fn failure_code(error: &anyhow::Error) -> u8 {
    match error.downcast_ref::<EngineError>() {
        Some(EngineError::Config(_) | EngineError::Load { .. } | EngineError::Ignore(_)) => {
            INVALID_CONFIG
        }
        _ => INTERNAL_FAILURE,
    }
}

fn run(cli: &Cli) -> Result<ExitCode> {
    if cli.list_rules {
        print_rules();
    }
    if cli.list_tags {
        print_tags();
    }
    if cli.list_profiles {
        print_profiles();
    }
    if cli.list_rules || cli.list_tags || cli.list_profiles {
        return Ok(ExitCode::SUCCESS);
    }
    let options = build_options(cli)?;
    let result = lint(&options)?;
    if cli.generate_ignore {
        let path = ignore_path(&result, &options.cwd);
        generate_ignore(&result, &path)?;
        println!("wrote {}", path.display());
        return Ok(ExitCode::SUCCESS);
    }
    if !result.config.quiet {
        for warning in &result.warnings {
            eprintln!("ansilint: warning: {warning}");
        }
    }
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    result.render(cli.format, &mut lock)?;
    Ok(if result.has_fatal(result.config.strict) {
        ExitCode::from(VIOLATIONS_FOUND)
    } else if result.files_linted == 0 {
        ExitCode::from(NO_FILES_MATCHED)
    } else {
        ExitCode::SUCCESS
    })
}

fn build_options(cli: &Cli) -> Result<LintOptions> {
    let cwd = std::env::current_dir()?;
    let mut options = LintOptions::new(&cwd);
    options.paths.clone_from(&cli.paths);
    options.config_path.clone_from(&cli.config_file);
    options.exclude_paths.clone_from(&cli.exclude);
    options.overrides = ConfigOverrides {
        profile: cli.profile,
        skip_list: cli.skip_list.clone(),
        warn_list: cli.warn_list.clone(),
        enable_list: cli.enable_list.clone(),
        tags: cli.tags.clone(),
        quiet: cli.quiet.then_some(true),
        strict: cli.strict.then_some(true),
        verbosity: (cli.verbosity > 0).then_some(cli.verbosity),
        ..ConfigOverrides::default()
    };
    options.ignore_file = std::env::var_os("ANSIBLE_LINT_IGNORE_FILE").map(PathBuf::from);
    if let Some(path) = &options.ignore_file {
        if path.is_relative() {
            options.ignore_file = Some(cwd.join(path));
        }
    }
    Ok(options)
}

fn parse_profile(value: &str) -> Result<Profile, String> {
    value
        .parse()
        .map_err(|()| format!("unknown profile `{value}`"))
}

fn parse_format(value: &str) -> Result<Format, String> {
    Format::from_name(value).ok_or_else(|| format!("unknown format `{value}`"))
}

fn ignore_path(result: &ansilint_engine::LintResult, cwd: &Path) -> PathBuf {
    if let Some(path) = &result.ignore_file_path {
        return path.clone();
    }
    if let Some(config) = &result.config_path {
        if let Some(parent) = config.parent() {
            return parent.join(".ansible-lint-ignore");
        }
    }
    cwd.join(".ansible-lint-ignore")
}

fn print_rules() {
    for rule in ansilint_engine::rules() {
        let meta = rule.meta();
        println!("{:<24} {:<12} {}", meta.id, meta.profile, meta.summary);
    }
}

fn print_tags() {
    let mut tags = BTreeSet::new();
    for rule in ansilint_engine::rules() {
        tags.extend(rule.meta().tags.iter().copied());
    }
    for tag in tags {
        println!("{tag}");
    }
}

fn print_profiles() {
    for profile in Profile::ALL {
        println!("{profile}");
    }
}
