# CLI

Status: **pre-alpha**. Phase 1 implements the lint pipeline and the flags marked
"implemented" below. Later phases add fixes, sarif, the language server and
completions. The rest of this page documents the target contract so flag names do
not drift.

## Invocation

```
ansilint [OPTIONS] [PATHS]...
```

With no paths, `ansilint` auto-detects Ansible content from the current directory
(playbooks, roles, collections) and warns when it is not running from the project
root.

## Flags (target)

```
  -c, --config-file <FILE>       Explicit configuration file
  -p, --profile <PROFILE>        min | basic | moderate | safety | shared | production
  -x, --skip-list <RULES>        Hide findings for these tags
  -w, --warn-list <RULES>        Report but do not fail on these tags
      --enable-list <RULES>      Enable opt-in rules
      --exclude <PATH>...        Exclude paths (relative to the current directory)
      --strict                   Treat warnings as failures
  -q, --quiet                    Suppress non-finding output
  -v, -vv, -vvv                  Increase verbosity
  -f, --format <FORMAT>          pep8 | plain | rich | json | codeclimate | quiet | sarif | github
      --sarif-file <FILE>        Also write SARIF to this file
      --nocolor / --force-color  Control ANSI colors
      --fix[=<all|none|RULES>]   Apply safe fixes (Phase 8, not yet)
      --diff                     Print fixes as a unified diff without writing
      --generate-ignore          Rewrite the ignore file from current findings
  -L, --list-rules               List rules
  -T, --list-tags                List tags
  -P, --list-profiles            List profiles
      --offline                  Accepted; linting is always offline
  -V, --version
```

Extra subcommands:

```
ansilint index build [--from-ansible-doc | --from-path <DIR>] [--collections <LIST>]   Phase 5
ansilint index list | show <fqcn>                                                     Phase 5
ansilint explain <rule-id>                                                            Phase 3
ansilint lsp                                                                          Phase 9
ansilint completions <shell>                                                          Phase 4
```

## Implemented in Phase 1

`-c`, `-p`, `-x`, `-w`, `--enable-list`, `--exclude`, `--strict`, `-q`, `-v`,
`-f` (`pep8`, `json`, `quiet`), `-L`, `-T`, `-P`, `--generate-ignore`,
`--nocolor`, `--offline`, `--version` and `--help`.

`-L` prints `id profile summary`; `-T` prints sorted tags; `-P` prints profiles in
cumulative order. These layouts are not yet byte-compatible with upstream and are
marked `VERIFY-UPSTREAM` (OPEN_QUESTIONS #2/#5).

## Exit codes

Implemented in Phase 1, still to be verified byte-for-byte against upstream (see
[OPEN_QUESTIONS.md](OPEN_QUESTIONS.md)):

| Code | Meaning |
|---|---|
| `0` | No fatal findings |
| `2` | Fatal findings present |
| `1` | Internal or usage error |

`strict: true` turns warnings into failures. The verified upstream behavior will
be recorded here and pinned by CLI tests before `v0.1.0`.

## Environment

- `ANSIBLE_LINT_IGNORE_FILE` overrides the ignore-file name.
- `ANSIBLE_LINT_SKIP_SCHEMA_UPDATE` is accepted and ignored.
