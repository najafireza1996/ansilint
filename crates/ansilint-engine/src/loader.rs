//! Reading a file once and validating that it is UTF-8 text.

use std::io;
use std::path::Path;

use ansilint_yaml::SourceFile;
use thiserror::Error;

/// Default maximum file size (5 MiB).
pub const DEFAULT_MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;

/// Why a file could not be loaded.
#[derive(Debug, Error)]
pub(crate) enum LoadFailure {
    /// The file exceeds the configured limit.
    #[error("file is {size} bytes, above the {limit} byte limit")]
    TooLarge {
        /// Observed size.
        size: u64,
        /// Configured limit.
        limit: u64,
    },
    /// The file is not valid UTF-8.
    #[error("file is not valid UTF-8")]
    NotUtf8,
    /// The file could not be read.
    #[error("could not read file: {0}")]
    Io(#[from] io::Error),
}

/// Loads a file, stripping a UTF-8 BOM and preserving CRLF.
///
/// # Errors
///
/// Returns [`LoadFailure`] for I/O errors, oversized files or invalid UTF-8.
pub(crate) fn load(path: &Path, limit: u64) -> Result<SourceFile, LoadFailure> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > limit {
        return Err(LoadFailure::TooLarge {
            size: metadata.len(),
            limit,
        });
    }
    let bytes = std::fs::read(path)?;
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > limit {
        return Err(LoadFailure::TooLarge { size, limit });
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| LoadFailure::NotUtf8)?;
    Ok(SourceFile::new(path.to_path_buf(), text.to_string()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, reason = "tests use fixed fixtures")]

    use std::fs;

    use super::{load, LoadFailure};

    #[test]
    fn strips_a_bom_and_keeps_crlf() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("a.yml");
        fs::write(&path, b"\xEF\xBB\xBFa: 1\r\n").expect("write");
        let source = load(&path, 1024).expect("load");
        assert_eq!(source.as_str(), "a: 1\r\n");
    }

    #[test]
    fn rejects_invalid_utf8() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("a.yml");
        fs::write(&path, [0xFF, 0xFE]).expect("write");
        assert!(matches!(load(&path, 1024), Err(LoadFailure::NotUtf8)));
    }

    #[test]
    fn rejects_oversized_files() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("a.yml");
        fs::write(&path, "a: 1\n").expect("write");
        assert!(matches!(load(&path, 2), Err(LoadFailure::TooLarge { .. })));
    }
}
