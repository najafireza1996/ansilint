//! Filesystem walk helpers shared by the workspace checks.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub(super) fn walk_files(directory: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files(directory, extension, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files(directory: &Path, extension: &str, files: &mut Vec<PathBuf>) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in read_entries(directory)? {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !should_skip_dir(&path) {
                collect_files(&path, extension, files)?;
            }
        } else if path
            .extension()
            .is_some_and(|candidate| candidate == extension)
        {
            files.push(path);
        }
    }
    Ok(())
}

pub(super) fn read_entries(directory: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
    {
        entries.push(entry?);
    }
    entries.sort_by_key(fs::DirEntry::path);
    Ok(entries)
}

pub(super) fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| matches!(name, "target" | ".git" | "node_modules" | ".venv"))
}

pub(super) fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
