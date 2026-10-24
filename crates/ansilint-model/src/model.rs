//! Ansible-aware model nodes.

use ansilint_core::Span;
use ansilint_yaml::Node;

/// A value with its source span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The value.
    pub value: T,
    /// Where the value was written.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Creates a spanned value.
    #[must_use]
    pub const fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

/// One key/value pair from a task, block or play mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyValue {
    /// Key and its span.
    pub key: Spanned<String>,
    /// Raw YAML value.
    pub value: Node,
}

/// Ordered, duplicate-preserving keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyMap {
    /// Entries in source order.
    pub entries: Vec<KeyValue>,
}

impl KeyMap {
    /// Returns the first value for a key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.entries
            .iter()
            .find(|entry| entry.key.value == key)
            .map(|entry| &entry.value)
    }

    /// Returns the span of the first key with this name.
    #[must_use]
    pub fn key_span(&self, key: &str) -> Option<Span> {
        self.entries
            .iter()
            .find(|entry| entry.key.value == key)
            .map(|entry| entry.key.span)
    }
}

/// Context inherited from the enclosing play, role or block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskCtx {
    /// Whether the task is inside a handlers list.
    pub in_handler: bool,
    /// Number of enclosing blocks.
    pub block_depth: u32,
    /// Index of the enclosing play, if any.
    pub play_index: Option<usize>,
    /// Enclosing role name, if known.
    pub role: Option<String>,
}

/// The normalized module reference of a task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleRef {
    /// Name as written (or parsed from a legacy form).
    pub requested: String,
    /// Canonical FQCN once an index is available.
    pub resolved_fqcn: Option<String>,
    /// Whether this came from `action:` or `local_action:`.
    pub is_legacy_action_form: bool,
}

/// A task action: the module key plus its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    /// The action key as written.
    pub raw_key: Spanned<String>,
    /// Normalized module reference.
    pub module: ModuleRef,
    /// Free-form value (`shell: echo hi`).
    pub free_form: Option<Spanned<String>>,
    /// Mapping-form arguments.
    pub args: Option<Node>,
}

/// A task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    /// Span of the whole task mapping.
    pub span: Span,
    /// `name`, when present.
    pub name: Option<Spanned<String>>,
    /// Normalized action.
    pub action: Action,
    /// Every non-action key.
    pub keys: KeyMap,
    /// Inherited context.
    pub ctx: TaskCtx,
}

impl Task {
    /// Returns `true` when this is an include/import action.
    #[must_use]
    pub fn is_include(&self) -> bool {
        crate::keywords::is_include_action(&self.action.module.requested)
    }
}

/// A block with optional rescue and always sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// Span of the block mapping.
    pub span: Span,
    /// Main task list.
    pub block: Vec<TaskItem>,
    /// Rescue task list.
    pub rescue: Vec<TaskItem>,
    /// Always task list.
    pub always: Vec<TaskItem>,
    /// Non-block keys.
    pub keys: KeyMap,
}

/// An item in a task list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskItem {
    /// A regular task.
    Task(Task),
    /// A block.
    Block(Block),
    /// An include/import action.
    IncludeImport(Task),
}

impl TaskItem {
    /// Returns the task for task and include items.
    #[must_use]
    pub fn as_task(&self) -> Option<&Task> {
        match self {
            Self::Task(task) | Self::IncludeImport(task) => Some(task),
            Self::Block(_) => None,
        }
    }

    /// Returns the span of the item.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Task(task) | Self::IncludeImport(task) => task.span,
            Self::Block(block) => block.span,
        }
    }
}

/// A role reference inside a play.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleRef {
    /// Span of the role entry.
    pub span: Span,
    /// Role name, when written as a scalar or with a `role` key.
    pub name: Option<Spanned<String>>,
    /// Role parameters.
    pub keys: KeyMap,
}

