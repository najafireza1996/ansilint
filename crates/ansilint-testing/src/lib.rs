//! Dev-only fixture builders, in-memory fakes and contract-test macros.
//!
//! It must not: be a normal dependency of a shipped crate.
#![forbid(unsafe_code)]

/// Generates the contract test for a rule implementation (ENG-12, H-C15).
///
/// Checks the metadata invariants ([`assert_meta_contract`]) and runs the rule
/// over [`CONTRACT_INPUTS`] as every file kind: it must not panic (no
/// `internal-error`), must be deterministic, and every finding's span must lie
/// inside the file. Expands to code that names `ansilint_yaml`,
/// `ansilint_model`, `ansilint_config` and `ansilint_rules`, so call it from an
/// `ansilint-rules` integration test with the `Rule` trait in scope.
#[macro_export]
macro_rules! rule_contract_tests {
    ($name:ident, $rule:expr) => {
        #[test]
        fn $name() {
            use ansilint_model::FileKind;
            let rule = $rule;
            let meta = rule.meta();
            $crate::assert_meta_contract(meta.id, meta.summary, meta.tags);
            let config = ansilint_config::Config::default();
            for text in $crate::CONTRACT_INPUTS {
                let source = ansilint_yaml::SourceFile::new("contract.yml", *text);
                let Ok(documents) = ansilint_yaml::parse(&source) else {
                    continue;
                };
                for kind in [
                    FileKind::Playbook,
                    FileKind::Tasks,
                    FileKind::Handlers,
                    FileKind::Yaml,
                ] {
                    let built = ansilint_model::build(&documents, kind);
                    let cx = ansilint_rules::FileCx {
                        source: &source,
                        documents: &documents,
                        kind,
                        built: &built,
                        config: &config,
                    };
                    let first = ansilint_rules::run(&[&rule], &cx);
                    let again = ansilint_rules::run(&[&rule], &cx);
                    assert_eq!(first, again, "nondeterministic on {text:?}");
                    for finding in &first {
                        let context = format!("{text:?} as {kind:?}");
                        assert_ne!(finding.tag.rule, "internal-error", "panicked on {context}");
                        $crate::assert_span_inside(
                            finding.span.start,
                            finding.span.end,
                            text,
                            &context,
                        );
                    }
                }
            }
        }
    };
}

/// Metadata invariants every rule must satisfy: non-empty id, summary and tag
/// list, no whitespace in the id, no duplicate tags.
///
/// # Panics
///
/// Panics when an invariant is violated.
pub fn assert_meta_contract(id: &str, summary: &str, tags: &[&str]) {
    assert!(!id.is_empty(), "rule id must not be empty");
    assert!(!summary.is_empty(), "rule summary must not be empty");
    assert!(!tags.is_empty(), "rule tags must not be empty");
    assert!(
        !id.contains(char::is_whitespace),
        "rule id must not contain whitespace"
    );
    let mut unique = tags.to_vec();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        tags.len(),
        "rule tags must not contain duplicates"
    );
}

/// Asserts that the half-open byte span `[start, end)` lies inside `text`.
///
/// # Panics
///
/// Panics when the span is inverted or ends past the text.
pub fn assert_span_inside(start: u32, end: u32, text: &str, context: &str) {
    let end_index = usize::try_from(end).unwrap_or(usize::MAX);
    assert!(
        start <= end && end_index <= text.len(),
        "span {start}..{end} outside {context}"
    );
}

/// Inputs every rule must handle without panicking: empty, comment-only,
/// non-Ansible shapes and small valid Ansible content.
pub const CONTRACT_INPUTS: &[&str] = &[
    "",
    "\n",
    "# only a comment\n",
    "a: 1\n",
    "- 1\n- [2, {b: c}]\n- null\n",
    "- hosts: all\n",
    "- hosts: all\n  tasks: not-a-list\n  roles: [{}]\n",
    "- name: t\n  ansible.builtin.debug: {msg: x}\n- {}\n- 3\n",
    "- block:\n    - name: b\n      ansible.builtin.debug: {}\n  rescue: x\n",
    "- &t {name: a, debug: {}}\n- *t\n- <<: *t\n",
    "---\na: 1\n---\n- hosts: all\n...\n",
    "key: \"tab\\there\"\r\nother: \u{e9}\u{65e5}\r\n",
];

/// One rule fixture: `tests/fixtures/<rule>/<kind>/<case>.yml` and its
/// expected findings at `tests/fixtures/<rule>/expected/<kind>-<case>.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixtureCase {
    /// Rule id (the fixture directory name).
    pub rule: String,
    /// `fail`, `pass` or `noqa`.
    pub kind: String,
    /// The YAML input.
    pub input: std::path::PathBuf,
    /// The expected findings file.
    pub expected: std::path::PathBuf,
}

/// Fixture kinds that hold YAML inputs (H-C9).
pub const FIXTURE_KINDS: &[&str] = &["fail", "pass", "noqa"];

/// Discovers every rule fixture under `root` (`tests/fixtures`), sorted.
/// Directories starting with `_` (such as `_valid`) are not rule fixtures.
///
/// # Errors
///
/// Returns an I/O error when a directory cannot be read.
pub fn fixture_cases(root: &std::path::Path) -> std::io::Result<Vec<FixtureCase>> {
    let mut cases = Vec::new();
    for rule_dir in sorted_entries(root)? {
        let rule = file_name(&rule_dir);
        if !rule_dir.is_dir() || rule.starts_with('_') {
            continue;
        }
        for kind in FIXTURE_KINDS {
            let kind_dir = rule_dir.join(kind);
            if !kind_dir.is_dir() {
                continue;
            }
            for input in sorted_entries(&kind_dir)? {
                let Some(stem) = input.file_stem().map(|stem| stem.to_string_lossy()) else {
                    continue;
                };
                let expected = rule_dir
                    .join("expected")
                    .join(format!("{kind}-{stem}.json"));
                cases.push(FixtureCase {
                    rule: rule.clone(),
                    kind: (*kind).to_string(),
                    input,
                    expected,
                });
            }
        }
    }
    Ok(cases)
}

fn sorted_entries(directory: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut paths = std::fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    Ok(paths)
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}
