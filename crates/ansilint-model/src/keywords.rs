//! The task-keyword table used to discover a task's action key.
//!
//! The list is checked in twice on purpose: as a sorted constant for O(log n)
//! lookups and as `data/task_keywords.txt` so contributors can review additions
//! without reading Rust. A unit test keeps them identical.

/// Keys that are task keywords, not module names. Sorted.
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
    "with_cartesian",
    "with_dict",
    "with_fileglob",
    "with_first_found",
    "with_flattened",
    "with_indexed_items",
    "with_ini",
    "with_inventory_hostnames",
    "with_items",
    "with_lines",
    "with_nested",
    "with_random_choice",
    "with_sequence",
    "with_subelements",
    "with_together",
];

/// Returns `true` when `key` is a task keyword rather than a module name.
#[must_use]
pub fn is_task_keyword(key: &str) -> bool {
    TASK_KEYWORDS.binary_search(&key).is_ok()
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

    const CHECKED_IN: &str = include_str!("../../../data/task_keywords.txt");

    #[test]
    fn keyword_table_is_sorted_and_unique() {
        assert!(TASK_KEYWORDS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn keyword_table_matches_the_checked_in_list() {
        let mut checked_in: Vec<&str> =
            CHECKED_IN.lines().filter(|line| !line.is_empty()).collect();
        checked_in.sort_unstable();
        assert_eq!(TASK_KEYWORDS, checked_in.as_slice());
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
