# Repository settings

Checklist for the human maintainer when the repository is published (P0-T7).

## Branch protection (`main`)

While the project has one maintainer (ADR 0004), enable only the last two
items below (block force pushes and deletion; no linear-history requirement).
Turn on the pull-request items when PR mode returns via a superseding ADR.

- Require a pull request before merging.
- Require at least **1 approving review** (ENG-46).
- Require review from `CODEOWNERS` for architecture-sensitive paths:
  `architecture.toml`, every `Cargo.toml`, `crates/ansilint-core/**`,
  `data/**` and `docs/adr/**`.
- Require status checks: `quality (ubuntu-latest)`, `quality (macos-latest)`,
  `quality (windows-latest)`, `msrv`, `hygiene`, `commit-lint`, `deny`.
- Require branches to be up to date before merging.
- Block force pushes to and deletion of `main`. Do **not** require linear
  history: the maintainer merges with `git merge --no-ff` (ADR 0004).
- While there is one maintainer, the maintainer pushes merges to `main`
  directly under the merge gate of ADR 0004 (ENGINEERING 12.9a); the review
  rules above apply to external pull requests. GitHub cannot enforce status
  checks on those direct pushes, so the gate is enforced by discipline.

## CODEOWNERS

`CODEOWNERS` names the sole maintainer, `@najafireza1996` (H-E3, OPEN_QUESTIONS
Decided #12). Add accounts when further maintainers join.

## Labels

`good first issue`, `help wanted`, `rule:<id>`, `area:<crate>`, `needs-repro`,
`false-positive`, `not-planned`.

## Security

- 2FA required for maintainers.
- Trusted publishing / tokens scoped to release workflows.
- Private vulnerability reporting enabled (`SECURITY.md`).

## Supply chain

Third-party Actions are pinned to commit SHAs (OPEN_QUESTIONS #13). Review the
pins on every dependency update.
`cargo deny` and `cargo audit` run in CI.
