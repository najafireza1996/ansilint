//! Ansible-aware model (plays, blocks, tasks, roles) built from the YAML AST.
//!
//! It must not: read files, resolve includes, or evaluate Jinja.
#![forbid(unsafe_code)]

mod build;
mod file_kind;
mod keywords;
mod model;

pub use build::build;
pub use file_kind::FileKind;
pub use keywords::{is_include_action, is_task_keyword, TASK_KEYWORDS};
pub use model::{
    Action, Block, BuiltModel, FileModel, KeyMap, KeyValue, ModelIssue, ModuleRef, Play, RoleRef,
    Spanned, Task, TaskCtx, TaskItem,
};
