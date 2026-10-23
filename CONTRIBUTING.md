# Contributing

Thanks for helping. The project is pre-alpha: the most useful contributions right
now are upstream verification notes (see [docs/OPEN_QUESTIONS.md](docs/OPEN_QUESTIONS.md)),
fitness-function improvements and documentation.

## Setup (one command)

```sh
git clone https://github.com/ansilint/ansilint
cd ansilint
cargo xtask setup   # checks rustc/cargo, installs the optional pre-commit hook
cargo xtask check   # architecture and hygiene checks, under 10 seconds
cargo xtask ci      # everything CI runs: fmt, clippy, tests, deny, machete, doc, coverage
```

`cargo xtask ci` needs `cargo-deny`, `cargo-machete` and `cargo-llvm-cov` when you
run it locally; `cargo xtask setup` tells you which are missing and the exact
`cargo install` command. CI installs them automatically.

If a `rust-toolchain.toml` override is installed, `rustup` uses it automatically.

## Ground rules

Read [docs/ENGINEERING.md](docs/ENGINEERING.md). The short version:

- correctness beats readability beats consistency beats performance; cleverness is last;
- no `unsafe`, no `unwrap`/`expect`/`panic!` outside tests;
- no I/O in pure crates (`architecture.toml` lists them);
- one logical change per commit, Conventional Commits;
- a bug fix starts with a failing test;
- user-visible changes update docs and `CHANGELOG.md`.

## First PR walkthrough

1. Pick a `good first issue` (or a row from [docs/PARITY.md](docs/PARITY.md)).
2. Comment on the issue so work is not duplicated.
3. Create a branch: `git switch -c fix/parser-error-span`.
4. Write the failing test first. Run it and confirm it fails for the expected reason.
5. Make the smallest change that passes. Run `cargo xtask ci`.
6. Update docs in the same commit if behavior is user-visible.
7. Open a PR using the template. CI must be green; a maintainer responds within
   48 hours.

If your PR changes an output format, messages, tags or exit codes, say so
explicitly: those are public API (ENG-30).

## Adding a rule (target: 15 minutes)

Once Phase 2 lands, a generator will scaffold everything:

```sh
cargo xtask new-rule <rule-id>
```

Until then, a rule requires exactly these files:

1. `crates/ansilint-rules/src/rules/<rule_id>.rs` — implement `Rule`.
2. One line in the registry macro (`crates/ansilint-rules/src/registry.rs`).
3. `tests/fixtures/<rule-id>/fail/`, `pass/`, `noqa/` (and `fix/` if fixable)
   with at least one YAML fixture each.
4. `docs/rules/<rule-id>.md` written in your own words, with examples and a link
   to the upstream documentation.
5. A `[rules.<rule-id>]` entry in `parity.toml`.

`cargo xtask check` fails if any of these is missing. Never invent upstream
semantics: verify first, then write the fact in `docs/OPEN_QUESTIONS.md`.

## Commit messages

```
type(scope): description
```

Types: `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`, `refactor`,
`revert`, `style`, `test`. CI validates them with `cargo xtask check-commits`.

## Code of conduct

By participating you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
Security issues: see [SECURITY.md](SECURITY.md).
