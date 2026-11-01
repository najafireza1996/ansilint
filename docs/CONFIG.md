# Configuration

Status: **pre-alpha**. Phase 1 implements discovery, typed parsing and merge
semantics. The keys and merge semantics below mirror upstream; each one is verified against the
official docs before implementation and recorded in
[OPEN_QUESTIONS.md](OPEN_QUESTIONS.md).

## Discovery

First found wins, searched from the current directory upwards, never outside the
git repository:

1. `.ansible-lint`
2. `.ansible-lint.yml`
3. `.ansible-lint.yaml`
4. `.config/ansible-lint.yml`
5. `.config/ansible-lint.yaml`

`-c/--config-file` takes precedence. Symlinks are followed.

## Keys (initial)

`profile`, `exclude_paths`, `skip_list`, `warn_list`, `enable_list`, `tags`,
`quiet`, `strict`, `verbosity`, `sarif_file`, `kinds`, `var_naming_pattern`,
`loop_var_prefix`, `task_name_prefix`, `max_block_depth`,
`only_builtins_allow_collections`, `only_builtins_allow_modules`, `write_list`,
`offline`, `use_default_rules`, `supported_ansible_also`.

Accepted and validated but **not implemented yet**, each with a notice so no
key is silently inert: `verbosity`, `sarif_file`, `var_naming_pattern`,
`loop_var_prefix`, `task_name_prefix`, `max_block_depth`,
`only_builtins_allow_collections`, `only_builtins_allow_modules`, `write_list`,
`supported_ansible_also`. Each takes effect with the rule or feature that reads
it. `offline` is honored by design: linting never uses the network.

Accepted but ignored, with a one-time notice (non-goals): `rulesdir`,
`mock_modules`, `mock_roles`, `extra_vars`, `skip_action_validation`.

Unknown keys are validated against the vendored `ansible-lint-config` schema
(Phase 6) and reported as `schema[ansible-lint-config]`.

## Merge semantics

- Scalar CLI options override config scalars.
- List options (`skip_list`, `warn_list`, `exclude_paths`) are **extended**, not
  replaced.
- `exclude_paths` from a config file are relative to the config file's location;
  `--exclude` is relative to the current directory.
- When `exclude_paths` is absent, `.cache/` (next to the config) is excluded
  implicitly; defining `exclude_paths` drops that default, as upstream does.
  `.git/` is always skipped. Hidden files and directories are currently not
  walked at all (see OPEN_QUESTIONS #19).

Every bullet is `VERIFY-UPSTREAM` until a test pins it. Phase 1 has tests for
"lists extend / scalars override", unknown and ignored keys, and the ignore-file
grammar.

## Paths

All paths (`-c`, inputs, `exclude_paths`, `--exclude`) are made absolute and
have `.`/`..` resolved lexically before use; symlinks are not resolved.
Reported paths are relative to the project root (git root, else the current
directory) with `/` separators; files outside the root get `../` segments and
only a file on another Windows drive is shown absolute. Matching is
case-sensitive on every platform, including Windows.

## `tags` (and `-t/--tags`)

Upstream: "only check rules whose id/tags match these values"
(<https://docs.ansible.com/projects/lint/usage/>). A finding is kept when a
selector equals its rule id, its full `rule[sub]` tag, or any tag of its rule
(as listed by `-T`, e.g. `syntax`). CLI values extend the config list. A
selector that matches no registered rule produces a notice
(`tag selector `x` matches no rule`), so a typo never yields a silently empty
report.

## Ignore file

Discovery order: `ANSIBLE_LINT_IGNORE_FILE`, then `.ansible-lint-ignore` and
`.config/ansible-lint-ignore.txt` next to the configuration (or project root).
Each line is `<path> <tag> [skip]`; a trailing `# comment` is ignored. Without
`skip` the finding is downgraded to a warning. `--generate-ignore` rewrites the
file from the current findings.
