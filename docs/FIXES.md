# Fixes

Status: **pre-alpha**. The fix engine lands in Phase 8.

## Philosophy

Fixes are **minimal, span-based text edits**, never whole-file YAML
re-serialization. This preserves comments, quoting, key order and formatting that
the author chose. Upstream reformats the whole file on `--fix`; that difference is
intentional and will be documented here per rule.

## Safety

- A fix is `Safe` or `Unsafe`; unsafe fixes require an explicit opt-in and are not
  part of 1.0.
- Edits are non-overlapping and applied back-to-front.
- Overlaps are conflicts: the planner keeps the first edit by rule priority and
  defers the rest to a later pass (maximum 5 passes).
- After applying, the engine re-lints; a fixed tag must not reappear.
- Writes are atomic (temporary file + rename) and only touch files in the lint set.
- `--fix --diff` prints a unified diff and writes nothing.

## Planned safe fixes (Phase 8)

`fqcn`, `name[casing]`, `no-jinja-when`, `partial-become`, `key-order`,
`command-instead-of-shell`, `deprecated-local-action`, `no-free-form`,
`no-log-password`, `yaml[trailing-spaces]`, `yaml[new-line-at-end-of-file]`,
`yaml[document-start]`, `yaml[truthy]`.

Each fix ships with a golden `before.yml`/`after.yml` fixture and an idempotency
test: `fix(fix(x)) == fix(x)`.
