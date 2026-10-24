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
