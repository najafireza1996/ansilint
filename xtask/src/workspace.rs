//! Workspace-root discovery shared by xtask subcommands.

use std::path::{Path, PathBuf};

/// Returns the workspace root (the parent of the `xtask` crate).
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Renders a path relative to the workspace root for stable output.
pub(crate) fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
