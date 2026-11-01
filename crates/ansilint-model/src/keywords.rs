//! The task-keyword table used to discover a task's action key.
//!
//! `data/task_keywords.txt` is a verbatim copy of the official Playbook
//! Keywords reference (Task section). Tests fail when a documented keyword is
//! not recognised here, or when this table holds an undocumented one.

/// Keys that are task keywords, not module names. Sorted.
///
/// Excludes the documented action carriers `action`/`local_action` (handled by
/// the builder) and `with_<lookup_plugin>` (matched as a prefix); adds the
/// handler keyword `listen`.
pub const TASK_KEYWORDS: &[&str] = &[
    "any_errors_fatal",
    "args",
    "async",
    "become",
    "become_exe",
    "become_flags",
    "become_method",
    "become_user",
    "changed_when",
    "check_mode",
    "collections",
    "connection",
    "debugger",
    "delay",
    "delegate_facts",
    "delegate_to",
    "diff",
    "environment",
    "failed_when",
    "ignore_errors",
    "ignore_unreachable",
    "listen",
    "loop",
    "loop_control",
    "module_defaults",
    "name",
    "no_log",
    "notify",
    "poll",
    "port",
    "register",
    "remote_user",
    "retries",
    "run_once",
    "tags",
    "throttle",
    "timeout",
    "until",
    "vars",
    "when",
];

/// Returns `true` when `key` is a task keyword rather than a module name.
#[must_use]
pub fn is_task_keyword(key: &str) -> bool {
    key.starts_with("with_") || TASK_KEYWORDS.binary_search(&key).is_ok()
}

/// Action keys handled as legacy forms or includes rather than plain modules.
#[must_use]
pub fn is_include_action(module: &str) -> bool {
    matches!(
        module.rsplit('.').next().unwrap_or(module),
        "include"
            | "include_tasks"
            | "import_tasks"
            | "include_role"
            | "import_role"
            | "include_playbook"
            | "import_playbook"
    )
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        reason = "test fixtures are fixed"
    )]

    use super::{is_include_action, is_task_keyword, TASK_KEYWORDS};

    const DOCUMENTED: &str = include_str!("../../../data/task_keywords.txt");
    /// Documented keywords that carry the action; the builder handles them.
    const ACTION_CARRIERS: &[&str] = &["action", "local_action"];
    /// Valid on handlers, documented outside the Playbook Keywords page.
    const HANDLER_ONLY: &[&str] = &["listen"];

    fn documented() -> Vec<&'static str> {
        DOCUMENTED
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect()
    }

    #[test]
    fn keyword_table_is_sorted_and_unique() {
        assert!(TASK_KEYWORDS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn checked_in_list_records_its_source_and_date() {
        assert!(DOCUMENTED.contains("https://docs.ansible.com/"));
        assert!(DOCUMENTED.contains("Retrieved: "));
    }

    #[test]
    fn every_documented_task_keyword_is_recognised() {
        let missing: Vec<&str> = documented()
            .into_iter()
            .filter(|keyword| !ACTION_CARRIERS.contains(keyword))
            .filter(|keyword| match keyword.strip_suffix("<lookup_plugin>") {
                Some(prefix) => {
                    !is_task_keyword(&format!("{prefix}env"))
                        || !is_task_keyword(&format!("{prefix}community.general.random_words"))
                }
                None => !is_task_keyword(keyword),
            })
            .collect();
        assert!(
            missing.is_empty(),
            "documented but unrecognised: {missing:?}"
        );
    }

    #[test]
    fn keyword_table_has_no_undocumented_entries() {
        let documented = documented();
        let extra: Vec<&&str> = TASK_KEYWORDS
            .iter()
            .filter(|keyword| !documented.contains(keyword) && !HANDLER_ONLY.contains(keyword))
            .collect();
        assert!(extra.is_empty(), "not in the documented list: {extra:?}");
    }

    #[test]
    fn distinguishes_keywords_from_actions() {
        assert!(is_task_keyword("when"));
        assert!(is_task_keyword("become_user"));
        assert!(!is_task_keyword("ansible.builtin.copy"));
        assert!(!is_task_keyword("action"));
        assert!(!is_task_keyword("include_tasks"));
    }

    #[test]
    fn recognises_include_and_import_actions() {
        assert!(is_include_action("ansible.builtin.include_tasks"));
        assert!(is_include_action("import_role"));
        assert!(!is_include_action("ansible.builtin.copy"));
    }
}
