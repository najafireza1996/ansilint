use super::matches_pattern;

#[test]
fn enable_list_sub_tags_map_to_the_owning_rule() {
    let enable = ["name[prefix]".to_string(), "no-same-owner".to_string()];
    assert!(matches_pattern(&enable, "name"));
    assert!(matches_pattern(&enable, "no-same-owner"));
    assert!(!matches_pattern(&enable, "name-prefix"));
    assert!(!matches_pattern(&enable, "no-same"));
}
