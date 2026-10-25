//! Project root, configuration discovery, file walking and kind detection.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ansilint_config::{Config, KindPattern, CONFIG_FILE_NAMES};
use ansilint_model::FileKind;
use ansilint_yaml::{Document, Node};
use globset::{Glob, GlobMatcher};
use ignore::WalkBuilder;

use crate::error::EngineError;

/// Project-level paths.
#[derive(Clone, Debug)]
pub(crate) struct Project {
    /// Git root, or the starting directory when there is no git repository.
    pub root: PathBuf,
    /// Discovered or explicit configuration file.
    pub config_path: Option<PathBuf>,
    /// Directory that relative configuration paths are resolved against.
    pub config_base: PathBuf,
}

/// Finds the git root by walking upwards.
#[must_use]
pub(crate) fn find_git_root(cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        current = current.parent()?.to_path_buf();
    }
}

/// Discovers the project root and configuration file.
#[must_use]
pub(crate) fn discover_project(cwd: &Path, explicit_config: Option<&Path>) -> Project {
    let git_root = find_git_root(cwd);
    let root = git_root.clone().unwrap_or_else(|| cwd.to_path_buf());
    let config_path = explicit_config
        .map(|path| {
            if path.is_relative() {
                cwd.join(path)
            } else {
                path.to_path_buf()
            }
        })
        .or_else(|| find_config(cwd, git_root.as_deref()));
    let config_base = config_path
        .as_ref()
        .and_then(|path| path.parent())
        .map_or_else(|| cwd.to_path_buf(), Path::to_path_buf);
    Project {
        root,
        config_path,
        config_base,
    }
}

fn find_config(cwd: &Path, stop_at: Option<&Path>) -> Option<PathBuf> {
    let mut directory = cwd.to_path_buf();
    loop {
        for name in CONFIG_FILE_NAMES {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if stop_at.is_some_and(|root| directory == root) {
            return None;
        }
        let parent = directory.parent()?;
        directory = parent.to_path_buf();
    }
}

/// Paths that are never linted.
#[derive(Clone, Debug, Default)]
pub(crate) struct ExcludeSet {
    prefixes: Vec<PathBuf>,
    globs: Vec<(PathBuf, GlobMatcher)>,
}

impl ExcludeSet {
    /// Builds exclusions from configuration and CLI paths.
    #[must_use]
    pub(crate) fn new(
        config: &Config,
        config_base: &Path,
        cli_paths: &[PathBuf],
        cwd: &Path,
    ) -> Self {
        let mut set = Self::default();
        set.add_prefix(cwd.join(".git"));
        set.add_prefix(cwd.join("target"));
        set.add_prefix(cwd.join(".cache"));
        for path in &config.exclude_paths {
            set.add(config_base, path);
        }
        for path in cli_paths {
            set.add(cwd, path);
        }
        set
    }

    fn add(&mut self, base: &Path, path: &Path) {
        if contains_glob(path) {
            if let Ok(glob) = Glob::new(&path.to_string_lossy()) {
                self.globs
                    .push((base.to_path_buf(), glob.compile_matcher()));
            }
        } else {
            self.add_prefix(base.join(path));
        }
    }

    fn add_prefix(&mut self, path: PathBuf) {
        self.prefixes.push(path);
    }

    /// Returns `true` when a path is excluded.
    #[must_use]
    pub(crate) fn is_excluded(&self, path: &Path) -> bool {
        if self.prefixes.iter().any(|prefix| path.starts_with(prefix)) {
            return true;
        }
        self.globs.iter().any(|(base, glob)| {
            let relative = path.strip_prefix(base).unwrap_or(path);
            glob.is_match(relative.to_string_lossy().replace('\\', "/"))
        })
    }
}

fn contains_glob(path: &Path) -> bool {
    path.to_string_lossy()
        .chars()
        .any(|character| matches!(character, '*' | '?' | '[' | '{'))
}

/// Walks the given inputs and returns sorted YAML files.
///
/// # Errors
///
/// Returns [`EngineError`] when a directory cannot be walked.
pub(crate) fn collect_files(
    inputs: &[PathBuf],
    excludes: &ExcludeSet,
) -> Result<Vec<PathBuf>, EngineError> {
    let mut files = BTreeSet::new();
    for input in inputs {
        if input.is_dir() {
            collect_directory(input, excludes, &mut files)?;
        } else if input.is_file() {
            files.insert(input.clone());
        }
    }
    Ok(files.into_iter().collect())
}

fn collect_directory(
    directory: &Path,
    excludes: &ExcludeSet,
    files: &mut BTreeSet<PathBuf>,
) -> Result<(), EngineError> {
    let filter = excludes.clone();
    let mut builder = WalkBuilder::new(directory);
    builder
        .standard_filters(true)
        .filter_entry(move |entry| !filter.is_excluded(entry.path()));
    for result in builder.build() {
        let entry = result?;
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() && has_yaml_extension(entry.path()) {
            files.insert(entry.path().to_path_buf());
        }
    }
    Ok(())
}

fn has_yaml_extension(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("yml") || extension.eq_ignore_ascii_case("yaml")
        })
}

/// Classifies files by configuration globs, then by content.
#[derive(Debug)]
pub(crate) struct Classifier {
    rules: Vec<(FileKind, GlobMatcher)>,
}

