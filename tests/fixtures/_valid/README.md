# False-positive corpus (H-A4)

Every YAML file here is valid input and must lint with **zero findings**.
`crates/ansilint-engine/tests/valid_corpus.rs` enforces this on every
`cargo xtask ci` run and fails if the corpus drops below 30 files.

A finding on any file here is a P0 bug: fix the linter, never the fixture —
unless the fixture itself is invalid YAML. Add a file for every false positive
ever reported.
