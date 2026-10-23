# Open questions

Questions that block or constrain implementation. Until an item is verified, the
code implements the conservative option (fewer findings) and marks the spot with
`TODO(verify)` only internally — never a guessed upstream behavior.

## Open

| # | Question | Impact | Conservative default | Owner |
|---|---|---|---|---|
| 1 | YAML crate choice: which parser exposes byte spans, comments, anchors/aliases, merge keys and multi-document streams? | Phase 1 loader | Spike in P1-T2; ADR `0001-yaml-parser` required before coding the wrapper | — |
| 2 | Exact upstream exit codes, and the exact `json`/`codeclimate` field names | Phase 1 / Phase 4 | `0` no fatal findings, `2` fatal findings, `1` internal/usage error; `json` mirrors the documented upstream shapes where known | — |
| 3 | Default `kinds` glob list and its precedence | Phase 1 file discovery | Ordered glob list, content sniffing as fallback; record before shipping | — |
| 4 | Upstream embedded yamllint configuration (which checks, which options) | Phase 2 `yaml` rule | Implement only checks we can verify; never enable a check by guess | — |
| 5 | Sub-tag lists and default severities for `fqcn`, `args`, `name`, `jinja`, `schema` | Phases 2–7 | Implement verified sub-tags only; unknown sub-tags are not emitted | — |
| 6 | Whether `args`, `complexity`, `jinja-template-extension`, `loop-var-prefix`, `no-prompting`, `run-once` are default-enabled or opt-in | Phase 3 / Phase 7 | Treat as opt-in until verified | — |
| 7 | Message-text compatibility policy: verbatim vs paraphrase | Phase 1 onwards | Keep short factual messages stable; document the policy in `docs/COMPAT_POLICY.md` before Phase 5 | — |
| 8 | Which collections may be embedded in the default index (license review) | Phase 5 | `ansible.builtin` only until reviewed | — |
| 9 | Schema licensing and vendoring vs user-cache fetch | Phase 6 | Vendor only schemas with a confirmed compatible license | — |
| 10 | Trademark-safe project name and final binary/crate names | Before first public release | Keep `ansilint` as the working name; run a global rename | — |
| 11 | `# noqa` exact grammar, task-level `skip_ansible_lint`, and whether any tags are non-skippable | Phase 1 post-processing | Support `rule` and `rule[sub]`; `schema`/parser errors are non-skippable | — |
| 12 | GitHub organization, maintainer handles and required reviewers | Repository setup | `CODEOWNERS` uses `@ansilint/maintainers` until real handles exist | — |
| 13 | SHA pinning for third-party GitHub Actions | CI hardening | Pinned to released major tags for Phase 0; replace with commit SHAs before public release | — |
| 14 | LSP stack (`tower-lsp-server` vs `lsp-server` + `lsp-types`) | Phase 9 | Decide by ADR in P9-T1 | — |

## Verified

Record each verified fact with its source URL and the date. Do not delete entries
when they are answered; move them here so future changes can be checked against
them.

| # | Fact | Source | Date |
|---|---|---|---|
| 11a | MSRV is Rust 1.85 with edition 2021; CI runs a dedicated MSRV job. | Internal decision, P0-T2 | 2026-10-23 |

## How to resolve an item

1. Fetch the official source (`https://docs.ansible.com/projects/lint/` or the
   upstream repository at a pinned commit).
2. Write the verified facts here, with the URL and date.
3. Add a regression test that fails if the behavior changes.
4. Only then remove the `TODO(verify)` marker and update the rule documentation.
