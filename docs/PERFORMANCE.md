# Performance

Performance is a product feature (G1: a 500-file repository in under 100 ms).
This page documents the budgets and the techniques that are always allowed. Any
optimization needs benchmark evidence linked in its PR (ENG-32), and budgets are
gates (ENG-33).

## Budgets

| Scenario | Target | Gate |
|---|---|---|
| Cold lint, 500 files | < 100 ms | record in `docs/BENCHMARKS.md` |
| Lint with index loaded, 500 files | < 150 ms | Phase 5 acceptance |
| Index load when unused | < 5 ms | lazy loading required |
| Schema compile, warm | < 20 ms total | Phase 6 acceptance |
| LSP diagnostics, 2 000-line playbook | < 30 ms p95 | Phase 9 acceptance |
| Single-file cold start | measured, never published without a script | — |

A regression of more than 20 % on the benchmark suite fails CI. Binary size and
clean-build time are tracked and may not regress more than 10 % without an ADR.

## Always-allowed techniques

- Read each file exactly once.
- Keep the source as `Arc<str>` and borrow slices; do not copy strings per node.
- Never allocate per character or per byte.
- Compile regexes and schemas once (`LazyLock`/`OnceLock`), never per file.
- Parallelize at file granularity only (`rayon`) and sort deterministically at the end.
- Resolve YAML anchors/aliases lazily and guard against expansion attacks.
- Prefer borrowed `&str`/slices/`Cow` in hot paths; a `.clone()` on a hot path needs
  a justification comment (ENG-28).
- Use integer byte offsets internally; derive line/column lazily.

## Not allowed without an ADR or benchmark

- A second read of the same file.
- Building indices or schemas on the lint hot path.
- Per-character state machines when a byte scan with `memchr`/`aho-corasick` fits.
- Parallelizing inside a file; it adds nondeterminism for little gain.
- Caching keyed by path without an invalidation story (LSP is the exception and
  has its own incremental design in Phase 9).

## Benchmark protocol

See [BENCHMARKS.md](BENCHMARKS.md). Numbers in issues or PRs are only meaningful
when produced by a script committed to this repository.
