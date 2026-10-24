# 0002. Phase 1 dependencies

Status: accepted

## Context

Phase 1 added the first external dependencies beyond `clap`, `anyhow`, `serde`,
`serde_json`, `toml`, `tempfile`, `assert_cmd` and `predicates`. ENG-42 requires a
justification for each: what it does, why `std`/existing dependencies do not
suffice, license, maintenance, MSRV, `unsafe` usage and size/compile cost. ENG-40
requires an ADR for significant new dependencies.

The MSRV is Rust 1.85 and several recent releases of `ignore`/`globset` require
1.88, so those two are pinned exactly.

## Decision

Add the following:

| Dependency | Used by | Why | License | Notes |
|---|---|---|---|---|
| `saphyr-parser` 0.2.2 | `ansilint-yaml` | YAML 1.2 events with byte spans; ADR 0001 | MIT OR Apache-2.0 | No C dependency; MSRV 1.85 |
| `thiserror` 2 | libraries | Typed, contextual errors without stringly-typed errors (ENG-25) | MIT OR Apache-2.0 | Proc-macro compile cost only |
| `ignore` 0.4.23 (pinned) | `ansilint-engine` | Gitignore-aware walking and symlink rules | MIT OR Unlicense | Newer releases require Rust 1.88 |
| `globset` 0.4.15 (pinned) | `ansilint-engine` | Ordered file-kind globs and excludes | MIT OR Unlicense | Newer releases require Rust 1.88 |
| `rayon` 1.10 | `ansilint-engine` | Deterministic per-file parallelism | MIT OR Apache-2.0 | No shared mutable state; results are sorted |
| `serde_json` | `ansilint-output` | Stable machine-readable output | MIT OR Apache-2.0 | Already in the workspace for metadata |
| `proptest` (dev) | `ansilint-core` | `LineIndex` round-trip properties | MIT OR Apache-2.0 | Test-only |
| `insta` (dev) | `ansilint-output`, `ansilint-config` | Output and message snapshots | Apache-2.0 | Test-only |
| `tempfile` (dev) | tests | Hermetic temporary directories (ENG-35) | MIT OR Apache-2.0 | Test-only |

No dependency introduces a transitive `unsafe` surface in our code, and none is
used in a pure crate for I/O: `ignore` and `rayon` live only in the engine.

## Consequences

- The dependency tree grows, but every addition has a single owning crate.
- `ignore`/`globset` are pinned; Dependabot will propose upgrades, and the MSRV
  job rejects them until the MSRV is raised by ADR.
- Proc-macro dependencies (`thiserror`, `serde_derive`) add compile time; the
  workspace caches them.
- Coverage and `cargo machete` catch unused additions.

## Alternatives considered

- **`walkdir` instead of `ignore`:** no `.gitignore` support; rejected.
- **Manual glob matching instead of `globset`:** error-prone and slower; rejected.
- **Thread pool from `std` instead of `rayon`:** more code, same per-file
  granularity; rejected for Phase 1.
- **`yaml-rust2`/`serde_yaml`:** no spans or deprecated; see ADR 0001.
- **`anyhow` in libraries:** loses typed errors; rejected (binaries only).
