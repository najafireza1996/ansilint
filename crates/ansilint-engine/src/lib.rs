//! Orchestration of discovery, loading, model building, rules, post-processing and reporting.
//!
//! It must not: implement rule logic or output formatting itself.
#![forbid(unsafe_code)]

mod discovery;
mod error;
mod loader;
mod post;

use std::io::Write;
use std::path::{Path, PathBuf};

use ansilint_config::{merge, Config, IgnoreFile, IGNORE_FILE_NAMES};
use ansilint_core::{Finding, Severity, Span, Tag};
use ansilint_model::build;
use ansilint_output::{DisplayFinding, Format, OutputError};
use ansilint_rules::FileCx;
use ansilint_yaml::parse;
use rayon::prelude::*;

pub use ansilint_config::{ConfigOverrides, Profile};
pub use ansilint_rules::{Rule, RuleMeta};
pub use error::EngineError;
pub use loader::DEFAULT_MAX_FILE_BYTES;

/// Returns the rule registry for listings.
#[must_use]
pub fn rules() -> &'static [&'static dyn Rule] {
    ansilint_rules::all()
}

use discovery::{Classifier, ExcludeSet, Project};

/// Options for one lint run.
#[derive(Clone, Debug)]
pub struct LintOptions {
    /// Working directory that relative paths are resolved against.
    pub cwd: PathBuf,
    /// Input paths; empty means the current directory.
    pub paths: Vec<PathBuf>,
    /// Explicit configuration file.
    pub config_path: Option<PathBuf>,
    /// CLI configuration overrides.
    pub overrides: ConfigOverrides,
    /// CLI exclusions, resolved relative to `cwd`.
    pub exclude_paths: Vec<PathBuf>,
    /// Explicit ignore file; otherwise discovered next to the config.
    pub ignore_file: Option<PathBuf>,
    /// Maximum file size in bytes.
    pub max_file_bytes: u64,
}

impl LintOptions {
    /// Creates options for a working directory with all defaults.
    #[must_use]
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            paths: Vec::new(),
            config_path: None,
            overrides: ConfigOverrides::default(),
            exclude_paths: Vec::new(),
            ignore_file: None,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
}

/// A finding with resolved one-based line and column.
#[derive(Clone, Debug)]
pub struct PositionedFinding {
    /// The finding.
    pub finding: Finding,
    /// One-based line.
    pub line: u32,
    /// One-based column in Unicode scalar values.
    pub col: u32,
}

/// The result of a lint run.
#[derive(Clone, Debug)]
pub struct LintResult {
    /// Findings, sorted deterministically.
    pub findings: Vec<PositionedFinding>,
    /// Number of files considered.
    pub files_linted: usize,
    /// Effective configuration.
    pub config: Config,
    /// Configuration file used, if any.
    pub config_path: Option<PathBuf>,
    /// Ignore file used, if any.
    pub ignore_file_path: Option<PathBuf>,
    /// Non-finding warnings (unknown config keys, no files, …).
    pub warnings: Vec<String>,
}

impl LintResult {
    /// Returns `true` when the exit code should be non-zero.
    #[must_use]
    pub fn has_fatal(&self, strict: bool) -> bool {
        self.findings.iter().any(|positioned| {
            positioned.finding.severity == Severity::Error
                || (strict && positioned.finding.severity == Severity::Warning)
        })
    }

    /// Renders findings in the selected format.
    ///
    /// # Errors
    ///
    /// Returns [`OutputError`] when writing or serialization fails.
    pub fn render(&self, format: Format, out: &mut impl Write) -> Result<(), OutputError> {
        let display: Vec<DisplayFinding<'_>> = self
            .findings
            .iter()
            .map(|positioned| DisplayFinding {
                finding: &positioned.finding,
                line: positioned.line,
                col: positioned.col,
            })
            .collect();
        ansilint_output::render(format, &display, out)
    }
}

/// Lints the configured paths.
///
/// # Errors
///
/// Returns [`EngineError`] for configuration, discovery or I/O failures that
/// prevent a meaningful run. Per-file load and parse problems are findings.
pub fn lint(options: &LintOptions) -> Result<LintResult, EngineError> {
    let project = discovery::discover_project(&options.cwd, options.config_path.as_deref());
    let (mut config, mut warnings) = load_config(project.config_path.as_deref())?;
    config = merge(&config, &options.overrides);
    let ignore_file_path = discover_ignore_file(options, &project);
    let ignore = load_ignore_file(ignore_file_path.as_deref())?;
    let classifier = Classifier::new(&config.kinds);
    let excludes = ExcludeSet::new(
        &config,
        &project.config_base,
        &options.exclude_paths,
        &options.cwd,
    );
    let inputs = resolve_inputs(options);
    let files = discovery::collect_files(&inputs, &excludes)?;
    if files.is_empty() {
        warnings.push("no YAML files were found to lint".to_string());
    }
    let active_rules = active_rules(&config);
    let outcomes: Vec<FileOutcome> = files
        .par_iter()
        .map(|path| {
            process_file(
                path,
                &project,
                &config,
                &classifier,
                ignore.as_ref(),
                &active_rules,
                options.max_file_bytes,
            )
        })
        .collect();
    let mut findings = Vec::new();
    for outcome in outcomes {
        findings.extend(outcome.findings);
        warnings.extend(outcome.warnings);
    }
    post::sort_and_dedupe(&mut findings);
    Ok(LintResult {
        findings,
        files_linted: files.len(),
        config,
        config_path: project.config_path,
        ignore_file_path,
        warnings,
    })
}

