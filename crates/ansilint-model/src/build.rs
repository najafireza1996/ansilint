//! Tolerant construction of the model from YAML documents.

use ansilint_core::Span;
use ansilint_yaml::{Document, Mapping, MappingEntry, Node};

use crate::keywords::is_task_keyword;
use crate::model::{
    Action, Block, BuiltModel, FileModel, KeyMap, KeyValue, ModelIssue, ModuleRef, Play, RoleRef,
    Spanned, Task, TaskCtx, TaskItem,
};
use crate::FileKind;

const MAX_BLOCK_DEPTH: u32 = 128;

/// Builds a model. Malformed constructs become [`ModelIssue`]s, never errors.
#[must_use]
pub fn build(documents: &[Document], kind: FileKind) -> BuiltModel {
    let mut builder = Builder::default();
    let model = match kind {
        FileKind::Playbook => builder.playbook(documents),
        FileKind::Tasks => FileModel::Tasks(builder.task_file(documents, false)),
        FileKind::Handlers => FileModel::Handlers(builder.task_file(documents, true)),
        _ => FileModel::Other,
    };
    BuiltModel {
        model,
        issues: builder.issues,
    }
}

#[derive(Default)]
struct Builder {
    issues: Vec<ModelIssue>,
}

impl Builder {
    fn issue(&mut self, span: Span, message: impl Into<String>) {
        self.issues.push(ModelIssue {
            span,
            message: message.into(),
        });
    }

    fn playbook(&mut self, documents: &[Document]) -> FileModel {
        let mut plays = Vec::new();
        let mut play_index = 0_usize;
        for document in documents {
            if document.root.is_null() {
                continue;
            }
            let Some(sequence) = document.root.as_sequence() else {
                self.issue(
                    document.root.span(),
                    "a playbook must be a sequence of plays",
                );
                continue;
            };
            for node in &sequence.items {
                if let Some(play) = self.play(node, play_index) {
                    plays.push(play);
                    play_index = play_index.saturating_add(1);
                }
            }
        }
        FileModel::Playbook(plays)
    }

    fn task_file(&mut self, documents: &[Document], in_handler: bool) -> Vec<TaskItem> {
        let ctx = TaskCtx {
            in_handler,
            ..TaskCtx::default()
        };
        let mut items = Vec::new();
        for document in documents {
            if document.root.is_null() {
                continue;
            }
            items.extend(self.task_list(Some(&document.root), &ctx));
        }
        items
    }

    fn play(&mut self, node: &Node, index: usize) -> Option<Play> {
        let Some(mapping) = node.as_mapping() else {
            self.issue(node.span(), "a play must be a mapping");
            return None;
        };
        let keys = self.key_map(mapping, &[]);
        let ctx = TaskCtx {
            play_index: Some(index),
            ..TaskCtx::default()
        };
        let pre_tasks = self.task_list(mapping.get("pre_tasks"), &ctx);
        let tasks = self.task_list(mapping.get("tasks"), &ctx);
        let post_tasks = self.task_list(mapping.get("post_tasks"), &ctx);
        let handlers = self.task_list(mapping.get("handlers"), &handler_ctx(&ctx));
        let roles = self.roles(mapping.get("roles"));
        Some(Play {
            span: node.span(),
            keys,
            pre_tasks,
            tasks,
            post_tasks,
            handlers,
            roles,
            vars: mapping.get("vars").cloned(),
        })
    }

    fn roles(&mut self, node: Option<&Node>) -> Vec<RoleRef> {
        let Some(node) = node else {
            return Vec::new();
        };
        let Some(sequence) = node.as_sequence() else {
            self.issue(node.span(), "`roles` must be a sequence");
            return Vec::new();
        };
        let mut roles = Vec::new();
        for item in &sequence.items {
            if let Some(role) = self.role(item) {
                roles.push(role);
            }
        }
        roles
    }

