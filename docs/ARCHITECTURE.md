# Architecture

A 30-minute tour of how `ansilint` is put together. The normative engineering rules
live in [ENGINEERING.md](ENGINEERING.md); this page explains the shape they protect.

## Pipeline

```
paths/args ──► Discovery ──► Loader ──► Model builder ──► Rule engine ──► Post-process ──► Output
 (CLI/LSP)    (walk, kinds)  (read,     (FileKind →        (parallel,      (noqa, ignore   (pep8/json/
                              YAML AST)  Play/Task/…)       pure rules)     file, profile,   sarif/…)
                                                                             severity, sort)
                                                              │
                                                              └─► Fix planner ──► Apply (optional, --fix)
```

1. **Discovery** — find the project root, load configuration, expand input paths,
   apply `exclude_paths`, classify each file into a `FileKind`.
2. **Loader** — read each file once (UTF-8, BOM tolerated, size guard), parse YAML
   into a span-carrying AST. Failures become findings, never panics.
3. **Model builder** — turn the AST plus `FileKind` into plays, blocks, tasks,
   handlers and role metadata. Tolerant: unknown shapes become `ModelIssue`s.
4. **Rule engine** — rules are pure functions over the immutable model, executed
   per file in parallel; a panic is isolated and reported as `internal-error`.
5. **Post-process** — inline `# noqa`, ignore file, skip/warn/enable lists, tags,
   profiles, severity mapping, dedupe and a deterministic sort.
6. **Output** — one or more formatters (`pep8`, `json`, `sarif`, …).

## Crates

Every crate states its responsibility and its non-goals in its crate-level docs
(ENG-07). The dependency direction is enforced mechanically by
`cargo xtask check` against `architecture.toml` (ENG-01).

```
core ← yaml ← model ← (config, index, schema, jinja) ← rules ← (fix, output) ← engine ← (lsp, cli)
```

| Crate | Responsibility | Must not |
|---|---|---|
| `ansilint-core` | `Span`, `LineIndex`, `Finding`, `Tag`, `Severity`, `Fix`, `Edit` | parse YAML, read files, know Ansible |
| `ansilint-yaml` | YAML AST with spans, comments, multi-doc, duplicate keys | interpret Ansible, perform I/O |
| `ansilint-model` | Ansible-aware plays/blocks/tasks/roles | read files, resolve includes, evaluate Jinja |
| `ansilint-config` | typed config values, parsing, merge semantics | discover or read files |
| `ansilint-index` | module/plugin index types, queries, redirects | build or download indexes |
| `ansilint-schema` | embedded schemas, validation, JSON-path → span | fetch schemas at lint time |
| `ansilint-jinja` | static Jinja scanning and formatting | evaluate templates |
| `ansilint-rules` | `Rule` trait, registry, rule implementations | perform I/O, mutate the model |
| `ansilint-fix` | span-based edit planning, conflicts, diffs | write files |
| `ansilint-output` | render findings into a caller-provided sink | decide exit codes, read global state |
| `ansilint-engine` | discovery, loading, orchestration, atomic writes | implement rule or formatter logic |
| `ansilint-lsp` | language server library | parse CLI arguments or own process lifecycle |
| `ansilint-cli` | the `ansilint` binary, flags, exit codes | implement linting logic |
| `ansilint-testing` | dev-only fixtures, fakes, contract-test macros | be a normal dependency |

## Functional core, imperative shell

Ten crates are **pure**: `core`, `yaml`, `model`, `config`, `rules`, `index`,
`schema`, `jinja`, `fix` and `output` (ENG-02). They receive bytes/strings/typed
values, return values, and never touch the filesystem, environment, processes,
network, clocks or randomness. Each one ships a `clippy.toml` denying that I/O,
and `cargo xtask check` additionally greps their sources so the rule fails even
when clippy is not invoked.

I/O lives only in `engine`, `cli`, `lsp` and `xtask`.

## Spans are the source of truth

Every AST node and model node carries a byte range (`Span`). Line/column are
derived lazily through `LineIndex`, so diagnostics, fixes and LSP positions all
come from one representation. Columns are 1-based and count Unicode scalar values;
LSP converts to UTF-16 offsets at its boundary (see [OUTPUT_FORMATS.md](OUTPUT_FORMATS.md)).

## Errors and panics

Libraries return typed errors (`thiserror`); binaries may use `anyhow`.
`unwrap`/`expect`/`panic!` are denied outside tests. A malformed file must produce
a `load-failure` or `parser-error` finding; a rule failure must produce
`internal-error`. Neither may abort the run.

## Configuration

Configuration is parsed once at the boundary into typed values and passed down
explicitly. No deep code reads environment variables or config files (ENG-08).
List options extend, scalar options override — the upstream semantics are tracked
in [OPEN_QUESTIONS.md](OPEN_QUESTIONS.md) until verified.

## MSRV and edition

- **Edition:** 2021.
- **MSRV:** Rust **1.85** (`rust-version` in the workspace manifest; CI has a
  dedicated MSRV job).
- **Toolchain:** `rust-toolchain.toml` asks for stable with `rustfmt` and `clippy`;
  the MSRV is validated separately so developers get current lints while CI keeps
  the floor honest.

The MSRV decision is recorded here per P0-T2. Changing it requires an ADR (ENG-40).

## File-kind detection

`FileKind` detection is an ordered list of `(kind, glob)` rules; the first match
wins and user `kinds:` entries are prepended. The default upstream pattern list
and its precedence are marked `VERIFY-UPSTREAM` and will be recorded here during
Phase 1 (P1-T4). Content sniffing is only a fallback.

## Further reading

- [ENGINEERING.md](ENGINEERING.md) — the non-negotiable rules (canonical).
- [PERFORMANCE.md](PERFORMANCE.md) — budgets and allowed hot-path techniques.
- [adr/](adr/) — why decisions are the way they are.
- [OPEN_QUESTIONS.md](OPEN_QUESTIONS.md) — upstream facts still to verify.