/// Writes an ignore file from the current findings.
///
/// # Errors
///
/// Returns [`EngineError`] when the file cannot be written.
pub fn generate_ignore(result: &LintResult, path: &Path) -> Result<(), EngineError> {
    let mut lines = Vec::new();
    for positioned in &result.findings {
        lines.push(format!(
            "{} {}  # {}",
            positioned.finding.path.to_string_lossy().replace('\\', "/"),
            positioned.finding.tag,
            positioned.finding.message.replace('\n', " ")
        ));
    }
    lines.sort();
    lines.dedup();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|source| EngineError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }
    let contents = if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    };
    std::fs::write(path, contents).map_err(|source| EngineError::Write {
        path: path.to_path_buf(),
        source,
    })
}

struct FileOutcome {
    findings: Vec<PositionedFinding>,
    warnings: Vec<String>,
}

fn load_error_finding(error: &loader::LoadFailure) -> Finding {
    Finding::new(
        Tag::new("load-failure"),
        error.to_string(),
        PathBuf::new(),
        Span::empty(0),
        Severity::Error,
    )
}

fn parser_error_finding(error: &ansilint_yaml::YamlError) -> Finding {
    Finding::new(
        Tag::new("parser-error"),
        error.message.clone(),
        PathBuf::new(),
        error.span,
        Severity::Error,
    )
}

fn load_config(path: Option<&Path>) -> Result<(Config, Vec<String>), EngineError> {
    let Some(path) = path else {
        return Ok((Config::default(), Vec::new()));
    };
    let text = read_to_string(path)?;
    let parsed = Config::from_bytes(&text)?;
    let mut warnings = Vec::new();
    for key in &parsed.ignored_keys {
        warnings.push(format!("configuration key `{key}` is accepted but ignored"));
    }
    for key in &parsed.unknown_keys {
        warnings.push(format!("unknown configuration key `{key}`"));
    }
    Ok((parsed.config, warnings))
}

fn discover_ignore_file(options: &LintOptions, project: &Project) -> Option<PathBuf> {
    if let Some(path) = &options.ignore_file {
        return Some(path.clone());
    }
    let base = project
        .config_path
        .as_ref()
        .and_then(|path| path.parent())
        .unwrap_or(&project.root);
    for name in IGNORE_FILE_NAMES {
        for directory in [base, project.root.as_path()] {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn load_ignore_file(path: Option<&Path>) -> Result<Option<IgnoreFile>, EngineError> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = read_to_string(path)?;
    Ok(Some(IgnoreFile::parse(&text)?))
}

fn read_to_string(path: &Path) -> Result<String, EngineError> {
    let mut text = std::fs::read_to_string(path).map_err(|source| EngineError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if let Some(stripped) = text.strip_prefix('\u{feff}') {
        text = stripped.to_string();
    }
    Ok(text)
}

fn resolve_inputs(options: &LintOptions) -> Vec<PathBuf> {
    if options.paths.is_empty() {
        return vec![options.cwd.clone()];
    }
    options
        .paths
        .iter()
        .map(|path| {
            if path.is_absolute() {
                path.clone()
            } else {
                options.cwd.join(path)
            }
        })
        .collect()
}

fn active_rules(config: &Config) -> Vec<&'static dyn Rule> {
    if !config.use_default_rules {
        return Vec::new();
    }
    ansilint_rules::all()
        .iter()
        .copied()
        .filter(|rule| {
            let meta = rule.meta();
            config.profile.includes(meta.profile)
                && (!meta.opt_in || matches_pattern(&config.enable_list, meta.id))
        })
        .collect()
}

fn matches_pattern(patterns: &[String], id: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| pattern.split('[').next().is_some_and(|head| head == id))
}

#[allow(
    clippy::too_many_arguments,
    reason = "process_file mirrors the pipeline stages"
)]
fn process_file(
    path: &Path,
    project: &Project,
    config: &Config,
    classifier: &Classifier,
    ignore: Option<&IgnoreFile>,
    active_rules: &[&dyn Rule],
    max_file_bytes: u64,
) -> FileOutcome {
    let output_path = discovery::relative_output_path(&project.root, path);
    let source = match loader::load(path, max_file_bytes) {
        Ok(source) => source,
        Err(error) => {
            let finding = load_error_finding(&error);
            return FileOutcome {
                findings: post::process(vec![finding], None, &output_path, config, ignore),
                warnings: Vec::new(),
            };
        }
    };
    let documents = match parse(&source) {
        Ok(documents) => documents,
        Err(error) => {
            return FileOutcome {
                findings: post::process(
                    vec![parser_error_finding(&error)],
                    Some(&source),
                    &output_path,
                    config,
                    ignore,
                ),
                warnings: Vec::new(),
            };
        }
    };
    let kind = discovery::sniff(classifier.classify(&project.root, path), &documents);
    let built = build(&documents, kind);
    let cx = FileCx {
        source: &source,
        documents: &documents,
        kind,
        built: &built,
        config,
    };
    let raw = run_rules(&cx, active_rules);
    FileOutcome {
        findings: post::process(raw, Some(&source), &output_path, config, ignore),
        warnings: Vec::new(),
    }
}

fn run_rules(cx: &FileCx<'_>, active_rules: &[&dyn Rule]) -> Vec<Finding> {
    // Model issues are retained for the schema/parser-error phases; surfacing
    // them as warnings today would create false positives on valid YAML.
    ansilint_rules::run(active_rules, cx)
}
