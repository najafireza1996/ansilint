# 0003. Phase 1 dependency audit (ENG-42)

Status: accepted

## Context

ADR 0002 justified each Phase 1 dependency but did not answer every ENG-42
question: maintenance health, `unsafe` usage, and compile-time and binary-size
impact. ADRs are immutable, so this record supersedes 0002 with the complete
answers (H-C14). Facts were measured on 2026-11-01 from the exact `Cargo.lock`
versions in `~/.cargo/registry` and with `cargo tree`.

## Decision

Keep the Phase 1 dependency set unchanged. Per dependency:

| Dependency (locked) | Kind | Why not std / existing | License | MSRV | `unsafe` lines in `src/` | Own transitive crates |
|---|---|---|---|---|---|---|
| `saphyr-parser` 0.2.2 | shipped | YAML 1.2 events with byte spans (ADR 0001) | MIT OR Apache-2.0 | 1.85 | 0 | 9 |
| `thiserror` 2.0.21 | shipped | typed errors without boilerplate (ENG-25) | MIT OR Apache-2.0 | 1.77 | 0 | 7 (proc-macro) |
| `ignore` 0.4.23 (pinned) | shipped | `.gitignore`-aware walking | MIT OR Unlicense | none declared; newer need 1.88 | 0 | 14 |
| `globset` 0.4.15 (pinned) | shipped | ordered kind/exclude globs | MIT OR Unlicense | none declared; newer need 1.88 | 0 | 7 |
| `rayon` 1.12.0 | shipped | per-file data parallelism | MIT OR Apache-2.0 | 1.80 | 67 | 5 |
| `serde_json` 1.0.154 | shipped | JSON output | MIT OR Apache-2.0 | 1.71 | 13 | 4 |
| `clap` 4.6.7 | shipped (CLI) | upstream-compatible flags | MIT OR Apache-2.0 | 1.85 | 0 (`forbid`) | 18 |
| `anyhow` 1.0.104 | shipped (CLI) | context-carrying errors in the binary only | MIT OR Apache-2.0 | 1.68 | 104 | 0 |
| `proptest` 1.12.0 | dev | property tests (`core`, `config`) | MIT OR Apache-2.0 | **1.88** | 5 | — |
| `insta` 1.49.0 | dev | output snapshots | Apache-2.0 | 1.66 | 1 | — |
| `tempfile` 3.27.0 | dev | hermetic temp dirs (ENG-35) | MIT OR Apache-2.0 | 1.63 | 3 | — |
| `assert_cmd` 2.2.2 | dev | binary integration tests | MIT OR Apache-2.0 | 1.85 | 0 | — |
| `predicates` 3.1.4 | dev | output assertions | MIT OR Apache-2.0 | 1.74 | 10 | — |

Maintenance: release dates were **not** measured for this record. Observed
evidence of active development: newer releases than our pins exist for
`ignore` (0.4.33) and `globset` (0.4.20). `saphyr-parser` is the youngest
dependency; ADR 0001 and our fuzz targets cover that risk. `cargo deny` gates
advisories and licenses on every CI run. Checking crates.io release recency is
left to the Phase 1 independent review.

Whole-binary impact (release profile, Apple M1, Rust 1.99.1): 50 crates in the
shipped graph; a warm-cache `cargo build --release -p ansilint-cli` took 16.7 s;
the binary is 3.13 MB (2.47 MB stripped). These are the baselines for the
ENG-33 "no more than 10 % regression without an ADR" rule.

## Consequences

- `unsafe` exists only inside `rayon`, `serde_json`, `anyhow` and dev-only
  crates; all our crates keep `#![forbid(unsafe_code)]`.
- `proptest` needs Rust 1.88, above our 1.85 MSRV. It is dev-only and the MSRV
  job runs `cargo check` (no dev-dependencies), so the MSRV holds for users;
  running the test suite requires a newer toolchain.
- `ignore`/`globset` stay pinned until the MSRV is raised by ADR.

## Alternatives considered

As in ADR 0002 (`walkdir`, manual globbing, a `std` thread pool,
`yaml-rust2`/`serde_yaml`, `anyhow` in libraries), plus: dropping `anyhow` from
the binary for a hand-written error type (more code for no user benefit;
rejected) and replacing `proptest` with an MSRV-compatible older release
(older APIs and no security fixes for a dev-only tool; rejected).
