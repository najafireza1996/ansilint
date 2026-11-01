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
pub struct KeyValue<'a> {
    /// Key and its span, borrowed from the document.
    pub key: Spanned<&'a str>,
    /// Raw YAML value, borrowed from the document (H-C8: never cloned).
    pub value: &'a Node,
}

/// Ordered, duplicate-preserving keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyMap<'a> {
    /// Entries in source order.
    pub entries: Vec<KeyValue<'a>>,
}

impl<'a> KeyMap<'a> {
    /// Returns the first value for a key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&'a Node> {
        self.entries
            .iter()
            .find(|entry| entry.key.value == key)
            .map(|entry| entry.value)
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
pub struct Action<'a> {
    /// The action key as written.
    pub raw_key: Spanned<&'a str>,
    /// Normalized module reference.
    pub module: ModuleRef,
    /// Free-form value (`shell: echo hi`), borrowed from the document.
    pub free_form: Option<Spanned<&'a str>>,
    /// Mapping-form arguments, borrowed from the document.
    pub args: Option<&'a Node>,
}

/// A task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task<'a> {
    /// Span of the whole task mapping.
    pub span: Span,
    /// `name`, when present.
    pub name: Option<Spanned<&'a str>>,
    /// Normalized action.
    pub action: Action<'a>,
    /// Every non-action key.
    pub keys: KeyMap<'a>,
    /// Inherited context.
    pub ctx: TaskCtx,
}

impl Task<'_> {
    /// Returns `true` when this is an include/import action.
    #[must_use]
    pub fn is_include(&self) -> bool {
        crate::keywords::is_include_action(&self.action.module.requested)
    }
}

/// A block with optional rescue and always sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block<'a> {
    /// Span of the block mapping.
    pub span: Span,
    /// Main task list.
    pub block: Vec<TaskItem<'a>>,
    /// Rescue task list.
    pub rescue: Vec<TaskItem<'a>>,
    /// Always task list.
    pub always: Vec<TaskItem<'a>>,
    /// Non-block keys.
    pub keys: KeyMap<'a>,
}

/// An item in a task list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskItem<'a> {
    /// A regular task.
    Task(Task<'a>),
    /// A block.
    Block(Block<'a>),
    /// An include/import action.
    IncludeImport(Task<'a>),
}

impl<'a> TaskItem<'a> {
    /// Returns the task for task and include items.
    #[must_use]
    pub fn as_task(&self) -> Option<&Task<'a>> {
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
pub struct RoleRef<'a> {
    /// Span of the role entry.
    pub span: Span,
    /// Role name, when written as a scalar or with a `role` key.
    pub name: Option<Spanned<&'a str>>,
    /// Role parameters.
    pub keys: KeyMap<'a>,
}

/// A play.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Play<'a> {
    /// Span of the play mapping.
    pub span: Span,
    /// Every play key.
    pub keys: KeyMap<'a>,
    /// `pre_tasks`.
    pub pre_tasks: Vec<TaskItem<'a>>,
    /// `tasks`.
    pub tasks: Vec<TaskItem<'a>>,
    /// `post_tasks`.
    pub post_tasks: Vec<TaskItem<'a>>,
    /// `handlers`.
    pub handlers: Vec<TaskItem<'a>>,
    /// `roles`.
    pub roles: Vec<RoleRef<'a>>,
    /// `vars`, when present.
    pub vars: Option<&'a Node>,
}

/// The model for one file.
///
/// Non-exhaustive (H-D5): later phases add models for more file kinds, so
/// callers outside this crate must keep a wildcard arm:
///
/// ```compile_fail
/// fn count(model: &ansilint_model::FileModel<'_>) -> usize {
///     use ansilint_model::FileModel;
///     match model {
///         FileModel::Playbook(plays) => plays.len(),
///         FileModel::Tasks(items) | FileModel::Handlers(items) => items.len(),
///         FileModel::Other | FileModel::Unknown => 0,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileModel<'a> {
    /// A playbook.
    Playbook(Vec<Play<'a>>),
    /// A task file.
    Tasks(Vec<TaskItem<'a>>),
    /// A handler file.
    Handlers(Vec<TaskItem<'a>>),
    /// A file kind without a specialized model yet.
    Other,
    /// The kind could not be determined.
    Unknown,
}

impl<'a> FileModel<'a> {
    /// Returns plays for playbook models.
    #[must_use]
    pub fn plays(&self) -> &[Play<'a>] {
        match self {
            Self::Playbook(plays) => plays,
            _ => &[],
        }
    }

    /// Returns task items for task and handler models.
    #[must_use]
    pub fn task_items(&self) -> &[TaskItem<'a>] {
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
pub struct BuiltModel<'a> {
    /// The model, always present even when malformed.
    pub model: FileModel<'a>,
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

    fn keys() -> KeyMap<'static> {
        KeyMap {
            entries: vec![KeyValue {
                key: Spanned::new("name", Span::new(0, 4)),
                value: Box::leak(Box::new(Node::Scalar(ansilint_yaml::Scalar {
                    value: "hello".to_string(),
                    style: ansilint_yaml::ScalarStyle::Plain,
                    span: Span::new(6, 11),
                    anchor: None,
                }))),
            }],
        }
    }

    fn task(module: &str) -> Task<'_> {
        Task {
            span: Span::new(0, 20),
            name: None,
            action: Action {
                raw_key: Spanned::new(module, Span::new(12, 18)),
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