impl Classifier {
    /// Builds a classifier from user patterns followed by defaults.
    #[must_use]
    pub(crate) fn new(user: &[KindPattern]) -> Self {
        let mut rules = Vec::new();
        for pattern in user {
            let Some(kind) = FileKind::from_name(&pattern.kind) else {
                continue;
            };
            if let Ok(glob) = Glob::new(&pattern.glob) {
                rules.push((kind, glob.compile_matcher()));
            }
        }
        for (kind, pattern) in default_patterns() {
            if let Ok(glob) = Glob::new(pattern) {
                rules.push((*kind, glob.compile_matcher()));
            }
        }
        Self { rules }
    }

    /// Returns the first matching kind, or [`FileKind::Unknown`].
    #[must_use]
    pub(crate) fn classify(&self, root: &Path, path: &Path) -> FileKind {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let relative = relative.to_string_lossy().replace('\\', "/");
        self.rules
            .iter()
            .find(|(_, glob)| glob.is_match(&relative))
            .map_or(FileKind::Unknown, |(kind, _)| *kind)
    }
}

fn default_patterns() -> &'static [(FileKind, &'static str)] {
    &[
        (FileKind::Meta, "**/meta/main.{yml,yaml}"),
        (FileKind::MetaRuntime, "**/meta/runtime.{yml,yaml}"),
        (FileKind::RoleArgSpec, "**/meta/argument_specs.{yml,yaml}"),
        (FileKind::Galaxy, "**/galaxy.{yml,yaml}"),
        (FileKind::Requirements, "**/requirements.{yml,yaml}"),
        (FileKind::Molecule, "**/molecule/**/*.{yml,yaml}"),
        (
            FileKind::ExecutionEnvironment,
            "**/execution-environment.{yml,yaml}",
        ),
        (FileKind::Tasks, "**/tasks/**/*.{yml,yaml}"),
        (FileKind::Handlers, "**/handlers/**/*.{yml,yaml}"),
        (FileKind::Defaults, "**/defaults/**/*.{yml,yaml}"),
        (FileKind::Vars, "**/vars/**/*.{yml,yaml}"),
        (FileKind::Vars, "**/group_vars/**/*.{yml,yaml}"),
        (FileKind::Vars, "**/host_vars/**/*.{yml,yaml}"),
        (FileKind::Yaml, "**/*.{yml,yaml}"),
    ]
}

/// Improves a path-only classification by sniffing the parsed content.
///
/// Phase 1 only promotes playbooks (a top-level list item with `hosts`). It
/// deliberately does not infer task files from arbitrary YAML lists, because
/// that would lint unrelated YAML as Ansible content and create false positives.
#[must_use]
pub(crate) fn sniff(kind: FileKind, documents: &[Document]) -> FileKind {
    if !matches!(kind, FileKind::Yaml | FileKind::Unknown) {
        return kind;
    }
    for document in documents {
        let Some(sequence) = document.root.as_sequence() else {
            continue;
        };
        let Some(first) = sequence.items.first().and_then(Node::as_mapping) else {
            continue;
        };
        if first.get("hosts").is_some() {
            return FileKind::Playbook;
        }
    }
    kind
}

/// Makes a path relative to the project root, using forward slashes.
#[must_use]
pub(crate) fn relative_output_path(root: &Path, path: &Path) -> PathBuf {
    let relative = path.strip_prefix(root).unwrap_or(path);
    PathBuf::from(relative.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use ansilint_model::FileKind;

    use super::{
        default_patterns, find_git_root, has_yaml_extension, relative_output_path, Classifier,
    };

    #[test]
    fn defaults_are_ordered_from_specific_to_generic() {
        let patterns = default_patterns();
        assert_eq!(patterns.last().map(|(kind, _)| *kind), Some(FileKind::Yaml));
        assert!(patterns.len() > 5);
    }

    #[test]
    fn classifies_role_files() {
        let classifier = Classifier::new(&[]);
        let root = std::path::Path::new("/project");
        assert_eq!(
            classifier.classify(root, &root.join("roles/x/tasks/main.yml")),
            FileKind::Tasks
        );
        assert_eq!(
            classifier.classify(root, &root.join("roles/x/meta/main.yml")),
            FileKind::Meta
        );
        assert_eq!(
            classifier.classify(root, &root.join("playbooks/site.yml")),
            FileKind::Yaml
        );
    }

    #[test]
    fn finds_no_git_root_in_a_temp_directory() {
        let directory = tempfile::tempdir().expect("temp dir");
        assert!(find_git_root(directory.path()).is_none());
    }

    #[test]
    fn yaml_extensions_are_case_insensitive() {
        assert!(has_yaml_extension(std::path::Path::new("a.yml")));
        assert!(has_yaml_extension(std::path::Path::new("a.YML")));
        assert!(has_yaml_extension(std::path::Path::new("a.Yaml")));
        assert!(!has_yaml_extension(std::path::Path::new("a.txt")));
    }

    #[test]
    fn directory_patterns_are_case_sensitive() {
        let classifier = Classifier::new(&[]);
        let root = std::path::Path::new("/project");
        assert_eq!(
            classifier.classify(root, &root.join("roles/x/Tasks/main.yml")),
            FileKind::Yaml
        );
    }

    #[test]
    fn output_paths_use_forward_slashes() {
        let root = std::path::Path::new("/project");
        let path = std::path::Path::new("/project/roles\\x\\tasks\\main.yml");
        assert_eq!(
            relative_output_path(root, path),
            std::path::PathBuf::from("roles/x/tasks/main.yml")
        );
    }
}
