#![allow(
    clippy::assert_is_empty,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use super::build;
use crate::model::{FileModel, TaskItem};
use crate::FileKind;
use ansilint_yaml::SourceFile;

fn model(text: &str, kind: FileKind) -> crate::model::BuiltModel {
    let source = SourceFile::new("test.yml", text);
    let documents = ansilint_yaml::parse(&source).expect("valid YAML");
    build(&documents, kind)
}

#[test]
fn builds_plays_tasks_blocks_and_includes() {
    let built = model(
        "- hosts: all\n  tasks:\n    - name: copy\n      ansible.builtin.copy:\n        src: a\n        dest: b\n    - name: legacy\n      action: shell echo hi\n    - block:\n        - name: include\n          include_tasks: other.yml\n",
        FileKind::Playbook,
    );
    let FileModel::Playbook(plays) = &built.model else {
        panic!("expected a playbook");
    };
    assert_eq!(plays.len(), 1);
    assert_eq!(plays[0].tasks.len(), 3);
    assert!(matches!(plays[0].tasks[1], TaskItem::Task(_)));
    assert!(matches!(plays[0].tasks[2], TaskItem::Block(_)));
    let TaskItem::Task(first) = &plays[0].tasks[0] else {
        panic!("expected a task");
    };
    assert_eq!(first.action.module.requested, "ansible.builtin.copy");
    assert_eq!(
        first.name.as_ref().map(|name| name.value.as_str()),
        Some("copy")
    );
    let TaskItem::Task(legacy) = &plays[0].tasks[1] else {
        panic!("expected a task");
    };
    assert!(legacy.action.module.is_legacy_action_form);
    assert_eq!(legacy.action.module.requested, "shell");
    assert_eq!(
        legacy
            .action
            .free_form
            .as_ref()
            .map(|value| value.value.as_str()),
        Some("echo hi")
    );
}

#[test]
fn classifies_include_tasks() {
    let built = model(
        "- name: include\n  include_tasks: other.yml\n",
        FileKind::Tasks,
    );
    let FileModel::Tasks(items) = &built.model else {
        panic!("expected tasks");
    };
    assert!(matches!(items[0], TaskItem::IncludeImport(_)));
}

#[test]
fn records_blocks_and_handler_context() {
    let built = model(
        "- hosts: all\n  handlers:\n    - name: restart\n      ansible.builtin.service:\n        name: x\n        state: restarted\n",
        FileKind::Playbook,
    );
    let FileModel::Playbook(plays) = &built.model else {
        panic!("expected a playbook");
    };
    let TaskItem::Task(task) = &plays[0].handlers[0] else {
        panic!("expected a handler");
    };
    assert!(task.ctx.in_handler);
}

#[test]
fn reports_tasks_with_two_actions() {
    let built = model(
        "- name: bad\n  ansible.builtin.copy: {src: a, dest: b}\n  ansible.builtin.file: {path: c}\n",
        FileKind::Tasks,
    );
    assert!(built
        .issues
        .iter()
        .any(|issue| issue.message.contains("more than one")));
}

#[test]
fn empty_input_is_tolerated() {
    let built = model("", FileKind::Playbook);
    assert!(built.model.plays().is_empty());
}

#[test]
fn parses_roles_and_legacy_action_mappings() {
    let built = model(
        "- hosts: all\n  roles:\n    - common\n    - role: web\n      become: true\n  tasks:\n    - name: legacy\n      action:\n        module: copy\n        args: {src: a, dest: b}\n",
        FileKind::Playbook,
    );
    let FileModel::Playbook(plays) = &built.model else {
        panic!("expected a playbook");
    };
    assert_eq!(plays[0].roles.len(), 2);
    assert_eq!(
        plays[0].roles[0]
            .name
            .as_ref()
            .map(|name| name.value.as_str()),
        Some("common")
    );
    let TaskItem::Task(task) = &plays[0].tasks[0] else {
        panic!("expected a task");
    };
    assert!(task.action.module.is_legacy_action_form);
    assert_eq!(task.action.module.requested, "copy");
    assert!(task.action.args.is_some());
}

#[test]
fn splits_legacy_action_strings() {
    assert_eq!(
        super::split_legacy_action("shell echo hi"),
        ("shell".to_string(), Some("echo hi".to_string()))
    );
    assert_eq!(
        super::split_legacy_action("  shell  "),
        ("shell".to_string(), None)
    );
}

#[test]
fn reports_non_sequence_roots() {
    let tasks = model("a: 1\n", FileKind::Tasks);
    assert!(tasks
        .issues
        .iter()
        .any(|issue| issue.message.contains("sequence")));
    let playbook = model("a: 1\n", FileKind::Playbook);
    assert!(playbook
        .issues
        .iter()
        .any(|issue| issue.message.contains("sequence")));
}

#[test]
fn builds_plays_from_every_document() {
    let built = model(
        "---\n- hosts: all\n  tasks:\n    - name: one\n      ansible.builtin.debug: {msg: one}\n---\n- hosts: web\n  tasks:\n    - name: two\n      ansible.builtin.debug: {msg: two}\n",
        FileKind::Playbook,
    );
    let FileModel::Playbook(plays) = &built.model else {
        panic!("expected a playbook");
    };
    assert_eq!(plays.len(), 2);
    assert_eq!(
        plays[1].keys.get("hosts").and_then(|node| node.as_str()),
        Some("web")
    );
}

#[test]
fn does_not_mistake_merge_keys_or_any_errors_fatal_for_actions() {
    let built = model(
        "- &base\n  name: base\n  ansible.builtin.debug: {msg: hi}\n- <<: *base\n  name: child\n  ansible.builtin.debug: {msg: hi}\n- name: fatal\n  any_errors_fatal: true\n  ansible.builtin.debug: {msg: hi}\n",
        FileKind::Tasks,
    );
    assert!(
        !built
            .issues
            .iter()
            .any(|issue| issue.message.contains("more than one action")),
        "unexpected action issues: {:?}",
        built.issues
    );
}
