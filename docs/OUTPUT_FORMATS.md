# Output formats

Status: **pre-alpha**. The formatters land in Phase 1 (`pep8`, `json`) and Phase 4
(the rest). Output is part of the public API (ENG-30): messages, tags and formats
are snapshot-tested.

## Positions

- Internally, everything is a byte offset (`Span`, half-open `[start, end)`).
- For display, `LineIndex` converts to **1-based line and 1-based column**.
- Columns count **Unicode scalar values**, not bytes and not UTF-16 code units.
- The LSP boundary converts to UTF-16 code units and is tested with emoji and CJK.

## Formats

| Format | Phase | Notes |
|---|---|---|
| `pep8` | 1 | default for non-TTY: `path:line: tag: message` |
| `json` | 1 | machine-readable; field names must match upstream where verified |
| `plain` / `rich` | 4 | human-readable, with code frames |
| `codeclimate` | 4 | CI integration |
| `sarif` | 4 | SARIF 2.1.0, hand-rolled structures |
| `github` | 4 | GitHub Actions annotations |
| `quiet` | 1 | findings only, no summary |

Sorting is always `(path, line, column, tag, message)`. Output is byte-identical
across runs and thread counts; no timestamps, no absolute paths in normal output.

Upstream field names for `json`/`codeclimate` are `VERIFY-UPSTREAM`
([OPEN_QUESTIONS.md](OPEN_QUESTIONS.md), item 2).
