# Compatibility harness

The harness and its fixtures live in a **separate repository** (`ansilint-compat`),
mounted here as a git submodule. It is not part of this repository's license story
because it runs upstream `ansible-lint` (Python) and may carry upstream-derived
fixtures with different licenses.

The harness:

1. runs `ansible-lint` and `ansilint` on the same pinned corpus with the same
   config/profile;
2. normalizes findings to `(path, line, tag)`;
3. classifies each finding as `match`, `extra` (false positive) or `missing`
   (false negative);
4. writes `compat-report.md` and a JSON scoreboard consumed by `cargo xtask
   gen-parity`.

CI runs the harness nightly (it needs Python and `ansible-core`); pull requests
use the last-known-good scoreboard.
