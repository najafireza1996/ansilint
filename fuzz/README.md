# Fuzzing

| Target | Surface |
|---|---|
| `yaml_parse` | YAML parser |
| `model` | model builder over every task-bearing file kind |
| `loader` | byte decoding (BOM, CRLF, invalid UTF-8) and every line/column lookup |
| `config` | configuration parsing |
| `ignore_file` | ignore-file parsing plus the render/parse round trip |

The nightly workflow runs each target for five minutes. Only `seed.*` files of
each corpus are committed.

A crash is always a bug: the linter must turn malformed input into a finding, never
a panic. Reproduce locally with `cargo +nightly fuzz run <target>`.
