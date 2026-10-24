# Benchmarks

No numbers are published until the script that produces them is committed to this
repository. Hand-typed numbers are noise.

## Protocol

- Record hardware (CPU model, RAM), OS and exact tool versions.
- Warm up twice, then run each measurement 10 times; report mean ± standard
  deviation and the minimum.
- Run cold-start measurements with a fresh process and a cleared page cache where
  the OS permits it; otherwise say so explicitly.
- Compare against the upstream `ansible-lint` version named in the report, and
  optionally other fast linters.
- Scenarios:
  1. one file, cold process;
  2. 500-file synthetic repository;
  3. the largest compatibility corpus repository;
  4. with and without the embedded index;
  5. `--fix --diff` (Phase 8).

## Phase 1 results

Date: 2026-10-23. Source: `cargo xtask bench` (library API, not CLI process
startup). Each run lints a freshly generated directory of 500 small playbooks.

| Machine | OS | Rust | Files | Best of 5 |
|---|---|---|---|---|
| Apple M1, 8 GiB | macOS 26.5 (arm64) | 1.99.1 | 500 | **12–13 ms** |

The 100 ms Phase 1 target has a wide margin on this machine. The number is
library-only; CLI startup and CI hardware measurements are added by the Phase 4
harness. Reproduce with `cargo xtask bench`.

Determinism: output was byte-identical with `RAYON_NUM_THREADS=1` and the default
thread pool on a 40-file fixture (20 warnings, 20 parser errors) in both `pep8`
and `json` formats.

## Current status

The end-to-end `hyperfine` harness lands in Phase 4 (P4-T5). Until then, treat
local numbers as directional and do not quote them in the README.
