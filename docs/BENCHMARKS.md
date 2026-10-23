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

## Current status

Phase 0: no benchmark harness or numbers yet. The harness lands in Phase 4 (P4-T5),
with per-phase numbers recorded here as later phases land. Before then, local
numbers are not comparable and must not be quoted in the README.
