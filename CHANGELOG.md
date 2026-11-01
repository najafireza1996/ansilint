# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Before 1.0, minor
releases may change output formats, but every change is listed here.

## [Unreleased]

### Fixed

- `# noqa` is read from real comment tokens only: text such as
  `"x # noqa: rule"` inside a quoted or block scalar no longer suppresses a
  finding, and `#` lines inside block scalars are no longer treated as comments.
- `skip_list`, ignore-file `skip` entries and `tags` no longer hide
  `parser-error`, `load-failure` or `internal-error`; `warn_list` and the
  ignore file no longer raise an `Info` finding to a warning.
- `--generate-ignore` quotes paths with spaces or `#`, so the generated file
  parses back to the same entries.
- `exclude_paths` now applies when the config is given as a relative path such
  as `-c ../.ansible-lint`; files outside the project root are reported with
  `../` instead of an absolute path.
- `.cache/` is excluded implicitly only when `exclude_paths` is not configured,
  as upstream documents; `target/` is no longer excluded.
- Configuration keys that are valid but not implemented yet (for example
  `sarif_file`, `max_block_depth`) now print a notice instead of being silently
  inert.
- `-t/--tags` CLI flag; a `tags` selector that matches no rule now prints a
  notice instead of silently producing an empty report.
- Content sniffing checks every top-level item in every document and recognises
  `import_playbook`; a playbook made only of imports is no longer reported as
  empty. Unmatched YAML is still never treated as a task file.

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
