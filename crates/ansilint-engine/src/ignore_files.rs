//! Ignore-file discovery, loading and generation.

use std::path::{Path, PathBuf};

use ansilint_config::{IgnoreEntry, IgnoreFile, IGNORE_FILE_NAMES};

use crate::discovery::Project;
use crate::{load_text, EngineError, LintOptions, LintResult};

/// Writes an ignore file from the current findings.
///
/// # Errors
///
/// Returns [`EngineError`] when the file cannot be written.
pub fn generate_ignore(result: &LintResult, path: &Path) -> Result<(), EngineError> {
    let mut lines = Vec::new();
    for positioned in &result.findings {
        let entry = IgnoreEntry {
            path: positioned.finding.path.to_string_lossy().replace('\\', "/"),
            tag: positioned.finding.tag.to_string(),
            skip: false,
            line: 0,
        };
        lines.push(format!(
            "{}  # {}",
            entry.to_line(),
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

pub(crate) fn discover_ignore_file(options: &LintOptions, project: &Project) -> Option<PathBuf> {
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

pub(crate) fn load_ignore_file(
    path: Option<&Path>,
    limit: u64,
) -> Result<Option<IgnoreFile>, EngineError> {
    let Some(path) = path else {
        return Ok(None);
    };
    Ok(Some(IgnoreFile::parse(load_text(path, limit)?.as_str())?))
}
