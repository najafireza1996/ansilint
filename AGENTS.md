# Rules for AI agents

This repository is implemented phase by phase from `ANSILINT_SPEC.md`. Read
[ENGINEERING.md](docs/ENGINEERING.md) before changing code. It is canonical.

## Non-negotiables

1. Work strictly phase by phase. Do not start phase N+1 until every acceptance
   criterion of phase N passes in CI.
2. Never invent upstream behavior. Verify against
   <https://docs.ansible.com/projects/lint/> and record the fact in
   `docs/OPEN_QUESTIONS.md` before coding. When the docs are silent, choose the
   conservative option (fewer findings) and log it.
3. No `unsafe`, no `unwrap`/`expect`/`panic!` in non-test code. Use typed errors
   (`thiserror` in libraries, `anyhow` in binaries).
4. No I/O in pure crates. See `architecture.toml` and each crate's `clippy.toml`.
5. No new crate, dependency, dependency edge, public trait or exception to
   ENGINEERING without an ADR in `docs/adr/` (ENG-40).
6. One logical change per commit, Conventional Commits (ENG-44). Refactors are
   separate from behavior changes.
7. Tests first for bugs and rules. A rule needs `fail/`, `pass/`, `noqa/` fixtures
   (plus `fix/` when fixable), a `docs/rules/<id>.md` page and a `parity.toml` entry.
8. User-visible changes update `CHANGELOG.md` and docs in the same commit.
9. Performance is a feature: read each file once, no per-character allocation,
   benchmark evidence for hot-path changes.
10. Before reporting a task as done, run `cargo xtask ci` and report the result.

## Useful commands

```sh
cargo xtask setup     # verify toolchain, install optional git hooks
cargo xtask check     # architecture and hygiene fitness functions
cargo xtask ci        # the single source of truth for "green"
cargo xtask gen-parity
cargo xtask gen-rule-docs
```

## When blocked

Write the question in `docs/OPEN_QUESTIONS.md`, pick the simplest reversible
option, and continue. Do not silently break an engineering rule.
