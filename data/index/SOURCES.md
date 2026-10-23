# Module index sources

No index artifacts are committed yet. Index building lands in Phase 5.

## Legal design decision

The shipped index stores **structural facts only** — names, types, required
flags, choices, relations, deprecations and routing. It stores **no descriptive
prose**. This keeps the MIT/Apache binary away from GPL prose while preserving the
checks that need metadata. See the specification, Section 10, and
OPEN_QUESTIONS #8.

## Provenance requirements

Every index blob must carry, in its header and in this file:

- collection name and version;
- source commit (for git checkouts) or `ansible-core` version;
- license per collection;
- fetch/build date;
- the command used to build it.

The default embedded index contains `ansible.builtin` only until a legal review
clears additional collections. Users can build local indexes from their own
installed collections with `ansilint index build`.
