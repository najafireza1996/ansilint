# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Before 1.0, minor
releases may change output formats, but every change is listed here.

## [Unreleased]

### Added

- Phase 0 bootstrap: Rust workspace with all product crates as documented stubs.
- Section 12 tooling: `cargo xtask check` fitness functions with
  deliberate-violation tests, `cargo xtask ci`, `cargo xtask setup`.
- `ansilint --version` binary with an integration test.
- CI matrix (Linux/macOS/Windows + MSRV), `cargo deny`, `cargo machete`,
  coverage, Conventional Commit validation.
- Engineering docs: architecture, standards, performance, benchmarks, open
  questions, technical-debt register, ADR template.
