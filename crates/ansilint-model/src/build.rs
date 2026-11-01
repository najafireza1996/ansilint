//! Tolerant construction of the model from YAML documents.

use std::collections::HashMap;

use ansilint_core::Span;
use ansilint_yaml::{Alias, Document, Mapping, Node, MAX_NESTING_DEPTH};

use crate::model::{
    Block, BuiltModel, FileModel, KeyMap, KeyValue, ModelIssue, Play, RoleRef, Spanned, Task,
    TaskCtx, TaskItem,
};
use crate::FileKind;

/// Builds a model. Malformed constructs become [`ModelIssue`]s, never errors.
#[must_use]
pub fn build(documents: &[Document], kind: FileKind) -> BuiltModel<'_> {
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
struct Builder<'a> {
    issues: Vec<ModelIssue>,
    /// Root of the document being built; aliases resolve within it.
    root: Option<&'a Node>,
    /// Anchor index of `root`, built on the first alias only.
    anchors: Option<HashMap<usize, &'a Node>>,
}

impl<'a> Builder<'a> {
    fn enter(&mut self, document: &'a Document) {
        self.root = Some(&document.root);
        self.anchors = None;
    }

    fn resolve(&mut self, alias: &Alias) -> Option<&'a Node> {
        let root = self.root?;
        self.anchors
            .get_or_insert_with(|| root.anchors())
            .get(&alias.anchor)
            .copied()
    }

    fn issue(&mut self, span: Span, message: impl Into<String>) {
        self.issues.push(ModelIssue {
            span,
            message: message.into(),
        });
    }

    fn playbook(&mut self, documents: &'a [Document]) -> FileModel<'a> {
        let mut plays = Vec::new();
        let mut play_index = 0_usize;
        for document in documents {
            if document.root.is_null() {
                continue;
            }
            self.enter(document);
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

    fn task_file(&mut self, documents: &'a [Document], in_handler: bool) -> Vec<TaskItem<'a>> {
        let ctx = TaskCtx {
            in_handler,
            ..TaskCtx::default()
        };
        let mut items = Vec::new();
        for document in documents {
            if document.root.is_null() {
                continue;
            }
            self.enter(document);
            items.extend(self.task_list(Some(&document.root), &ctx));
        }
        items
    }

    fn play(&mut self, node: &'a Node, index: usize) -> Option<Play<'a>> {
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
            vars: mapping.get("vars"),
        })
    }

    fn roles(&mut self, node: Option<&'a Node>) -> Vec<RoleRef<'a>> {
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

    fn role(&mut self, node: &'a Node) -> Option<RoleRef<'a>> {
        if let Some(name) = node.as_str() {
            return Some(RoleRef {
                span: node.span(),
                name: Some(Spanned::new(name, node.span())),
                keys: KeyMap::default(),
            });
        }
        let Some(mapping) = node.as_mapping() else {
            self.issue(node.span(), "a role entry must be a string or mapping");
            return None;
        };
        let keys = self.key_map(mapping, &[]);
        let name = keys
            .get("role")
            .and_then(Node::as_str)
            .map(|name| Spanned::new(name, keys.key_span("role").unwrap_or(node.span())));
        Some(RoleRef {
            span: node.span(),
            name,
            keys,
        })
    }

    fn task_list(&mut self, node: Option<&'a Node>, ctx: &TaskCtx) -> Vec<TaskItem<'a>> {
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

    fn task_item(&mut self, node: &'a Node, ctx: &TaskCtx) -> Option<TaskItem<'a>> {
        if let Some(alias) = node.as_alias() {
            return self.aliased_task(node.span(), alias, ctx);
        }
        let Some(mapping) = node.as_mapping() else {
            self.issue(node.span(), "a task list item must be a mapping");
            return None;
        };
        if mapping.get("block").is_some() {
            return Some(TaskItem::Block(self.block(node, mapping, ctx)));
        }
        let task = self.task(node.span(), mapping, ctx)?;
        Some(Self::wrap_task(task))
    }

    /// A task written as `- *anchor`; the span stays on the alias.
    fn aliased_task(&mut self, span: Span, alias: &Alias, ctx: &TaskCtx) -> Option<TaskItem<'a>> {
        let mapping = self.resolve(alias)?.as_mapping()?;
        // Aliased blocks are not expanded: a block can alias itself or fan out
        // through block/rescue/always, which would grow exponentially.
        if mapping.get("block").is_some() {
            return None;
        }
        let task = self.task(span, mapping, ctx)?;
        Some(Self::wrap_task(task))
    }

    fn wrap_task(task: Task<'a>) -> TaskItem<'a> {
        if task.is_include() {
            TaskItem::IncludeImport(task)
        } else {
            TaskItem::Task(task)
        }
    }

    fn block(&mut self, node: &'a Node, mapping: &'a Mapping, ctx: &TaskCtx) -> Block<'a> {
        // Unreachable for parsed input (each block level is two YAML levels),
        // kept so the walk stays bounded for any `Node` tree.
        if usize::try_from(ctx.block_depth).unwrap_or(usize::MAX) >= MAX_NESTING_DEPTH {
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

    fn task(&mut self, span: Span, mapping: &'a Mapping, ctx: &TaskCtx) -> Option<Task<'a>> {
        let (own_index, entry) = match self.action_index(mapping) {
            Some(index) => (Some(index), mapping.entries.get(index)?),
            None => (None, self.merged_action(mapping)?),
        };
        let action = self.action(entry)?;
        let keys = self.key_map_excluding(mapping, own_index);
        let name = keys
            .get("name")
            .and_then(Node::as_str)
            .map(|name| Spanned::new(name, keys.key_span("name").unwrap_or(span)));
        Some(Task {
            span,
            name,
            action,
            keys,
            ctx: ctx.clone(),
        })
    }

    fn key_map(&mut self, mapping: &'a Mapping, excluded: &[&str]) -> KeyMap<'a> {
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
                key: Spanned::new(key, entry.key_span),
                value: &entry.value,
            });
        }
        KeyMap { entries }
    }

    fn key_map_excluding(&mut self, mapping: &'a Mapping, index: Option<usize>) -> KeyMap<'a> {
        let mut entries = Vec::new();
        for (current, entry) in mapping.entries.iter().enumerate() {
            if Some(current) == index {
                continue;
            }
            let Some(key) = entry.key.as_str() else {
                self.issue(entry.key_span, "mapping keys must be strings");
                continue;
            };
            entries.push(KeyValue {
                key: Spanned::new(key, entry.key_span),
                value: &entry.value,
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

mod action;

#[cfg(test)]
mod tests;
