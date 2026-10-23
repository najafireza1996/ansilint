# Configuration

Status: **pre-alpha**. Config discovery and parsing land in Phase 1 (P1-T3). The
keys and merge semantics below mirror upstream; each one is verified against the
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
- When `exclude_paths` is absent, `.cache/` is excluded by default.

Every bullet is `VERIFY-UPSTREAM` until a test pins it.
