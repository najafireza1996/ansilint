# Output formats

Status: **pre-alpha**. Phase 1 implements `pep8`, `json` and `quiet`; the rest
land in Phase 4. Output is part of the public API (ENG-30): messages, tags and formats
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
| `json` | 1 | machine-readable; stable shape below |
| `plain` / `rich` | 4 | human-readable, with code frames |
| `codeclimate` | 4 | CI integration |
| `sarif` | 4 | SARIF 2.1.0, hand-rolled structures |
| `github` | 4 | GitHub Actions annotations |
| `quiet` | 1 | findings only, no summary |

Sorting is always `(path, line, column, tag, message)`. Output is byte-identical
across runs and thread counts; no timestamps, no absolute paths in normal output.

## Phase 1 JSON shape

```json
[
  {
    "path": "playbooks/site.yml",
    "line": 3,
    "column": 5,
    "tag": "parser-error",
    "message": "bad yaml",
    "severity": "error"
  }
]
```

The array is serialized on one line, sorted by `(path, line, column, tag,
message)`. Severity is `info`, `warning` or `error`. Field names are ours for now;
matching upstream exactly is `VERIFY-UPSTREAM` ([OPEN_QUESTIONS.md](OPEN_QUESTIONS.md),
item 2) and will be pinned by snapshots before `v0.1.0`.

## PEP8 shape

```
playbooks/site.yml:3: parser-error: bad yaml
```

The column is intentionally omitted, matching the documented default format. The
JSON output carries it.
