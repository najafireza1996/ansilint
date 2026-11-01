//! Project root, configuration discovery, file walking and kind detection.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use ansilint_config::{Config, KindPattern, CONFIG_FILE_NAMES};
use ansilint_model::{is_import_playbook, FileKind};
use ansilint_yaml::{Document, Node};
use globset::{Glob, GlobBuilder, GlobMatcher};
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
        .map(|path| normalize(cwd, path))
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
        // Upstream: `.cache/` is "implicit unless exclude_paths is defined in
        // config", resolved like any config entry (relative to the config).
        if config.exclude_paths.is_empty() {
            set.add_prefix(config_base.join(".cache"));
        }
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
            self.add_prefix(normalize(base, path));
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
    /// Builds a classifier from user patterns followed by defaults. Unknown
    /// kind names and invalid globs are skipped with a notice (H-D4).
    #[must_use]
    pub(crate) fn new(user: &[KindPattern]) -> (Self, Vec<String>) {
        let mut rules = Vec::new();
        let mut notices = Vec::new();
        for pattern in user {
            let Some(kind) = FileKind::from_name(&pattern.kind) else {
                notices.push(format!("kinds: unknown file kind `{}`", pattern.kind));
                continue;
            };
            match compile_glob(&pattern.glob) {
                Some(glob) => rules.push((kind, glob)),
                None => notices.push(format!(
                    "kinds: invalid glob `{}` for `{}`",
                    pattern.glob, pattern.kind
                )),
            }
        }
        // Only upstream's unclosed-brace rulebook glob fails here; it matches
        // nothing upstream either (pinned by a test).
        for (name, pattern) in default_kinds() {
            if let (Some(kind), Some(glob)) = (FileKind::from_name(name), compile_glob(pattern)) {
                rules.push((kind, glob));
            }
        }
        (Self { rules }, notices)
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

/// Upstream `DEFAULT_KINDS` (H-E2), checked in as data so the facts live in
/// one place (ENG-15).
const DEFAULT_KINDS: &str = include_str!("../../../data/default_kinds.toml");

/// `(kind name, glob)` pairs in upstream order. Entry lines have the fixed
/// shape `["kind", "glob"],`; everything else in the file is header.
fn default_kinds() -> impl Iterator<Item = (&'static str, &'static str)> {
    DEFAULT_KINDS.lines().filter_map(|line| {
        line.trim()
            .strip_prefix("[\"")?
            .strip_suffix("\"],")?
            .split_once("\", \"")
    })
}

/// Compiles a glob with upstream (wcmatch) semantics: `*` never crosses `/`,
/// and a `**` that is not a whole path component acts like `*`.
fn compile_glob(pattern: &str) -> Option<GlobMatcher> {
    let translated: Vec<String> = pattern
        .split('/')
        .map(|component| {
            if component != "**" && component.contains("**") {
                component.replace("**", "*")
            } else {
                component.to_string()
            }
        })
        .collect();
    GlobBuilder::new(&translated.join("/"))
        .literal_separator(true)
        .build()
        .ok()
        .map(|glob| glob.compile_matcher())
}

/// Improves a path-only classification by sniffing the parsed content.
///
/// Only playbooks are promoted: a top-level list item, in any document, with
/// `hosts` or an `import_playbook` key (H-A3). Task files are deliberately never
/// inferred from arbitrary YAML lists: unknown is not Tasks, because linting
/// unrelated YAML as Ansible content creates false positives.
#[must_use]
pub(crate) fn sniff(kind: FileKind, documents: &[Document]) -> FileKind {
    if !matches!(kind, FileKind::Yaml | FileKind::Unknown) {
        return kind;
    }
    let is_play = |mapping: &ansilint_yaml::Mapping| {
        mapping.entries.iter().any(|entry| {
            entry
                .key
                .as_str()
                .is_some_and(|key| key == "hosts" || is_import_playbook(key))
        })
    };
    let found = documents.iter().any(|document| {
        document.root.as_sequence().is_some_and(|sequence| {
            sequence
                .items
                .iter()
                .filter_map(Node::as_mapping)
                .any(is_play)
        })
    });
    if found {
        FileKind::Playbook
    } else {
        kind
    }
}

/// The one normal form for config bases, inputs and exclusions (H-C4):
/// absolute against `cwd`, with `.` and `..` resolved lexically. Lexical on
/// purpose: `canonicalize` follows symlinks and adds `\\?\` on Windows.
#[must_use]
pub(crate) fn normalize(cwd: &Path, path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let mut normal = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other),
        }
    }
    normal
}

/// Makes a normalized path relative to the project root, using forward
/// slashes. Paths outside the root get `../` segments; only a path with no
/// common prefix at all (another Windows drive) stays absolute.
#[must_use]
pub(crate) fn relative_output_path(root: &Path, path: &Path) -> PathBuf {
    let common = root
        .components()
        .zip(path.components())
        .take_while(|(left, right)| left == right)
        .count();
    let relative: PathBuf = if common == 0 {
        path.to_path_buf()
    } else {
        root.components()
            .skip(common)
            .map(|_| Component::ParentDir)
            .chain(path.components().skip(common))
            .collect()
    };
    PathBuf::from(relative.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests;
