# Benchmarks

- Micro-benchmarks use `criterion` and live next to their crate (or here, as
  `benches/<name>.rs` once a hot path exists).
- End-to-end benchmarks use `hyperfine` scripts committed here, so every published
  number is reproducible.

Protocol and budgets are in [../docs/BENCHMARKS.md](../docs/BENCHMARKS.md) and
[../docs/PERFORMANCE.md](../docs/PERFORMANCE.md).
