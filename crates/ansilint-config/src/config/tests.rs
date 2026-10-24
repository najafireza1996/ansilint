#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests use fixed fixtures and panic on broken assertions"
)]

use super::{merge, Config, ConfigOverrides, ParsedConfig};
use crate::Profile;

fn parse(text: &str) -> ParsedConfig {
    Config::from_bytes(text).expect("valid configuration")
}

#[test]
fn empty_input_gives_defaults() {
    let parsed = parse("");
    assert_eq!(parsed.config, Config::default());
}

#[test]
fn parses_known_keys() {
    let parsed = parse(
        "profile: basic\nexclude_paths: [.cache, vendor]\nskip_list: [yaml]\nquiet: true\nstrict: true\nverbosity: 2\nkinds:\n  - kind: playbook\n    glob: \"playbooks/**/*.yml\"\n",
    );
    assert_eq!(parsed.config.profile, Profile::Basic);
    assert_eq!(parsed.config.exclude_paths.len(), 2);
    assert_eq!(parsed.config.skip_list, vec!["yaml"]);
    assert!(parsed.config.quiet);
    assert!(parsed.config.strict);
    assert_eq!(parsed.config.verbosity, 2);
    assert_eq!(parsed.config.kinds.len(), 1);
}

#[test]
fn reports_unknown_and_ignored_keys() {
    let parsed = parse("unknown_key: 1\nrulesdir: rules\n");
    assert_eq!(parsed.unknown_keys, vec!["unknown_key"]);
    assert_eq!(parsed.ignored_keys, vec!["rulesdir"]);
}

#[test]
fn rejects_invalid_values_with_a_span() {
    let error = Config::from_bytes("quiet: maybe\n").expect_err("invalid boolean");
    assert!(error.span.start > 0);
}

#[test]
fn merges_lists_and_overrides_scalars() {
    let base = parse("skip_list: [a]\nwarn_list: [b]\nquiet: false\n").config;
    let overrides = ConfigOverrides {
        skip_list: vec!["c".to_string()],
        quiet: Some(true),
        profile: Some(Profile::Safety),
        ..ConfigOverrides::default()
    };
    let merged = merge(&base, &overrides);
    assert_eq!(merged.skip_list, vec!["a", "c"]);
    assert_eq!(merged.warn_list, vec!["b"]);
    assert!(merged.quiet);
    assert_eq!(merged.profile, Profile::Safety);
}

#[test]
fn parses_every_scalar_and_list_key() {
    let parsed = parse(
        "profile: production\nsarif_file: out.sarif\nvar_naming_pattern: '^[a-z]+$'\nloop_var_prefix: item_\ntask_name_prefix: acme_\nmax_block_depth: 4\nonly_builtins_allow_collections: [ansible.builtin]\nonly_builtins_allow_modules: [copy]\nwrite_list: [yaml]\noffline: false\nuse_default_rules: false\nsupported_ansible_also: ['2.18']\nenable_list: ['name[prefix]']\ntags: [yaml]\nverbosity: 3\n",
    );
    assert_eq!(parsed.config.profile, Profile::Production);
    assert_eq!(
        parsed.config.sarif_file,
        Some(std::path::PathBuf::from("out.sarif"))
    );
    assert_eq!(
        parsed.config.var_naming_pattern.as_deref(),
        Some("^[a-z]+$")
    );
    assert_eq!(parsed.config.loop_var_prefix.as_deref(), Some("item_"));
    assert_eq!(parsed.config.task_name_prefix.as_deref(), Some("acme_"));
    assert_eq!(parsed.config.max_block_depth, Some(4));
    assert_eq!(
        parsed.config.only_builtins_allow_collections,
        vec!["ansible.builtin"]
    );
    assert_eq!(parsed.config.only_builtins_allow_modules, vec!["copy"]);
    assert_eq!(parsed.config.write_list, vec!["yaml"]);
    assert!(!parsed.config.offline);
    assert!(!parsed.config.use_default_rules);
    assert_eq!(parsed.config.supported_ansible_also, vec!["2.18"]);
    assert_eq!(parsed.config.enable_list, vec!["name[prefix]"]);
    assert_eq!(parsed.config.tags, vec!["yaml"]);
    assert_eq!(parsed.config.verbosity, 3);
}

#[test]
fn rejects_invalid_structure_and_values() {
    assert!(Config::from_bytes("[]\n").is_err());
    assert!(Config::from_bytes("kinds: 5\n").is_err());
    assert!(Config::from_bytes("kinds:\n  - kind: playbook\n").is_err());
    assert!(Config::from_bytes("max_block_depth: nope\n").is_err());
    assert!(Config::from_bytes("verbosity: 999\n").is_err());
    assert!(Config::from_bytes("profile: none\n").is_err());
}