/// A play.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Play {
    /// Span of the play mapping.
    pub span: Span,
    /// Every play key.
    pub keys: KeyMap,
    /// `pre_tasks`.
    pub pre_tasks: Vec<TaskItem>,
    /// `tasks`.
    pub tasks: Vec<TaskItem>,
    /// `post_tasks`.
    pub post_tasks: Vec<TaskItem>,
    /// `handlers`.
    pub handlers: Vec<TaskItem>,
    /// `roles`.
    pub roles: Vec<RoleRef>,
    /// `vars`, when present.
    pub vars: Option<Node>,
}

/// The model for one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileModel {
    /// A playbook.
    Playbook(Vec<Play>),
    /// A task file.
    Tasks(Vec<TaskItem>),
    /// A handler file.
    Handlers(Vec<TaskItem>),
    /// A file kind without a specialized model yet.
    Other,
    /// The kind could not be determined.
    Unknown,
}

impl FileModel {
    /// Returns plays for playbook models.
    #[must_use]
    pub fn plays(&self) -> &[Play] {
        match self {
            Self::Playbook(plays) => plays,
            _ => &[],
        }
    }

    /// Returns task items for task and handler models.
    #[must_use]
    pub fn task_items(&self) -> &[TaskItem] {
        match self {
            Self::Tasks(items) | Self::Handlers(items) => items,
            _ => &[],
        }
    }
}

/// A tolerated model-construction problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelIssue {
    /// Location of the malformed construct.
    pub span: Span,
    /// Explanation.
    pub message: String,
}

/// The model plus any issues found while building it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltModel {
    /// The model, always present even when malformed.
    pub model: FileModel,
    /// Tolerated problems.
    pub issues: Vec<ModelIssue>,
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::assert_is_empty,
        reason = "empty collections are the subject of these assertions"
    )]

    use ansilint_core::Span;
    use ansilint_yaml::Node;

    use super::{
        Action, FileModel, KeyMap, KeyValue, ModuleRef, Play, Spanned, Task, TaskCtx, TaskItem,
    };

    fn keys() -> KeyMap {
        KeyMap {
            entries: vec![KeyValue {
                key: Spanned::new("name".to_string(), Span::new(0, 4)),
                value: Node::Scalar(ansilint_yaml::Scalar {
                    value: "hello".to_string(),
                    style: ansilint_yaml::ScalarStyle::Plain,
                    span: Span::new(6, 11),
                    anchor: None,
                }),
            }],
        }
    }

    fn task(module: &str) -> Task {
        Task {
            span: Span::new(0, 20),
            name: None,
            action: Action {
                raw_key: Spanned::new(module.to_string(), Span::new(12, 18)),
                module: ModuleRef {
                    requested: module.to_string(),
                    resolved_fqcn: None,
                    is_legacy_action_form: false,
                },
                free_form: None,
                args: None,
            },
            keys: keys(),
            ctx: TaskCtx::default(),
        }
    }

    #[test]
    fn key_maps_expose_values_and_spans() {
        let map = keys();
        assert_eq!(map.get("name").and_then(Node::as_str), Some("hello"));
        assert_eq!(map.key_span("name"), Some(Span::new(0, 4)));
        assert!(map.get("missing").is_none());
    }

    #[test]
    fn includes_and_items_are_classified() {
        let include = task("include_tasks");
        assert!(include.is_include());
        let regular = task("ansible.builtin.copy");
        assert!(!regular.is_include());
        let item = TaskItem::IncludeImport(include);
        assert!(item.as_task().is_some());
        assert_eq!(item.span(), Span::new(0, 20));
    }

    #[test]
    fn file_models_expose_plays_and_items() {
        let play = Play {
            span: Span::new(0, 1),
            keys: KeyMap::default(),
            pre_tasks: Vec::new(),
            tasks: Vec::new(),
            post_tasks: Vec::new(),
            handlers: Vec::new(),
            roles: Vec::new(),
            vars: None,
        };
        assert_eq!(FileModel::Playbook(vec![play]).plays().len(), 1);
        assert!(FileModel::Other.plays().is_empty());
        assert_eq!(
            FileModel::Tasks(vec![TaskItem::Task(task("copy"))])
                .task_items()
                .len(),
            1
        );
    }
}
