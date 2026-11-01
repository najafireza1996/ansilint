use super::matches_pattern;

#[test]
fn enable_list_sub_tags_map_to_the_owning_rule() {
    let enable = ["name[prefix]".to_string(), "no-same-owner".to_string()];
    assert!(matches_pattern(&enable, "name"));
    assert!(matches_pattern(&enable, "no-same-owner"));
    assert!(!matches_pattern(&enable, "name-prefix"));
    assert!(!matches_pattern(&enable, "no-same"));
}

fn result_with(severity: ansilint_core::Severity) -> super::LintResult {
    let finding = ansilint_core::Finding::new(
        ansilint_core::Tag::new("warning"),
        "message",
        std::path::PathBuf::from("a.yml"),
        ansilint_core::Span::new(0, 1),
        severity,
    );
    super::LintResult {
        findings: vec![super::PositionedFinding {
            finding,
            line: 1,
            col: 1,
        }],
        files_linted: 1,
        config: ansilint_config::Config::default(),
        config_path: None,
        ignore_file_path: None,
        warnings: Vec::new(),
    }
}

#[test]
fn strict_fails_on_any_remaining_finding_including_info() {
    // Upstream: `options.strict and result.matches` (Verified #24).
    use ansilint_core::Severity;
    assert!(!result_with(Severity::Info).has_fatal(false));
    assert!(result_with(Severity::Info).has_fatal(true));
    assert!(!result_with(Severity::Warning).has_fatal(false));
    assert!(result_with(Severity::Warning).has_fatal(true));
    assert!(result_with(Severity::Error).has_fatal(false));
}