    fn role(&mut self, node: &Node) -> Option<RoleRef> {
        if let Some(name) = node.as_str() {
            return Some(RoleRef {
                span: node.span(),
                name: Some(Spanned::new(name.to_string(), node.span())),
                keys: KeyMap::default(),
            });
        }
        let Some(mapping) = node.as_mapping() else {
            self.issue(node.span(), "a role entry must be a string or mapping");
            return None;
        };
        let keys = self.key_map(mapping, &[]);
        let name = keys.get("role").and_then(Node::as_str).map(|name| {
            Spanned::new(
                name.to_string(),
                keys.key_span("role").unwrap_or(node.span()),
            )
        });
        Some(RoleRef {
            span: node.span(),
            name,
            keys,
        })
    }

    fn task_list(&mut self, node: Option<&Node>, ctx: &TaskCtx) -> Vec<TaskItem> {
        let Some(node) = node else {
            return Vec::new();
        };
        let Some(sequence) = node.as_sequence() else {
            self.issue(node.span(), "a task list must be a sequence");
            return Vec::new();
        };
        let mut items = Vec::new();
        for item in &sequence.items {
            if let Some(item) = self.task_item(item, ctx) {
                items.push(item);
            }
        }
        items
    }

    fn task_item(&mut self, node: &Node, ctx: &TaskCtx) -> Option<TaskItem> {
        if node.as_alias().is_some() {
            return None;
        }
        let Some(mapping) = node.as_mapping() else {
            self.issue(node.span(), "a task list item must be a mapping");
            return None;
        };
        if mapping.get("block").is_some() {
            return Some(TaskItem::Block(self.block(node, mapping, ctx)));
        }
        let task = self.task(node, mapping, ctx)?;
        if task.is_include() {
            Some(TaskItem::IncludeImport(task))
        } else {
            Some(TaskItem::Task(task))
        }
    }

    fn block(&mut self, node: &Node, mapping: &Mapping, ctx: &TaskCtx) -> Block {
        if ctx.block_depth >= MAX_BLOCK_DEPTH {
            self.issue(node.span(), "block nesting exceeds the supported depth");
            return Block {
                span: node.span(),
                block: Vec::new(),
                rescue: Vec::new(),
                always: Vec::new(),
                keys: KeyMap::default(),
            };
        }
        let keys = self.key_map(mapping, &["block", "rescue", "always"]);
        let mut child = ctx.clone();
        child.block_depth = child.block_depth.saturating_add(1);
        let block = self.task_list(mapping.get("block"), &child);
        let rescue = self.task_list(mapping.get("rescue"), &child);
        let always = self.task_list(mapping.get("always"), &child);
        Block {
            span: node.span(),
            block,
            rescue,
            always,
            keys,
        }
    }

    fn task(&mut self, node: &Node, mapping: &Mapping, ctx: &TaskCtx) -> Option<Task> {
        let action_index = self.action_index(mapping);
        let Some(action_index) = action_index else {
            self.issue(node.span(), "a task has no action key");
            return None;
        };
        let entry = mapping.entries.get(action_index)?;
        let action = self.action(entry)?;
        let keys = self.key_map_excluding(mapping, action_index);
        let name = keys.get("name").and_then(Node::as_str).map(|name| {
            Spanned::new(
                name.to_string(),
                keys.key_span("name").unwrap_or(node.span()),
            )
        });
        Some(Task {
            span: node.span(),
            name,
            action,
            keys,
            ctx: ctx.clone(),
        })
    }

    fn action_index(&mut self, mapping: &Mapping) -> Option<usize> {
        let mut found = None;
        let mut count = 0_usize;
        for (index, entry) in mapping.entries.iter().enumerate() {
            let Some(key) = entry.key.as_str() else {
                continue;
            };
            if key == "<<" || is_task_keyword(key) {
                continue;
            }
            count = count.saturating_add(1);
            if found.is_none() {
                found = Some(index);
            }
        }
        if count > 1 {
            self.issue(mapping.span, "a task has more than one action key");
        }
        found
    }

