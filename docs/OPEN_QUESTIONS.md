# Open questions

Questions that block or constrain implementation. Until an item is verified, the
code implements the conservative option (fewer findings) and marks the spot with
`VERIFY-UPSTREAM(Q<n>)`, where `<n>` is the row number below (H-E4). `cargo xtask
check` rejects the marker without a row number, with a number that has no row,
and the old `TODO(verify)` form. Never guess upstream behavior.

## Open

| # | Question | Impact | Conservative default | Owner |
|---|---|---|---|---|
| 2 | Exact upstream `json`/`codeclimate` field names (exit codes: verified, see Verified #24) | Phase 4 | `json` mirrors the documented upstream shapes where known | — |
| 4 | Upstream embedded yamllint configuration (which checks, which options) | Phase 2 `yaml` rule | Implement only checks we can verify; never enable a check by guess | — |
| 5 | Sub-tag lists and default severities for `fqcn`, `args`, `name`, `jinja`, `schema` | Phases 2–7 | Implement verified sub-tags only; unknown sub-tags are not emitted | — |
| 6 | Whether `args`, `complexity`, `jinja-template-extension`, `loop-var-prefix`, `no-prompting`, `run-once` are default-enabled or opt-in | Phase 3 / Phase 7 | Treat as opt-in until verified | — |
| 8 | Which collections may be embedded in the default index (license review) | Phase 5 | `ansible.builtin` only until reviewed | — |
| 9 | Schema licensing and vendoring vs user-cache fetch | Phase 6 | Vendor only schemas with a confirmed compatible license | — |
| 10 | Trademark-safe project name and final binary/crate names | Before first public release | Keep `ansilint` as the working name; run a global rename | — |
| 11 | `# noqa` exact grammar, task-level `skip_ansible_lint`, and whether any tags are non-skippable | Phase 1 post-processing | `rule` and `rule[sub]` are implemented; `schema`/parser/load/internal errors are non-skippable by noqa, `skip_list`, ignore `skip` and `tags` (they may be downgraded, H-D2); task-level skipping is not implemented | — |
| 13 | Review the commit SHAs used to pin third-party GitHub Actions | CI hardening | Pinned to commit SHAs since Phase 0; review on every dependency update | — |
| 14 | LSP stack (`tower-lsp-server` vs `lsp-server` + `lsp-types`) | Phase 9 | Decide by ADR in P9-T1 | — |
| 28 | Upstream tags the empty-playbook finding `syntax-check[empty-playbook]`; ours is `warning` with message `empty playbook`. Renaming changes a user-visible tag (ENG-30). | Phase 6 `syntax-check` (static equivalent) | Keep `warning` until `syntax-check` exists, then move it there with a CHANGELOG entry | maintainer |
| 15 | ~~The line-oriented comment scanner can see a `#` inside a literal/folded block scalar as a comment~~ **Resolved (H-D1):** candidates inside quoted or block scalars are discarded using AST scalar spans, and `# noqa` reads only real comment tokens. | — | — | — |

## Decided

Rows the maintainer accepted as decided (H-E7). They keep their numbers so
existing references stay valid. To reopen a decision, move it back to Open.

| # | Question | Decision | Date | Follow-up |
|---|---|---|---|---|
| 12 | GitHub maintainer handles and required reviewers | `CODEOWNERS` names `@najafireza1996`, the sole maintainer (H-E3) | 2026-11-04 | None. Add handles when a second maintainer joins (ADR 0004) |
| 18 | Explicit rayon worker stack size (H-B3) | No custom stack: recursion is capped at `MAX_NESTING_DEPTH` (128 levels) and `Node::anchors` is iterative; 100 000-deep inputs pass on the default 2 MiB stacks. A custom pool would spawn threads on every `lint()` call | 2026-11-04 | None. Revisit if recursion is added that `MAX_NESTING_DEPTH` does not bound |
| 19 | Hidden files and directories (`.github/`, `.cache/`) | Keep skipping them, the conservative option (upstream lints them) | 2026-11-04 | P3-T8: decide from compat-harness data; flip with a CHANGELOG entry |
| 21 | Precedence when both `.ansible-lint-ignore` and `.config/ansible-lint-ignore.txt` exist | `.ansible-lint-ignore` wins (next to the config, then at the project root); every single-file layout is pinned by `finds_the_ignore_file_in_every_documented_layout` | 2026-11-04 | None (H-C5 done). Re-check if upstream documents a precedence |
| 23 | `enable_list: [rule[sub]]` sub-tag filtering (H-D6) | Maps to the owning rule for now; no Phase 1 rule has an opt-in sub-tag, so nothing can over-report | 2026-11-04 | P2-T6: per-sub-tag opt-in in `RuleMeta`, together with the `name` rule |
| 26 | Working branch for Batch E | Batch E continues on `phase-1.5-batch-a`; the stale `phase-1-hardening` branch is not touched | 2026-11-04 | None as a task. The maintainer picks the branch to merge before `v0.0.1` is tagged |

**Audit of #26 (2026-11-04).** The 10 commits only on `phase-1-hardening`
(`dc33d52`…`28ebea8`) are not patch-identical to anything on `main`, but their
content is: every test and function they add exists on `main` (`aae0499`,
checked with `git grep -w "fn <name>" main`), as do the files of the three
commits without functions (`contracts.rs`, the `model` fuzz target and seed,
`COMPAT_POLICY.md`, `BENCHMARKS.md`, the `non_exhaustive` enums and the
panic-strategy check). Four names are absent on `phase-1.5-batch-a` only because
later work replaced them on purpose: `usage_errors_exit_with_one` →
`usage_errors_exit_with_two_like_argparse` (H-E1),
`handles_uppercase_extensions_by_sniffing` →
`discovers_files_with_uppercase_extensions` (`b2beda7`), and the old fixture
helpers in `tests/fixtures.rs` → the `ansilint-testing` harness (H-C9). Nothing
is missing; both old branches stay untouched.

## Verified

Record each verified fact with its source URL and the date. Do not delete entries
when they are answered; move them here so future changes can be checked against
them.

| # | Fact | Source | Date |
|---|---|---|---|
| 1 | YAML parser is `saphyr-parser` 0.2.2 (YAML 1.2, byte spans, multi-doc, anchors/aliases as ids). Comments are scanned line-by-line because the parser does not emit them. | <https://docs.rs/saphyr-parser/0.2.2/> and ADR 0001 | 2026-10-23 |
| 7 | Message policy: tags and profiles verbatim; messages verbatim when verified against a real `ansible-lint` run, otherwise documented per rule. | `docs/COMPAT_POLICY.md`, ansible-lint 26.10.1 runs on two public repos | 2026-10-23 |
| 11a | MSRV is Rust 1.85 with edition 2021; CI runs a dedicated MSRV job. | Internal decision, P0-T2 | 2026-10-23 |
| 17 | `-t/--tags` keeps rules "whose id/tags match these values"; we also accept `rule[sub]`. Config key `tags` is not documented upstream; it shares the CLI semantics. | <https://docs.ansible.com/projects/lint/usage/> | 2026-11-01 |
| 20 | `exclude_paths` entries resolve relative to the config file; `.cache/` is "implicit unless exclude_paths is defined in config"; config is never loaded from outside the git repository; the ignore file "should reside adjacent to the config file". | <https://docs.ansible.com/projects/lint/configuring/> | 2026-11-01 |
| 22 | `kinds` entries are single-key `<kind>: <glob>` mappings (e.g. `- playbook: "**/examples/*.{yml,yaml}"`), prepended to the defaults; first match wins. | <https://raw.githubusercontent.com/ansible/ansible-lint/main/.ansible-lint> | 2026-11-01 |
| 24 | Exit codes (`constants.RC`): 0 success (incl. warnings only), 2 violations, 3 invalid config, 4 lock timeout, 5 no files matched, 8 fixed violations, 130 Ctrl-C; `--strict` fails on any remaining match; usage errors come from plain `argparse` (exit 2). | <https://github.com/ansible/ansible-lint/blob/67a48717843c4d5d37681294cb5c9967781b0a31/src/ansiblelint/constants.py>, `app.py` `report_outcome`, `__main__.py` | 2026-11-01 |
| 25 | Default `kinds` are upstream `DEFAULT_KINDS` (39 ordered entries, first match wins), copied to `data/default_kinds.toml`; upstream matches with wcmatch (`*` stays in one path component). Its rulebook glob has an unclosed brace and matches nothing. | <https://github.com/ansible/ansible-lint/blob/67a48717843c4d5d37681294cb5c9967781b0a31/src/ansiblelint/config.py> | 2026-11-01 |
| 27 | "Empty playbook, nothing to do" is raised by ansible-core only when the loaded playbook data is `None` (empty or comment-only file); a play without tasks is valid. ansible-lint relays it as `syntax-check[empty-playbook]`. | <https://github.com/ansible/ansible/blob/025bb4d508b0a649b91f346334d557bc65736665/lib/ansible/playbook/__init__.py>, <https://github.com/ansible/ansible-lint/blob/67a48717843c4d5d37681294cb5c9967781b0a31/src/ansiblelint/rules/syntax_check.md> | 2026-11-01 |
| 16 | Task keywords are the "Task" list of the Playbook Keywords reference, copied verbatim into `data/task_keywords.txt`. `action`/`local_action` carry the action; `with_<lookup_plugin>` is any `with_` prefix. `listen` is a handler keyword documented elsewhere. | <https://docs.ansible.com/projects/ansible/latest/reference_appendices/playbooks_keywords.html> | 2026-11-01 |

## How to resolve an item

1. Fetch the official source (`https://docs.ansible.com/projects/lint/` or the
   upstream repository at a pinned commit).
2. Write the verified facts here, with the URL and date.
3. Add a regression test that fails if the behavior changes.
4. Only then remove the `VERIFY-UPSTREAM(Q<n>)` marker and update the rule documentation.
