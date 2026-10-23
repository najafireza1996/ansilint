# Language server

Status: **pre-alpha**. `ansilint lsp` lands in Phase 9.

## Goals

- Diagnostics on open, change (debounced ~150 ms) and save, with no `ansible-core`
  and no Python installed.
- Quick fixes generated from `Fix` values, including "add `# noqa`".
- Hover and completion from the embedded module index; prose only when a local
  `ansible-doc` is available.
- Incremental re-lint of the changed document; multi-root workspaces; config reload.

## Constraints

- The engine is exposed as a library API with no filesystem assumptions, so the
  LSP can lint in-memory text.
- LSP positions are UTF-16 code units; the conversion from our scalar-value
  columns is unit-tested with emoji and CJK input. This is a classic bug area.
- The LSP stack is chosen by ADR in P9-T1 (`tower-lsp-server` vs `lsp-server` +
  `lsp-types`).

## Latency target

Diagnostics for a 2 000-line playbook in under 30 ms p95.
