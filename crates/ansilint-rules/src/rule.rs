//! The rule trait and the context it receives.

use ansilint_config::Config;
use ansilint_core::Finding;
use ansilint_model::{Block, BuiltModel, FileKind, Play, Task};
use ansilint_yaml::{Document, SourceFile};

use crate::RuleMeta;

/// Read-only context for one file.
pub struct FileCx<'a> {
    /// Source text and line index.
    pub source: &'a SourceFile,
    /// Parsed documents.
    pub documents: &'a [Document],
    /// Detected kind.
    pub kind: FileKind,
    /// Ansible-aware model and model issues.
    pub built: &'a BuiltModel,
    /// Effective configuration.
    pub config: &'a Config,
}

/// A pure rule.
///
/// Implementations must be deterministic, side-effect free and thread-safe.
pub trait Rule: Send + Sync {
    /// Static metadata.
    fn meta(&self) -> &'static RuleMeta;

    /// Called once per file.
    fn check_file(&self, _cx: &FileCx<'_>, _out: &mut Vec<Finding>) {}

    /// Called for every play of a playbook.
    fn check_play(&self, _cx: &FileCx<'_>, _play: &Play, _out: &mut Vec<Finding>) {}

    /// Called for every task, including handlers and includes.
    fn check_task(&self, _cx: &FileCx<'_>, _task: &Task, _out: &mut Vec<Finding>) {}

    /// Called for every block before its children.
    fn check_block(&self, _cx: &FileCx<'_>, _block: &Block, _out: &mut Vec<Finding>) {}

    /// Called for raw-text rules (`no-tabs`, future `yaml[*]`).
    fn check_text(&self, _cx: &FileCx<'_>, _out: &mut Vec<Finding>) {}
}
