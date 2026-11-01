use ansilint_model::FileKind;

use ansilint_config::Config;

use super::{
    compile_glob, default_kinds, find_git_root, has_yaml_extension, relative_output_path,
    Classifier, ExcludeSet,
};

/// H-E2: (path relative to the project root, upstream kind name), following
/// upstream `DEFAULT_KINDS` order at ansible-lint 67a4871; first match wins.
const UPSTREAM_KIND_SAMPLES: &[(&str, &str)] = &[
    ("templates/motd.j2", "jinja2"),
    ("roles/x/templates/conf.yml.j2", "jinja2"),
    ("files/a.j2.bak", "jinja2"),
    (".github/workflows/ci.yml", "yaml"),
    ("roles/x/templates/config.yml", "text"),
    ("execution-environment.yml", "execution-environment"),
    (".ansible-lint", "ansible-lint-config"),
    (".ansible-lint.yml", "ansible-lint-config"),
    (".config/ansible-lint.yaml", "ansible-lint-config"),
    ("ansible-navigator.yml", "ansible-navigator-config"),
    ("inventory/hosts.yml", "inventory"),
    ("roles/x/meta/requirements.yml", "requirements"),
    ("galaxy.yml", "galaxy"),
    ("releasenotes/notes/fix.yaml", "reno"),
    ("group_vars/all.yml", "vars"),
    ("host_vars/web1/main.yml", "vars"),
    ("roles/x/vars/main.yml", "vars"),
    ("roles/x/defaults/main.yml", "vars"),
    ("roles/x/tasks/main.yml", "tasks"),
    ("roles/x/tasks/sub/more.yml", "tasks"),
    ("site.meta.yml", "play-argspec"),
    ("playbooks/site.yml", "playbook"),
    ("deploy-playbook.yml", "playbook"),
    ("roles/x/handlers/main.yml", "handlers"),
    ("roles/x/handlers/sub/main.yml", "yaml"),
    ("tests/integration/targets/t/meta/main.yml", "test-meta"),
    ("roles/x/meta/main.yml", "meta"),
    ("meta/runtime.yml", "meta-runtime"),
    ("roles/x/meta/argument_specs.yml", "role-arg-spec"),
    (".config/molecule/config.yml", "yaml"),
    ("molecule/default/requirements.yml", "requirements"),
    ("molecule/default/molecule.yml", "yaml"),
    ("requirements.yml", "requirements"),
    ("molecule/default/converge.yml", "playbook"),
    (".yamllint", "yaml"),
    ("changelogs/changelog.yaml", "changelog"),
    ("docker-compose.yml", "yaml"),
    (".pre-commit-config.yaml", "yaml"),
    ("tests/sanity/ignore-2.15.txt", "sanity-ignore-file"),
    ("plugins/modules/foo.py", "plugin"),
    ("setup.py", "python"),
    ("playbooks/sub/site.yml", "yaml"),
    ("rulebooks/r.yml", "yaml"),
];

#[test]
fn default_kinds_file_is_pinned_and_complete() {
    assert!(super::DEFAULT_KINDS.contains("67a48717843c4d5d37681294cb5c9967781b0a31"));
    assert_eq!(default_kinds().count(), 39);
    let unusable: Vec<(&str, &str)> = default_kinds()
        .filter(|(name, pattern)| {
            FileKind::from_name(name).is_none() || compile_glob(pattern).is_none()
        })
        .collect();
    assert_eq!(unusable, [("rulebook", "**/rulebooks/*.{yml,yaml")]);
}

#[test]
fn default_kinds_match_upstream_order_for_sample_paths() {
    let classifier = Classifier::new(&[]).0;
    let root = std::path::Path::new("/project");
    let mismatches: Vec<String> = UPSTREAM_KIND_SAMPLES
        .iter()
        .filter_map(|(path, expected)| {
            let actual = classifier.classify(root, &root.join(path)).as_str();
            (actual != *expected).then(|| format!("{path}: expected {expected}, got {actual}"))
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
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
    let classifier = Classifier::new(&[]).0;
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

/// H-E6: each of the 13 kinds added in H-E2, with a sample path; the two that
/// can never be a file's kind are listed with the reason.
const PREVIOUSLY_MISSING_KINDS: &[(FileKind, Option<&str>)] = &[
    (FileKind::Jinja2, Some("roles/web/templates/nginx.conf.j2")),
    (FileKind::Text, Some("roles/web/templates/index.html")),
    (
        FileKind::AnsibleLintConfig,
        Some(".config/ansible-lint.yml"),
    ),
    (
        FileKind::AnsibleNavigatorConfig,
        Some("ansible-navigator.yaml"),
    ),
    (FileKind::Reno, Some("releasenotes/notes/feature-x.yaml")),
    // Upstream's glob `**/rulebooks/*.{yml,yaml` has an unclosed brace and
    // matches nothing, upstream included.
    (FileKind::Rulebook, None),
    (
        FileKind::Pattern,
        Some("extensions/patterns/web/meta/pattern.json"),
    ),
    (FileKind::PlayArgspec, Some("playbooks/site.meta.yml")),
    // `**/roles/*/` matches role directories; only files are classified.
    (FileKind::Role, None),
    (
        FileKind::TestMeta,
        Some("tests/integration/targets/web/meta/main.yml"),
    ),
    (
        FileKind::SanityIgnoreFile,
        Some("tests/sanity/ignore-2.17.txt"),
    ),
    (FileKind::Plugin, Some("plugins/filter/text.py")),
    (FileKind::Python, Some("docs/conf.py")),
];

#[test]
fn every_previously_missing_kind_is_detected_or_documented() {
    let classifier = Classifier::new(&[]).0;
    let root = std::path::Path::new("/project");
    let wrong: Vec<String> = PREVIOUSLY_MISSING_KINDS
        .iter()
        .filter_map(|(kind, path)| path.map(|path| (kind, path)))
        .filter_map(|(kind, path)| {
            let actual = classifier.classify(root, &root.join(path));
            (actual != *kind).then(|| format!("{path}: expected {kind:?}, got {actual:?}"))
        })
        .collect();
    assert_eq!(PREVIOUSLY_MISSING_KINDS.len(), 13);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let role_dir = classifier.classify(root, &root.join("roles/web"));
    assert_ne!(
        role_dir,
        FileKind::Role,
        "a path without a trailing slash is a file"
    );
}
