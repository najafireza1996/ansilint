# Repository settings

Checklist for the human maintainer when the repository is published (P0-T7).

## Branch protection (`main`)

- Require a pull request before merging.
- Require at least **1 approving review** (ENG-46).
- Require review from `CODEOWNERS` for architecture-sensitive paths:
  `architecture.toml`, every `Cargo.toml`, `crates/ansilint-core/**`,
  `data/**` and `docs/adr/**`.
- Require status checks: `quality (ubuntu-latest)`, `quality (macos-latest)`,
  `quality (windows-latest)`, `msrv`, `hygiene`, `commit-lint`, `deny`.
- Require branches to be up to date before merging.
- Require linear history; block force pushes and deletions.
- The sole maintainer may self-merge only with green CI and a 24-hour wait
  (ENG-46).

## CODEOWNERS

`CODEOWNERS` currently uses `@ansilint/maintainers` and must be replaced with real
accounts before the first public release (OPEN_QUESTIONS #12).

## Labels

`good first issue`, `help wanted`, `rule:<id>`, `area:<crate>`, `needs-repro`,
`false-positive`, `not-planned`.

## Security

- 2FA required for maintainers.
- Trusted publishing / tokens scoped to release workflows.
- Private vulnerability reporting enabled (`SECURITY.md`).

## Supply chain

Third-party Actions are pinned to released major tags in Phase 0 and must be
pinned to commit SHAs before the first public release (OPEN_QUESTIONS #13).
`cargo deny` and `cargo audit` run in CI.