    fn action(&mut self, entry: &MappingEntry) -> Option<Action> {
        let key = entry.key.as_str()?;
        let key_span = entry.key.span();
        let legacy = matches!(key, "action" | "local_action");
        let (requested, free_form, args) = if legacy {
            self.legacy_action(&entry.value)?
        } else {
            let (_ignored, free_form, args) = Self::plain_action(&entry.value);
            (key.to_string(), free_form, args)
        };
        Some(Action {
            raw_key: Spanned::new(key.to_string(), key_span),
            module: ModuleRef {
                requested,
                resolved_fqcn: None,
                is_legacy_action_form: legacy,
            },
            free_form,
            args,
        })
    }

    fn plain_action(value: &Node) -> (String, Option<Spanned<String>>, Option<Node>) {
        match value {
            Node::Scalar(scalar) => (
                String::new(),
                Some(Spanned::new(scalar.value.clone(), scalar.span)),
                None,
            ),
            Node::Mapping(_) | Node::Sequence(_) | Node::Alias(_) => {
                (String::new(), None, Some(value.clone()))
            }
            Node::Null { .. } => (String::new(), None, None),
        }
    }

    fn legacy_action(
        &mut self,
        value: &Node,
    ) -> Option<(String, Option<Spanned<String>>, Option<Node>)> {
        match value {
            Node::Scalar(scalar) => {
                let (module, rest) = split_legacy_action(&scalar.value);
                let free_form = rest.map(|rest| Spanned::new(rest, scalar.span));
                Some((module, free_form, None))
            }
            Node::Mapping(mapping) => {
                let module = mapping
                    .get("module")
                    .and_then(Node::as_str)
                    .unwrap_or_default()
                    .to_string();
                Some((module, None, Some(value.clone())))
            }
            _ => {
                self.issue(value.span(), "legacy action needs a module name");
                None
            }
        }
    }

    fn key_map(&mut self, mapping: &Mapping, excluded: &[&str]) -> KeyMap {
        let mut entries = Vec::new();
        for entry in &mapping.entries {
            let Some(key) = entry.key.as_str() else {
                self.issue(entry.key_span, "mapping keys must be strings");
                continue;
            };
            if excluded.contains(&key) {
                continue;
            }
            entries.push(KeyValue {
                key: Spanned::new(key.to_string(), entry.key_span),
                value: entry.value.clone(),
            });
        }
        KeyMap { entries }
    }

    fn key_map_excluding(&mut self, mapping: &Mapping, index: usize) -> KeyMap {
        let mut entries = Vec::new();
        for (current, entry) in mapping.entries.iter().enumerate() {
            if current == index {
                continue;
            }
            let Some(key) = entry.key.as_str() else {
                self.issue(entry.key_span, "mapping keys must be strings");
                continue;
            };
            entries.push(KeyValue {
                key: Spanned::new(key.to_string(), entry.key_span),
                value: entry.value.clone(),
            });
        }
        KeyMap { entries }
    }
}

fn handler_ctx(ctx: &TaskCtx) -> TaskCtx {
    let mut handler = ctx.clone();
    handler.in_handler = true;
    handler
}

fn split_legacy_action(value: &str) -> (String, Option<String>) {
    let trimmed = value.trim();
    let split = trimmed
        .char_indices()
        .find(|(_, character)| character.is_whitespace())
        .map(|(index, _)| index);
    let Some(split) = split else {
        return (trimmed.to_string(), None);
    };
    let module = trimmed.get(..split).unwrap_or(trimmed).to_string();
    let rest = trimmed
        .get(split..)
        .map(str::trim)
        .filter(|rest| !rest.is_empty());
    (module, rest.map(str::to_string))
}

#[cfg(test)]
mod tests;
