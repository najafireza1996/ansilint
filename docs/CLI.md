# CLI

Status: **pre-alpha**. Phase 1 implements the lint pipeline and the flags marked
"implemented" below. Later phases add fixes, sarif, the language server and
completions. The rest of this page documents the target contract so flag names do
not drift.

## Invocation

```
ansilint [OPTIONS] [PATHS]...
```

With no paths, `ansilint` walks the current directory and lints every `.yml` /
`.yaml` file it finds (respecting `.gitignore` and `exclude_paths`). Upstream-style
auto-detection of playbooks, roles and collections, with a warning when not run
from the project root, is planned for Phase 3 (H-C13).

## Flags (target)

```
  -c, --config-file <FILE>       Explicit configuration file
  -p, --profile <PROFILE>        min | basic | moderate | safety | shared | production
  -x, --skip-list <RULES>        Hide findings for these tags
  -w, --warn-list <RULES>        Report but do not fail on these tags
      --enable-list <RULES>      Enable opt-in rules
  -t, --tags <TAGS>              Only report rules whose id, rule[sub] or rule tag matches
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

Verified against upstream ansible-lint `constants.RC` and `app.report_outcome`
at commit `67a48717843c4d5d37681294cb5c9967781b0a31` (2026-10-30); each code
has a test in `crates/ansilint-cli/tests/cli.rs`.

| Code | Meaning | Test |
|---|---|---|
| `0` | Success: no findings, or only warnings | `clean_files_exit_with_zero`, `warnings_pass_unless_strict` |
| `2` | Violations found: any error; with `--strict`, any remaining finding (warnings and `Info` too) | `parser_errors_exit_with_two`, `warnings_pass_unless_strict` |
| `2` | Usage error (upstream uses `argparse`, which exits 2), including an unknown `-f`/`-p` value | `usage_errors_exit_with_two_like_argparse` |
| `3` | Invalid configuration: unreadable/missing config or ignore file, invalid value | `invalid_configuration_exits_with_three` |
| `5` | No file was analyzed (upstream treats a run that linted nothing as a failure) | `no_files_matched_exits_with_five` |
| `130` | Interrupted: default SIGINT termination, which shells report as 130 (Unix; see Windows below) | `interrupt_terminates_with_sigint_which_shells_report_as_130` (Unix) |
| `1` | Unexpected failure of ansilint itself outside rule isolation (e.g. an unreadable directory while walking, or stdout closed before the findings are written) | `failing_to_write_output_exits_with_one_on_every_platform`, `unexpected_io_failure_exits_with_one` (Unix) |

### Ctrl-C on Windows

ansilint installs no console control handler, so on Windows Ctrl-C ends the
process through the default handler with `STATUS_CONTROL_C_EXIT`
(`0xC000013A`). `cmd` shows it as `%ERRORLEVEL%` `-1073741510`, and PowerShell
reports the same value in `$LASTEXITCODE`. It is never `130`: Windows has no
signals, and no shell turns the status into 128 + signal number. Scripts that
need to tell an interruption apart should test for that value. No test covers
this: sending Ctrl-C to a child process needs the Win32
`GenerateConsoleCtrlEvent` API, which requires `unsafe` or a new dependency
(both ruled out, AGENTS.md).

Upstream's `4` (lock timeout) is never emitted; `8` (violations fixed) is
reserved for `--fix` in Phase 8. Usage errors exited `1` before H-E1 (H-C12);
that decision is reversed to match upstream.

## Environment

- `ANSIBLE_LINT_IGNORE_FILE` overrides the ignore-file name.
- `ANSIBLE_LINT_SKIP_SCHEMA_UPDATE` is accepted and ignored.
