//! Action-key discovery: own keys, merge keys and the legacy `action:` forms.

use ansilint_yaml::{Mapping, MappingEntry, Node};

use super::Builder;
use crate::keywords::is_task_keyword;
use crate::model::{Action, ModuleRef, Spanned};

impl<'a> Builder<'a> {
    /// The action of a task that only inherits it through `<<: *anchor` or
    /// `<<: [*a, *b]`. Own keys take precedence, so this runs only when the
    /// task has no action of its own. One level deep: merges are not chased.
    pub(super) fn merged_action(&mut self, mapping: &Mapping) -> Option<&'a MappingEntry> {
        let mut sources = Vec::new();
        for entry in mapping.get_all("<<") {
            match &entry.value {
                Node::Alias(alias) => sources.push(alias.clone()),
                Node::Sequence(sequence) => {
                    sources.extend(sequence.items.iter().filter_map(Node::as_alias).cloned());
                }
                _ => {}
            }
        }
        let found = sources.iter().find_map(|alias| {
            let merged = self.resolve(alias)?.as_mapping()?;
            merged.entries.iter().find(|entry| {
                entry
                    .key
                    .as_str()
                    .is_some_and(|key| key != "<<" && !is_task_keyword(key))
            })
        });
        if found.is_none() {
            self.issue(mapping.span, "a task has no action key");
        }
        found
    }

    pub(super) fn action_index(&mut self, mapping: &Mapping) -> Option<usize> {
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

    pub(super) fn action(&mut self, entry: &MappingEntry) -> Option<Action> {
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
}

pub(super) fn split_legacy_action(value: &str) -> (String, Option<String>) {
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
