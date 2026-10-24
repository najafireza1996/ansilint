# 0001. YAML parser: saphyr-parser with a span-normalizing AST

Status: accepted

## Context

Phase 1 needs a YAML parser that gives byte spans for every node, preserves
duplicate keys, supports multiple documents, exposes anchors/aliases, and is
usable with MSRV 1.85. Comments are also needed for `# noqa` and later for
`yaml[comments]`. The parser must not be the deprecated `serde_yaml`, and the AST
must be ours so rules can depend on stable types.

We evaluated:

- `saphyr-parser` 0.2.2 (the parser half of `saphyr`, YAML 1.2, MIT/Apache-2.0,
  MSRV 1.85 with default features).
- `marked-yaml` 0.8 (provenance spans, MIT, but its data model does not preserve
  duplicate keys and it is a smaller ecosystem).
- `yaml-rust2` (no byte spans) and libyaml wrappers (spans, but comments are still
  not exposed and the wrapper adds a C dependency).

A spike with `saphyr-parser` confirmed:

- every event carries a `Span` with byte offsets and line/column markers;
- `DocumentStart(true)`/`DocumentEnd` give explicit `---`/`...` positions;
- anchors arrive as numeric ids (0 means “none”), aliases carry the referenced id;
- mapping events preserve duplicate keys because they are a flat event stream;
- multi-document streams work with `Parser::load(..., multi = true)`;
- quoted scalar spans include trailing comments (`"x" # note`), so the AST
  normalizes the span to the closing quote;
- comments themselves are not emitted as events.

## Decision

Use `saphyr-parser` as the event source and build our own AST in
`ansilint-yaml`:

- events are consumed with `Parser::next_event`, which lets parsing be recursive
  and keeps error spans;
- scalar spans are normalized: trim surrounding whitespace for plain/block
  scalars, and compute the closing quote for single/double-quoted scalars;
- duplicate mapping keys are stored in a `Vec<MappingEntry>`; `Mapping::get`
  returns the first match and `get_all` exposes duplicates;
- comments are discovered by a line-oriented scanner in `ansilint-yaml`, not by
  the parser. It ignores `#` inside quotes and requires a leading space for
  inline comments, which is sufficient for `# noqa` in Phase 1;
- anchors and aliases are stored as ids. Resolution is deliberately deferred:
  rules that need it will get a lazy accessor once merge-key behavior is
  verified upstream.

## Consequences

- The parser and AST are pure, have no C dependency, and keep spans for
  diagnostics, fixes and the LSP.
- We own the normalization bugs: quoted spans, block scalars and comment
  detection need regression tests. The comment scanner can misclassify a `#`
  inside a literal/folded block scalar; this is recorded in
  `docs/OPEN_QUESTIONS.md` and does not affect Phase 1 rules.
- `saphyr-parser` is a new dependency of a pure crate. It is MIT/Apache-2.0,
  maintained, and its MSRV matches ours.
- The AST stores decoded scalar values as `String` in Phase 1. Borrowing slices
  from the source is a later optimization; it requires a lifetime-carrying AST
  and benchmark evidence.

## Alternatives considered

- **marked-yaml:** spans are convenient, but duplicate-key preservation and a
  stable rule-facing AST were not guaranteed; rejected.
- **yaml-rust2:** no byte spans; computing them ourselves would duplicate the
  parser; rejected.
- **libyaml/unsafe-libyaml:** spans exist, comments do not, and the C dependency
  conflicts with the single-static-binary goal; rejected.
- **`serde_yaml`:** deprecated and loses spans/duplicates; rejected for the
  linter AST. It is also not used for configuration, which shares this AST.

## Spike evidence

The spike is not committed (it was a temporary example). Its observed event
streams for mappings, sequences, quoted scalars, block scalars, aliases and
multi-document input are reproduced as unit tests in
`crates/ansilint-yaml/src/parser.rs`.
