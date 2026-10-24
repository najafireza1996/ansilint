# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Before 1.0, minor
releases may change output formats, but every change is listed here.

## [Unreleased]

### Added

- Phase 1: `Span`/`LineIndex` core types with property tests.
- Phase 1: span-carrying YAML AST (saphyr-parser) with comments, duplicate keys,
  anchors/aliases and multi-document support.
- Phase 1: typed configuration parsing and merge semantics, profiles and ignore
  files.
- Phase 1: discovery, UTF-8/BOM/CRLF loading, FileKind detection and content
  sniffing.
- Phase 1: plays/blocks/tasks/actions model with a checked-in task-keyword table.
- Phase 1: rule trait, registry, panic isolation and the `min`-profile meta rules
  (`parser-error`, `load-failure`, `internal-error`, `warning`).
- Phase 1: inline `# noqa`, ignore files, skip/warn/enable lists, profiles,
  deterministic sorting and strict mode.
- Phase 1: PEP8, JSON and quiet formatters, CLI flags, exit codes, list commands
  and `--generate-ignore`.
- Phase 1: `cargo xtask bench` (500-file synthetic repository, 12–13 ms best of 5)
  and a YAML fuzz target with a nightly workflow; a 10-minute local fuzz run
  completed 13.1M executions without a crash.
- Phase 0 bootstrap: Rust workspace with all product crates as documented stubs.
- Section 12 tooling: `cargo xtask check` fitness functions with
  deliberate-violation tests, `cargo xtask ci`, `cargo xtask setup`.
- `ansilint --version` binary with an integration test.
- CI matrix (Linux/macOS/Windows + MSRV), `cargo deny`, `cargo machete`,
  coverage, Conventional Commit validation.
- Engineering docs: architecture, standards, performance, benchmarks, open
  questions, technical-debt register, ADR template.
