# ansilint

**Status: pre-alpha.** Phase 1 is implemented: a working lint pipeline, the
`min`-profile rules, configuration, `# noqa`, PEP8/JSON output and a CLI. Phase 2
adds the text/format rule batch; the full upstream rule catalog is not covered yet.

A fast, single-binary Ansible linter written in Rust that needs no Python and no
`ansible-core` — and can run the checks that other fast linters skip, because it
ships its own module index and schemas.

> Not affiliated with or endorsed by Red Hat or the Ansible project. "Ansible" is a
> trademark of Red Hat/IBM; this project uses it only to describe what it lints.

## Why another linter?

- The reference linter is Python-based, needs `ansible-core`, and some checks work
  by importing and executing module code.
- Existing fast alternatives cover only the purely static subset of rules.
- `ansilint` is a single static binary: same config files, same rule IDs, same
  `# noqa` and ignore-file semantics, same profiles — plus offline schema
  validation, an embedded module index and a language server.

## Status and roadmap

| Phase | Deliverable | State |
|---|---|---|
| 0 | Workspace, tooling, CI, fitness functions | done |
| 1 | Core engine and CLI skeleton (`min` profile) | implemented, pending release |
| 2 | Text/format rules (`yaml`, `name`, …) | planned |
| 3 | Static semantic rules and profiles | planned |
| 4 | Outputs, integrations, `v0.1.0` | planned |
| 5 | Module index and FQCN family | planned |
| 6 | Schemas | planned |
| 7 | `args` rule | planned |
| 8 | Fix engine | planned |
| 9 | Language server | planned |
| 10 | `jinja` rules, hardening, `v1.0.0` | planned |

The generated [parity table](docs/PARITY.md) will be the front page number once
rules land.

## Build from source

```sh
cargo xtask setup
cargo xtask ci
cargo run -p ansilint-cli -- --version
```

Prebuilt binaries, Homebrew, pre-commit and the GitHub Action arrive in Phase 4.

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — pipeline, crates, purity rules, MSRV
- [Engineering standards](docs/ENGINEERING.md) — `ENG-xx` rules (canonical)
- [CLI](docs/CLI.md), [configuration](docs/CONFIG.md),
  [output formats](docs/OUTPUT_FORMATS.md), [fixes](docs/FIXES.md),
  [language server](docs/LSP.md)
- [Benchmarks](docs/BENCHMARKS.md), [performance](docs/PERFORMANCE.md)
- [Open questions](docs/OPEN_QUESTIONS.md), [technical debt](docs/TECH_DEBT.md)
- [Contributing](CONTRIBUTING.md)

## Telemetry

None. Ever. The linter is offline by default and makes no network requests during
normal operation.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
