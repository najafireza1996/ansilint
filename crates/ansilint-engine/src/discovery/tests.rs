use ansilint_model::FileKind;

use ansilint_config::Config;

use super::{
    default_patterns, find_git_root, has_yaml_extension, relative_output_path, Classifier,
    ExcludeSet,
};

#[test]
fn defaults_are_ordered_from_specific_to_generic() {
    let patterns = default_patterns();
    assert_eq!(patterns.last().map(|(kind, _)| *kind), Some(FileKind::Yaml));
    assert!(patterns.len() > 5);
}

#[test]
fn classifies_role_files() {
    let classifier = Classifier::new(&[]);
    let root = std::path::Path::new("/project");
    assert_eq!(
        classifier.classify(root, &root.join("roles/x/tasks/main.yml")),
        FileKind::Tasks
    );
    assert_eq!(
        classifier.classify(root, &root.join("roles/x/meta/main.yml")),
        FileKind::Meta
    );
    assert_eq!(
        classifier.classify(root, &root.join("playbooks/site.yml")),
        FileKind::Yaml
    );
}

#[test]
fn finds_no_git_root_in_a_temp_directory() {
    let directory = tempfile::tempdir().expect("temp dir");
    assert!(find_git_root(directory.path()).is_none());
}

#[test]
fn yaml_extensions_are_case_insensitive() {
    assert!(has_yaml_extension(std::path::Path::new("a.yml")));
    assert!(has_yaml_extension(std::path::Path::new("a.YML")));
    assert!(has_yaml_extension(std::path::Path::new("a.Yaml")));
    assert!(!has_yaml_extension(std::path::Path::new("a.txt")));
}

#[test]
fn directory_patterns_are_case_sensitive() {
    let classifier = Classifier::new(&[]);
    let root = std::path::Path::new("/project");
    assert_eq!(
        classifier.classify(root, &root.join("roles/x/Tasks/main.yml")),
        FileKind::Yaml
    );
}

fn sniffed(text: &str) -> FileKind {
    let source = ansilint_yaml::SourceFile::new("x.yml", text);
    let documents = ansilint_yaml::parse(&source).expect("valid YAML");
    super::sniff(FileKind::Yaml, &documents)
}

#[test]
fn sniffs_playbooks_from_any_item_and_document() {
    assert_eq!(
        sniffed("- import_playbook: a.yml\n- hosts: all\n"),
        FileKind::Playbook
    );
    assert_eq!(
        sniffed("- import_playbook: a.yml\n- ansible.builtin.import_playbook: b.yml\n"),
        FileKind::Playbook
    );
    assert_eq!(
        sniffed("---\nkey: 1\n---\n- hosts: all\n"),
        FileKind::Playbook
    );
}

#[test]
fn never_sniffs_generic_yaml_as_ansible() {
    assert_eq!(sniffed("- name: a\n  image: nginx\n"), FileKind::Yaml);
    assert_eq!(
        sniffed("services:\n  web:\n    image: nginx\n"),
        FileKind::Yaml
    );
    assert_eq!(sniffed("- name: x\n  shell: echo\n"), FileKind::Yaml);
    assert_eq!(sniffed(""), FileKind::Yaml);
}

#[test]
fn cache_is_implicit_only_without_configured_exclude_paths() {
    let base = std::path::Path::new("/project");
    let cache = base.join(".cache/x.yml");
    let implicit = ExcludeSet::new(&Config::default(), base, &[], base);
    assert!(implicit.is_excluded(&cache));
    let configured = Config {
        exclude_paths: vec!["vendor".into()],
        ..Config::default()
    };
    let explicit = ExcludeSet::new(&configured, base, &[], base);
    assert!(!explicit.is_excluded(&cache));
    assert!(explicit.is_excluded(&base.join("vendor/y.yml")));
}

#[test]
fn normalize_resolves_dots_lexically() {
    let root = std::env::temp_dir();
    assert_eq!(
        super::normalize(&root, std::path::Path::new("a/./b/../c")),
        root.join("a").join("c")
    );
    assert_eq!(
        super::normalize(&root.join("work"), std::path::Path::new("../x.yml")),
        root.join("x.yml")
    );
}

#[test]
fn paths_outside_the_root_get_parent_segments() {
    let base = std::env::temp_dir();
    assert_eq!(
        relative_output_path(&base.join("project"), &base.join("other").join("x.yml")),
        std::path::PathBuf::from("../other/x.yml")
    );
    assert_eq!(
        relative_output_path(&base.join("a").join("b"), &base.join("x.yml")),
        std::path::PathBuf::from("../../x.yml")
    );
}

#[test]
fn output_paths_use_forward_slashes() {
    let root = std::path::Path::new("/project");
    let path = std::path::Path::new("/project/roles\\x\\tasks\\main.yml");
    assert_eq!(
        relative_output_path(root, path),
        std::path::PathBuf::from("roles/x/tasks/main.yml")
    );
}
