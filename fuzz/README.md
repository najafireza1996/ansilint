# Fuzzing

Targets are added in Phase 1 (P1-T11): the YAML parser and the loader. The nightly
workflow runs each target for at least five minutes.

A crash is always a bug: the linter must turn malformed input into a finding, never
a panic. Reproduce locally with `cargo +nightly fuzz run <target>`.
